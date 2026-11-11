# make-usbroot.ps1 - regenerate the unlock USB stick root from pinned public
# inputs. NVIDIA firmware blobs are NOT distributed with this project; this
# script downloads the pinned public packages, verifies their SHA-256, and
# regenerates every file the EFI application needs for the chosen GPU family.
#
# Usage:
#   .\make-usbroot.ps1                     (localhost: detect this machine's GPUs)
#   .\make-usbroot.ps1 -Family tu10x -Out E:\
#   .\make-usbroot.ps1 -Family all         (every RTX 20/30/40 family)
#   .\make-usbroot.ps1 -UtilsSource run    (pin the nvidia-utils transport:
#                     auto = local DriverStore (badge mode) -> Arch archive
#                     -> first-party .run -> CachyOS cache, first that lands;
#                     arch | run | cachyos | local pins one route)
#   .\make-usbroot.ps1 -NoInstallPrompt    (skip the post-build install offer)
#   .\make-usbroot.ps1 -InstallTo D:\      (copy the layout into an existing
#                     FAT32 partition without asking; the volume must pass the
#                     same eligibility checks as the probe)
#
# After generating the layout the script probes for EXISTING FAT32 partitions
# it can copy into: an internal (non-USB) disk's non-boot data partition is
# offered for a persistent install, a USB stick's partition for the same file
# copy. Nothing on the target is ever overwritten or deleted, no partition is
# ever created/resized/reformatted, and no NVRAM boot entry is written.
#
# -Family (default localhost):
#   ad102 | rtx40 | 40   RTX 40 series (Ada)          - no Windows prerequisites
#   ga102 | rtx30 | 30   RTX 30 series (Ampere)       - kmod-verified upstream,
#                                                     EFI path has no silicon evidence yet
#   tu10x | rtx20 | 20   RTX 20 series (Turing)       - EXPECTS GSP firmware mode.
#                        The script WARNS (never blocks) unless EnableGpuFirmware=1
#                        and nvidia-smi shows a running GSP firmware; the legacy
#                        driver path TDR-loops on the unlocked card. Building on a
#                        DIFFERENT machine than the target? Pass -SkipGspCheck.
#   tu11x | rtx16 | 16   GTX 16 series (TU116/117)    - EXPERIMENTAL, same GSP gate.
#                        RE-derived route (issue #151): die-keyed TU116 booter
#                        archive + the TU102 frame table with the park word
#                        re-anchored (0x7EA4 -> 0x7E9A). NOT field-proven;
#                        read-back is the only verdict. Not part of -Family all:
#                        explicit opt-in only.
#   localhost            enumerate the NVIDIA GPUs in THIS machine and build
#                        exactly the families found there; throws when no RTX
#                        20/30/40 card is present. Detection reads the driver's
#                        bound-device list (Services\nvlddmkm\Enum) first - the
#                        only registry structure PnP keeps free of card-swap
#                        residue - so display-class keys orphaned by previously
#                        removed cards (repair benches) are ignored, not built.
#   all                  build all three proven 20-40 series families on one
#                        stick (tu11x excluded - see above). The Turing GSP gate
#                        runs lenient: still warns when a LOCAL Turing card sits
#                        in legacy mode, but is silent when this host has no
#                        Turing adapter at all.
#
# Requires: PowerShell 5+ and internet access to the pinned URLs.

param(
    [ValidateSet("ad102", "ga102", "tu10x", "tu11x", "all", "localhost",
                 "rtx40", "rtx30", "rtx20", "rtx16", "40", "30", "20", "16")]
    [string]$Family = "localhost",
    # Default to the copies shipped next to this script.
    [string]$AssetsExe = "",
    [string]$BootEfi = "",
    [string]$Out = ".\usbroot",
    [string]$Work = ".\inputs",
    # Explicit path to the 128 KiB cmp50hx GspRmBoot archive. Optional; the
    # script otherwise looks next to itself, in the reference checkout, or
    # downloads the pinned blob.
    [string]$ProvenBlob = "",
    # Build for another machine: skip the host-side Turing GSP check.
    [switch]$SkipGspCheck,
    # Which nvidia-utils transport to try: auto (default) tries the LOCAL
    # DriverStore first (badge mode; most build hosts have the driver
    # installed), then the download group (Arch archive -> first-party .run
    # -> CachyOS cache); arch | run | cachyos pins one download route.
    # Selects the TRANSPORT only - the hash gate is identical for downloads,
    # and local is badge mode (version pin waived;
    # docs/gsp-compatibility.md). The offline path always wins regardless:
    # any ONE exact container file already in the work dir is accepted as
    # (cached).
    [ValidateSet("auto", "arch", "run", "cachyos", "cachyos4", "local")]
    [string]$UtilsSource = "auto",
    # Display-class registry key to inspect (override for tests/offline hives).
    [string]$ClassKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}',
    # PnP's bound-device list for the NVIDIA driver (override for tests/offline
    # hives). Only devnodes the driver actually bound to this boot appear here.
    [string]$NvServiceEnumKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\nvlddmkm\Enum',
    # After generating the layout, probe for EXISTING FAT32 partitions to
    # copy it into: an internal (non-USB) disk's non-boot data partition is
    # offered for a persistent install (boot it from the one-time boot menu,
    # no NVRAM entry), a USB stick's partition for the same file copy. The
    # probe never creates/resizes/reformats partitions, never touches an
    # ESP/system/boot volume, and never overwrites an existing file.
    # -NoInstallPrompt skips the interactive offer (scripted runs).
    [switch]$NoInstallPrompt,
    # Non-interactive install target: the ROOT of a FAT32 volume ("D:\").
    # Must pass the same eligibility checks as the probe (FAT32, room for
    # the layout, not ESP/system/boot). Implies -NoInstallPrompt for the
    # rest of the menu.
    [string]$InstallTo = ""
)

# A bare drive root ("G:\") ends in a backslash, which New-Item -Path rejects
# with "The path is not of a legal form" on Windows PowerShell 5.1 (the DONE
# message suggests exactly that form) - issue #195. Keep $Out as "G:\" for the
# path building below (PathBuf::join turns a bare "G:" into the drive-relative
# "G:file"), and only create the directories from the trimmed form.
$mkdirOut = $Out.TrimEnd('\')
$mkdirWork = $Work.TrimEnd('\')

$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 corrupts large Invoke-WebRequest transfers while the
# progress UI is active ("unexpected EOF or 0 bytes") - issue #59.
$ProgressPreference = 'SilentlyContinue'

# Normalize the family aliases (the task is 20/30/40 series, not chip names).
$familyArg = $Family.ToLowerInvariant()
$normalized = switch ($familyArg) {
    { $_ -in "ad102", "rtx40", "40" } { "ad102" }
    { $_ -in "ga102", "rtx30", "30" } { "ga102" }
    { $_ -in "tu10x", "rtx20", "20" } { "tu10x" }
    { $_ -in "tu11x", "rtx16", "16" } { "tu11x" }
    default { $familyArg }   # localhost | all pass through
}

# Device id -> build family (same ranges as loader/src/main.rs).
function Get-UnlockFamily([string]$DevHex) {
    $dev = [Convert]::ToInt32($DevHex, 16)
    if ($dev -ge 0x1E02 -and $dev -le 0x1F80) { return "tu10x" }
    if (($dev -ge 0x1F81 -and $dev -le 0x1FFF) -or
        ($dev -ge 0x2182 -and $dev -le 0x21D1)) { return "tu11x" }
    if (($dev -ge 0x2200 -and $dev -le 0x22FF) -or
        ($dev -ge 0x2400 -and $dev -le 0x25FF)) { return "ga102" }
    if ($dev -ge 0x2600 -and $dev -le 0x28FF) { return "ad102" }
    return $null
}

# Raw display-class scan. Kept as the fallback path only: class keys are
# never cleaned up when a card is removed, so a bench that has seen many
# cards accumulates ghost adapters here (MatchingDeviceId survives, the
# hardware does not - even the full driver value set survives).
function Get-AdaptersFromClassKey {
    $adapters = @()
    foreach ($k in (Get-ChildItem $ClassKey -ErrorAction SilentlyContinue)) {
        $props = Get-ItemProperty $k.PSPath -ErrorAction SilentlyContinue
        if ($props.MatchingDeviceId -and
            $props.MatchingDeviceId -match '(?i)ven_10de&dev_([0-9a-f]{4})') {
            $adapters += [pscustomobject]@{ Dev = $Matches[1].ToLower(); Desc = "$($props.DriverDesc)" }
        }
    }
    return $adapters
}

# The devnodes the NVIDIA driver is actually bound to in THIS boot. PnP
# maintains Services\<driver>\Enum exclusively for started devnodes: unlike
# the display-class key, entries disappear the boot after the card is pulled,
# which makes this the only registry structure that separates a live adapter
# from card-swap residue without leaving the registry. Returns $null when the
# NVIDIA driver has never been installed on this OS install.
function Get-BoundNvidiaInstancePaths {
    $props = Get-ItemProperty $NvServiceEnumKey -ErrorAction SilentlyContinue
    if (-not $props) { return $null }
    return @($props.PSObject.Properties |
             Where-Object { $_.Name -match '^\d+$' -and $_.Value -is [string] } |
             ForEach-Object { $_.Value })
}

# Resolve one live instance path to its Enum entry and the display-class key
# it owns (linked by the Enum 'Driver' value, e.g. "{4d36e968...}\0005").
function Get-InstanceRegistryProps([string]$InstancePath) {
    $enumProps = Get-ItemProperty (Join-Path 'HKLM:\SYSTEM\CurrentControlSet\Enum' $InstancePath) -ErrorAction SilentlyContinue
    $clsProps = $null
    $slot = ""
    if ($enumProps -and $enumProps.Driver) {
        $clsProps = Get-ItemProperty (Join-Path (Split-Path $ClassKey -Parent) $enumProps.Driver) -ErrorAction SilentlyContinue
        if ($clsProps) { $slot = Split-Path $enumProps.Driver -Leaf }
    }
    [pscustomobject]@{ Enum = $enumProps; Class = $clsProps; Slot = $slot }
}

function Get-InstanceDesc($Props, [string]$InstancePath) {
    if ($Props.Class -and $Props.Class.DriverDesc) { return "$($Props.Class.DriverDesc)" }
    if ($Props.Enum -and $Props.Enum.DeviceDesc) { return (($Props.Enum.DeviceDesc -split ';')[-1]) }
    return $InstancePath
}

# localhost: NVIDIA display adapters in THIS machine. The driver's
# bound-device list is authoritative (ghost-free by construction); the
# display-class scan is the fallback for hosts without a bound driver, and
# nvidia-smi the last resort when the class key has no matching entries.
function Get-LocalNvidiaAdapters {
    $bound = Get-BoundNvidiaInstancePaths
    if ($bound -and $bound.Count -gt 0) {
        $adapters = @()
        foreach ($inst in $bound) {
            if ($inst -notmatch '(?i)ven_10de&dev_([0-9a-f]{4})') { continue }
            $adapters += [pscustomobject]@{
                Dev  = $Matches[1].ToLower()
                Desc = Get-InstanceDesc (Get-InstanceRegistryProps $inst) $inst
            }
        }
        # Diagnostics: class-key entries whose device id is not bound this
        # boot are card-swap residue - call them out so the trimming is not
        # a silent behavior change on repair benches.
        $boundDevs = @($adapters | ForEach-Object { $_.Dev } | Select-Object -Unique)
        $ghosts = @()
        foreach ($k in (Get-ChildItem $ClassKey -ErrorAction SilentlyContinue)) {
            $props = Get-ItemProperty $k.PSPath -ErrorAction SilentlyContinue
            if ($props.MatchingDeviceId -and
                $props.MatchingDeviceId -match '(?i)ven_10de&dev_([0-9a-f]{4})') {
                $gdev = $Matches[1].ToLower()
                if ($boundDevs -notcontains $gdev) { $ghosts += $gdev }
            }
        }
        foreach ($g in ($ghosts | Select-Object -Unique)) {
            Write-Warning ("registry ghost ignored (card not bound this boot): 10de:{0} - orphaned display-class key" -f $g)
        }
        return $adapters
    }
    $adapters = Get-AdaptersFromClassKey
    if (-not $adapters -and (Get-Command nvidia-smi -ErrorAction SilentlyContinue)) {
        $rows = & nvidia-smi --query-gpu=pci.device_id,name --format=csv,noheader 2>$null
        foreach ($row in $rows) {
            # pci.device_id prints device+vendor concatenated ("0x1F0710DE"):
            # keep the FIRST 4 hex digits (the device id).
            if ($row -match '(?i)0x([0-9a-f]{8})') {
                $parts = $row -split ',\s*', 2
                $adapters += [pscustomobject]@{ Dev = $Matches[1].Substring(0, 4); Desc = $parts[1] }
            }
        }
    }
    return $adapters
}

# Resolve the family argument into a build list.
$gateStrict = $true
if ($normalized -eq "all") {
    # One stick with every 20-40 series family; may target a machine other
    # than this one, so the Turing GSP gate runs lenient below.
    $Families = @("tu10x", "ga102", "ad102")
    $gateStrict = $false
} elseif ($normalized -eq "localhost") {
    $detected = @()
    foreach ($a in Get-LocalNvidiaAdapters) {
        $fam = Get-UnlockFamily $a.Dev
        if (-not $fam) {
            Write-Host ("[localhost] 10de:{0} - outside RTX 20/30/40 series, ignoring ({1})" -f $a.Dev, $a.Desc)
            continue
        }
        Write-Host ("[localhost] 10de:{0} -> {1} ({2})" -f $a.Dev, $fam, $a.Desc)
        if ($detected -notcontains $fam) { $detected += $fam }
    }
    if ($detected.Count -eq 0) {
        throw "localhost: no supported NVIDIA adapter (RTX 20/30/40/16 series) found on
this machine. Use an explicit -Family (ad102 | ga102 | tu10x | tu11x) or -Family all."
    }
    # Canonical build order regardless of enumeration order. tu11x rides
    # along when a 16-series card is present, same as any detected family.
    $Families = @("tu10x", "tu11x", "ga102", "ad102") | Where-Object { $detected -contains $_ }
    Write-Host ("[localhost] building for: {0}" -f ($Families -join ", "))
    Write-Host "[localhost] full platform snapshot (board/BIOS/VBIOS/driver + advisories): ssdiag --inventory"
    if ($Families.Count -lt 3) {
        Write-Host ("[localhost] NOTE: this stick will only carry {0} assets. Cards from other" -f ($Families -join "/") +
                    " generations in the TARGET machine (e.g. a mixed 30/40-series rig) will be skipped at boot." +
                    " For a multi-generation machine use: .\make-usbroot.ps1 -Family all")
    }
} else {
    $Families = @($normalized)
}

# Pinned public inputs (see AGENTS.md / assets/src/manifest.rs). The
# SHA-256 pin is the only authority: a mirror serving byte-identical bytes
# is acceptable, a download failure never justifies a different version.
$Sources = @(
    @{
        Name = "NVIDIA-kernel-module-source-610.43.03.tar.xz"
        Urls = @("https://download.nvidia.com/XFree86/NVIDIA-kernel-module-source/NVIDIA-kernel-module-source-610.43.03.tar.xz")
        Sha256 = "7E118923C7A23EDC36114D63273A46E3E04E9AF98695A42203E7AC2DFE9FC1DC"
    }
)

# The nvidia-utils GSP-firmware input is a SOURCE GROUP (issue #212): any
# container that yields the pinned inner ELFs is acceptable - the gate is
# content-addressed (gsp_ga10x 57237362..., gsp_tu10x 73065619...), not
# container-addressed. The original CachyOS v3 archive keeps only a
# ~3-version rolling window per package and dropped 610.43.03 on 2026-11-08;
# it stays last in line for anyone holding a cache of the exact bytes. The
# Arch Linux Archive (official rollback archive, versions never purged) and
# the first-party NVIDIA .run (XFree86 tree is append-only) are
# permanent-class transports; all three verified bit-identical firmware on
# 2026-11-08. Order: alive-and-permanent first.
$UtilsSources = @(
    @{
        Id = "arch"; Kind = "pkg"; Name = "nvidia-utils-610.43.03-1-x86_64.pkg.tar.zst"
        Urls = @("https://archive.archlinux.org/packages/n/nvidia-utils/nvidia-utils-610.43.03-1-x86_64.pkg.tar.zst")
        Sha256 = "C3367FF68632AC7DAF6CA50EF7CBDD1CB5D09F7FE0B26125B312C73E52A34CF4"
    },
    @{
        Id = "run"; Kind = "run"; Name = "NVIDIA-Linux-x86_64-610.43.03.run"
        Urls = @("https://download.nvidia.com/XFree86/Linux-x86_64/610.43.03/NVIDIA-Linux-x86_64-610.43.03.run")
        Sha256 = "45E2D4C134A23C35E50F253A4AA63E7E5E8D17E3D185D4A07C8A58E9612ED392"
    },
    @{
        Id = "cachyos"; Kind = "pkg"; Name = "nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst"
        Urls = @("https://archive.cachyos.org/archive/cachyos-v3/nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst")
        Sha256 = "E17E7C92EEFFA178B0FE1F40EB17A47095F8FC3CE4A77BF115CF5BD5A4DF86E9"
    },
    @{
        # The v4 (x86-64-v4) container of the SAME 610.43.03-1 package -
        # still alive after the v3 entry rolled out of the archive window
        # (issue #212/#205): found by kbaccki on #205, inner GSP ELFs
        # verified bit-identical to the pin on 2026-11-08.
        Id = "cachyos4"; Kind = "pkg"; Name = "nvidia-utils-610.43.03-1-x86_64_v4.pkg.tar.zst"
        Urls = @("https://archive.cachyos.org/archive/cachyos-v4/nvidia-utils-610.43.03-1-x86_64_v4.pkg.tar.zst")
        Sha256 = "0FE26739363632A91DF0D60738460AFD5620ABB3A9175F9FB4D8A3794D3DB780"
    }
)

# The provenance-pinned Turing BOOT_DIRECT bootloader. 0x6000 is not an
# arbitrary truncation: it is GSP_RM_BOOT_SIZE (cmp50hx kernel source), the
# exact length the booter consumes. The loader hard-checks >= 0x6000 and pins
# the topology, but the OUTPUT is always this exact 24 KiB slice.
# The slice (de676bae...) fired the successful TU106 run (2026-10-27,
# attempt 13: "gsp_boot_tu10x.bin loaded 24576/24576 bytes"). The GSP ELF's
# first 0x6000 bytes (c76c43b1...) is NOT the same image and must not ship.
# Transport mirrors: the SHA-256 pin is the only authority - any source
# serving byte-identical bytes is acceptable, and a download failure never
# justifies a different version (constitution 6). raw.githubusercontent.com
# is DNS-poisoned/SNI-reset behind the Chinese mainland firewall, so the
# jsDelivr CDNs and a gh-proxy relay are pinned as verified transports for
# the SAME object (all three re-hashed to fa43239b... on 2026-11-01).
$ProvenBlobName = "gsp_rm_boot_dbg.bin"
$ProvenBlobSrcSha256 = "FA43239BCEE7B97CA62F007CC68487560A39E19F74F3DDE7486DB3F98DF8E471"
$ProvenBlobUrls = @(
    "https://raw.githubusercontent.com/xrip/cmp50hx-unlock/master/efi-unlock/blobs/gsp_rm_boot_dbg.bin",
    "https://fastly.jsdelivr.net/gh/xrip/cmp50hx-unlock@master/efi-unlock/blobs/gsp_rm_boot_dbg.bin",
    "https://cdn.jsdelivr.net/gh/xrip/cmp50hx-unlock@master/efi-unlock/blobs/gsp_rm_boot_dbg.bin",
    "https://gh-proxy.com/https://raw.githubusercontent.com/xrip/cmp50hx-unlock/master/efi-unlock/blobs/gsp_rm_boot_dbg.bin"
)
$ProvenSliceLen = 0x6000
$ProvenSliceSha256 = "de676bae28a480011d3d012db14bef539324e62a841a9627863c689bea168af3"

# ---------- Turing GSP-mode gate (warn without GSP firmware mode) ----------
# Turing's Windows driver only accepts a pre-OS FEAT unlock in GSP-RM firmware
# mode; the legacy path TDR-loops (Code 43 / watchdog storm) on the unlocked
# card. Building a tu10x stick on a machine that has not enabled GSP mode is
# how users brick their session - warn here, point at the fix, and let the
# operator decide: stick regeneration must never block on the BUILD host's
# state; the TARGET machine's GSP prerequisites are what actually matter.

function Test-TuringDevId([string]$MatchingDeviceId) {
    if ($MatchingDeviceId -notmatch '(?i)ven_10de&dev_([0-9a-f]{4})') { return $false }
    $dev = [Convert]::ToInt32($Matches[1], 16)
    return (($dev -ge 0x1E02 -and $dev -le 0x1FFF) -or
            ($dev -ge 0x2182 -and $dev -le 0x21D1))
}

function Get-TuringAdapterState {
    # Instance-exact: only devnodes the driver is bound to this boot can be
    # GSP-mode candidates, and each maps to exactly one class key via the
    # Enum 'Driver' value. Class keys orphaned by removed cards are skipped
    # entirely; the raw class-key scan only runs when nothing is bound (no
    # usable NVIDIA driver session - there, residue is unavoidable).
    $bound = Get-BoundNvidiaInstancePaths
    $state = @()
    if ($bound -and $bound.Count -gt 0) {
        foreach ($inst in $bound) {
            if (-not (Test-TuringDevId $inst)) { continue }
            $p = Get-InstanceRegistryProps $inst
            $gsp = $null
            if ($p.Class) { $gsp = $p.Class.EnableGpuFirmware }
            $state += [pscustomobject]@{
                Slot = $(if ($p.Slot) { $p.Slot } else { $inst })
                Desc = Get-InstanceDesc $p $inst
                Gsp  = $gsp
            }
        }
        return $state
    }
    foreach ($k in (Get-ChildItem $ClassKey -ErrorAction SilentlyContinue)) {
        $props = Get-ItemProperty $k.PSPath -ErrorAction SilentlyContinue
        if (-not $props.MatchingDeviceId -or -not (Test-TuringDevId $props.MatchingDeviceId)) { continue }
        $state += [pscustomobject]@{
            Slot = $k.PSChildName
            Desc = $props.DriverDesc
            Gsp  = $props.EnableGpuFirmware
        }
    }
    return $state
}

# Runtime corroboration: a registry value alone only proves the switch is SET,
# not that the current driver session actually loaded GSP firmware (a stale
# non-restarted session keeps the legacy path). nvidia-smi reports N/A when
# GSP is inactive.
function Get-GspRuntimeState {
    $smi = Get-Command nvidia-smi -ErrorAction SilentlyContinue
    if (-not $smi) { return "absent" }
    $out = & $smi.Source -q 2>$null | Out-String
    if ($LASTEXITCODE -ne 0) { return "failed" }
    $hits = [regex]::Matches($out, '(?m)^\s*GSP Firmware Version\s*:\s*(.+?)\s*$')
    if ($hits.Count -eq 0) { return "unknown" }
    foreach ($h in $hits) {
        if ($h.Groups[1].Value -notmatch '(?i)^N/?A$') { return "active" }
    }
    return "inactive"
}

$gateStatus = "n/a"
if (($Families -contains "tu10x" -or $Families -contains "tu11x") -and -not $SkipGspCheck) {
    Write-Host "[gate] Turing family selected - checking Windows GSP firmware mode"
    $turing = @(Get-TuringAdapterState)
    if ($turing.Count -eq 0) {
        if ($gateStrict) {
            Write-Warning "No NVIDIA Turing display adapter PRESENT on THIS machine (orphaned display-class keys from removed cards are ignored)."
            Write-Host "If the unlock target is a different machine, pass -SkipGspCheck to silence this check."
        } else {
            # -Family all may target a machine other than this one.
            Write-Host "[gate] no Turing adapter on this host - GSP check not applicable (-Family all)"
        }
        $gateStatus = "no-turing"
        Write-Host "[gate] skipped (no Turing adapter on this host)"
    } else {
        $notGsp = @($turing | Where-Object { $_.Gsp -ne 1 })
        foreach ($t in $turing) {
            $state = if ($t.Gsp -eq 1) {
                "GSP mode (EnableGpuFirmware=1)"
            } else {
                "LEGACY mode (EnableGpuFirmware unset/0) - WILL TDR on the unlocked card"
            }
            Write-Host ("  Turing adapter {0}: {1} -> {2}" -f $t.Slot, $t.Desc, $state)
        }
        if ($notGsp.Count -gt 0) {
            Write-Warning "Turing adapter(s) above are NOT in GSP firmware mode. The build continues, but using the stick on a legacy-mode Turing target WILL TDR (Code 43 / watchdog storm).
Fix BEFORE using the stick: run .\enable-turing-unlock.ps1 from an
elevated PowerShell on the target, then RESTART (a hibernate resume
does NOT re-initialize the driver and will not apply the mode)."
            $gateStatus = "legacy-mode"
        } else {
            $runtime = Get-GspRuntimeState
            switch ($runtime) {
                "active" {
                    Write-Host "[ok] GSP firmware running (nvidia-smi reports a firmware version)"
                }
                "absent" {
                    Write-Warning "nvidia-smi not found - registry check only"
                }
                "unknown" {
                    Write-Warning "nvidia-smi has no 'GSP Firmware Version' field - registry check only"
                }
                "failed" {
                    Write-Warning "nvidia-smi -q failed - registry check only"
                }
                "inactive" {
                    Write-Warning "Registry says EnableGpuFirmware=1 but nvidia-smi reports GSP firmware N/A.
The setting is not active in this driver session. The build continues, but
RESTART (not hibernate resume) before using the stick on this machine.
If it persists, the driver may be too old for GSP."
                    $gateStatus = "runtime-inactive"
                }
            }
            if ($runtime -eq "active") {
                Write-Host "[ok] all Turing adapters in GSP firmware mode"
                $gateStatus = "passed"
            }
        }
    }
} elseif ($Families -contains "tu10x" -or $Families -contains "tu11x") {
    $gateStatus = "skipped"
    Write-Host "[gate] Turing GSP check skipped (-SkipGspCheck)"
}

if (-not $AssetsExe) { $AssetsExe = Join-Path $PSScriptRoot "assets-x86_64-windows.exe" }
if (-not $BootEfi)   { $BootEfi   = Join-Path $PSScriptRoot "BOOTX64.EFI" }
if (-not (Test-Path $AssetsExe)) { throw "assets tool not found: $AssetsExe" }
if (-not (Test-Path $BootEfi))   { throw "BOOTX64.EFI not found: $BootEfi" }
New-Item -ItemType Directory -Force -Path $mkdirWork, $mkdirOut | Out-Null

# ---------- pinned-download transport (issues #52/#59/#180/#205) ----------
# PS 5.1 Invoke-WebRequest is a single connection with no resume: one reset
# on a 314 MB file restarts from zero, and with one URL x two passes the run
# dies. curl.exe (ships with Windows 10 1803+, 64-bit PowerShell) is the
# preferred transport: real exit codes and HTTP Range resume, so a dropped
# transfer continues where it died. Invoke-WebRequest stays as the fallback
# (32-bit PowerShell has no visible System32 curl.exe). The SHA-256 pin
# remains the only authority: every transport writes to .part and the hash
# gate decides - a killed or corrupted transfer never lands at $dest.
$script:CurlExe = (Get-Command curl.exe -ErrorAction SilentlyContinue).Source

function Invoke-PinnedFetch {
    # One transfer attempt to $OutFile. Never throws; returns Ok/Via/Reason
    # so the caller can log the verdict. curl's own error line (if any)
    # prints above ours and exit codes are stable, so no stderr capture.
    param([string]$Url, [string]$OutFile)
    if ($script:CurlExe) {
        & $script:CurlExe -fLsS --retry 2 -C - --connect-timeout 30 -o $OutFile $Url
        $code = $LASTEXITCODE
        if ($code -eq 33) {
            # Server refused the resume range on a leftover .part: restart
            # the transfer clean instead of failing the attempt.
            & $script:CurlExe -fLsS --retry 2 --connect-timeout 30 -o $OutFile $Url
            $code = $LASTEXITCODE
        }
        if ($code -eq 0) { return @{ Ok = $true; Via = "curl.exe"; Reason = "" } }
        return @{ Ok = $false; Via = "curl.exe"; Reason = "exit $code" }
    }
    try {
        Invoke-WebRequest -Uri $Url -OutFile $OutFile
        return @{ Ok = $true; Via = "Invoke-WebRequest"; Reason = "" }
    } catch {
        $msg = ($_.Exception.Message -split "`r?`n")[0]
        return @{ Ok = $false; Via = "Invoke-WebRequest"; Reason = $msg }
    }
}

function Get-PinnedFile {
    # Fetch one pinned input: cached-accept, then every URL over up to three
    # passes. Returns $true on success, $false on total failure (the caller
    # decides the report). Two DISTINCT failure verdicts are recorded in
    # $script:LastPinnedKind ("wrong-bytes" = a source answered but the bytes
    # did not hash to the pin; "no-transport" = nothing was ever delivered -
    # not a hash accusation, and a hash mismatch is not flaky network). A
    # failed transport keeps .part so the next pass or the next run resumes.
    param([string]$Name, [string[]]$Urls, [string]$Sha256, [string]$Dest)
    $part = "$Dest.part"
    if ((Test-Path $Dest) -and (Get-FileHash -Algorithm SHA256 $Dest).Hash -eq $Sha256) {
        Write-Host "[ok] $Name SHA-256 verified (cached)"
        return $true
    }
    Remove-Item $Dest -Force -ErrorAction SilentlyContinue
    $servedWrongBytes = $false
    $lastReason = "no transport attempted"
    foreach ($pass in 1..3) {
        foreach ($u in $Urls) {
            $had = [long](Get-Item $part -ErrorAction SilentlyContinue).Length
            if ($had -gt 0) {
                Write-Host ("[fetch] {0} <- {1} (pass {2}, resuming {3} bytes)" -f $Name, $u, $pass, $had)
            } else {
                Write-Host ("[fetch] {0} <- {1} (pass {2})" -f $Name, $u, $pass)
            }
            $r = Invoke-PinnedFetch $u $part
            if ($r.Ok) {
                $hash = (Get-FileHash -Algorithm SHA256 $part).Hash
                if ($hash -eq $Sha256) {
                    Move-Item -Force $part $Dest
                    Write-Host "[ok] $Name SHA-256 verified"
                    return $true
                }
                $servedWrongBytes = $true
                $lastReason = "wrong bytes from $u (sha256 $hash)"
                Write-Host "[warn] $Name`: $u served the wrong bytes (sha256 $hash, want $Sha256) - refusing this copy"
                # Wrong bytes must never be resumed into a good file.
                Remove-Item $part -Force
            } else {
                # Keep .part: the next pass (or the next run) resumes it.
                $kept = [long](Get-Item $part -ErrorAction SilentlyContinue).Length
                Write-Host ("[warn] {0}: transport failed via {1} ({2}); {3} bytes kept for resume" -f $Name, $r.Via, $r.Reason, $kept)
                $lastReason = "{0} from {1}" -f $r.Reason, $u
            }
        }
    }
    Remove-Item $part -Force -ErrorAction SilentlyContinue
    $script:LastPinnedKind = $(if ($servedWrongBytes) { "wrong-bytes" } else { "no-transport" })
    $script:LastPinnedReason = $lastReason
    return $false
}

function Get-LocalDriverStoreGsp {
    # Badge-mode source (docs/gsp-compatibility.md): the LOCALLY INSTALLED
    # NVIDIA driver's own GSP firmware. The DriverStore ships the split
    # gsp_ga10x.bin / gsp_tu10x.bin ELFs (format era >= 525; pre-52x drivers
    # have a single multi-generation gsp.bin and are refused with a named
    # reason). The extracted fwimage+fwsignature pair is paired by
    # construction, and the local driver is the handoff target's own driver
    # - the most realistic GSP to map. Version pin WAIVED by measured
    # compat: 525-621 all unlock (TU106 bench, 16.7-37.1 MB).
    $stores = Get-ChildItem 'C:\Windows\System32\DriverStore\FileRepository' `
        -Directory -Filter 'nv*inf_amd64_*' -ErrorAction SilentlyContinue |
        Where-Object { Test-Path (Join-Path $_.FullName 'gsp_tu10x.bin') }
    if (-not $stores) {
        Write-Host "[local] no DriverStore directory with gsp_tu10x.bin (no NVIDIA driver installed, or a pre-52x driver whose store carries only the single gsp.bin)"
        return $null
    }
    $best = $null
    foreach ($st in $stores) {
        $inf = Get-ChildItem $st.FullName -Filter '*.inf' -ErrorAction SilentlyContinue |
            Select-Object -First 1
        $ver = $null
        if ($inf) {
            $line = (Select-String -Path $inf.FullName -Pattern '^DriverVer\s*=\s*(.+)$' -ErrorAction SilentlyContinue |
                Select-Object -First 1)
            if ($line) { $ver = $line.Matches[0].Groups[1].Value.Trim() }
        }
        Write-Host ("[local] candidate {0} (DriverVer {1})" -f $st.Name, $(if ($ver) { $ver } else { "unknown" }))
        if (-not $best) { $best = @{ Dir = $st.FullName; Ver = $ver }; continue }
        # Newest wins; version-less candidates lose to any versioned one.
        if ($ver -and ($best.Ver -eq $null -or $ver -gt $best.Ver)) {
            $best = @{ Dir = $st.FullName; Ver = $ver }
        }
    }
    if (-not (Test-Path (Join-Path $best.Dir 'gsp_ga10x.bin'))) {
        Write-Host "[local] chosen store lacks gsp_ga10x.bin - refusing"
        return $null
    }
    Write-Host ("[local] using DriverStore {0} (DriverVer {1})" -f $best.Dir, $(if ($best.Ver) { $best.Ver } else { "unknown" }))
    return @{ Dir = $best.Dir; Ver = $best.Ver }
}

function Get-UtilsPackage {
    # Resolve the nvidia-utils GSP-firmware input from the SOURCE GROUP
    # above: automatic fallback, first source that lands wins (cached file,
    # or a verified download). -UtilsSource pins a single route; the offline
    # cache-accept check inside Get-PinnedFile runs first for every
    # candidate, so a user-supplied exact file still wins regardless.
    # In auto mode the LOCAL DriverStore leads (most build hosts have the
    # NVIDIA driver installed; its GSP is the handoff target's own firmware
    # and badge mode waives the version pin - measured 525-621 compatible,
    # docs/gsp-compatibility.md); downloads follow as fallback. Returns
    # @{ Path; Kind }; on total failure throws the all-sources report so the
    # user sees every transport, its pinned hash and how to go fully
    # offline (any ONE of the files).
    $candidates = if ($UtilsSource -eq "auto") { @(@{ Id = "local"; Kind = "local" }) + @($UtilsSources) }
                  elseif ($UtilsSource -eq "local") { @(@{ Id = "local"; Kind = "local" }) }
                  else { $UtilsSources | Where-Object { $_.Id -eq $UtilsSource } }
    foreach ($s in $candidates) {
        if ($s.Kind -eq "local") {
            $store = Get-LocalDriverStoreGsp
            if ($store) {
                return @{ Path = $store.Dir; Kind = "local"; Ver = $store.Ver }
            }
            Write-Host "[fallback] local DriverStore unusable - trying the next pinned source"
            continue
        }
        $dest = Join-Path $Work $s.Name
        if (Get-PinnedFile -Name $s.Name -Urls $s.Urls -Sha256 $s.Sha256 -Dest $dest) {
            return @{ Path = $dest; Kind = $s.Kind }
        }
        Write-Host ("[fallback] {0} unusable ({1}: {2}) - trying the next pinned source" -f $s.Name, $script:LastPinnedKind, $script:LastPinnedReason)
    }
    $listing = ($candidates | ForEach-Object {
        if ($_.Kind -eq "local") {
            "  - local DriverStore (C:\Windows\System32\DriverStore\FileRepository\nv*inf_amd64_*\gsp_*.bin)"
        } else {
            "  - {0}`n      {1}`n      sha256 {2}" -f $_.Name, ($_.Urls -join " ; "), $_.Sha256
        }
    }) -join "`n"
    $pinnedNote = $(if ($UtilsSource -ne "auto") {
        "Only '$UtilsSource' was tried (pinned via -UtilsSource). Re-run without the flag to try every transport."
    })
    throw @"
ALL pinned nvidia-utils sources failed - the GSP firmware container could not be obtained.
Every attempt above logs its own verdict (transport failure vs wrong bytes). Sources tried:
$listing
$pinnedNote
Options:
  1. Fix the network condition and re-run - a partial download resumes where it died.
  2. Go offline: download ANY ONE of the files above by any means (browser, another
     machine, proxy) and drop it into $Work unchanged - the SHA-256 gate accepts it
     offline. All three carry bit-identical GSP firmware.
Never substitute a different driver version: the pinned SHA-256 values above are the
only accepted bytes (a newer nvidia-utils, e.g. 615.x, is NOT interchangeable).
"@
}

foreach ($s in $Sources) {
    if (-not (Get-PinnedFile -Name $s.Name -Urls $s.Urls -Sha256 $s.Sha256 -Dest (Join-Path $Work $s.Name))) {
        if ($script:LastPinnedKind -eq "wrong-bytes") {
            throw "download failed for $($s.Name): a source answered but did not serve the pinned bytes (sha256 $($s.Sha256)). The pin/URL pairing is broken - please report this. Never substitute another version of the file."
        }
        throw "download failed for $($s.Name): no transport delivered the file (last: $($script:LastPinnedReason)).
This is a network/client failure, not a bad pin - re-run to retry (a partial download resumes), or obtain the exact file by any means and drop it into $Work (sha256 $($s.Sha256)) to build offline."
    }
}
$UtilsInput = Get-UtilsPackage

$SrcTar = Join-Path $Work $Sources[0].Name

# Remove assets from an earlier run so a root cannot end up as a half-old,
# half-new mix (e.g. a 40-series v67 payload next to a 20-series booter).
Get-ChildItem $Out -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '^(booter_load_|gsp_boot|gsp_|fwimage_|fwsignature_|sec2_bl_|scrubber_|v67-|gsp-gate\.txt$|BOOTX64\.EFI$)' } |
    ForEach-Object { Remove-Item $_.FullName -Force }

# ---------- shared extraction (one pass for the whole build) ----------
# fw / fw-run always extract BOTH GSP ELFs (gsp_ga10x.bin + gsp_tu10x.bin)
# in a single pass, so it runs once no matter how many families build. The
# subcommand follows whichever source-group container landed: a .pkg.tar.zst
# goes through fw, the first-party .run through fw-run (same inner hash
# gate; requires an assets build that advertises fw-run in its usage).
Write-Host "[extract] GSP firmware images (gsp_ga10x + gsp_tu10x)"
if ($UtilsInput.Kind -eq "local") {
    # Badge mode (docs/gsp-compatibility.md): the ELFs come from the local
    # DriverStore (the installed driver's own GSP; fwimage+fwsignature
    # paired by construction). Version pin WAIVED - measured 525-621 all
    # unlock. Format-gated downstream: the per-family gsp-sections calls
    # fail loudly on a pre-52x single-ELF store.
    Write-Host "[extract] BADGE MODE: GSP from the local DriverStore$(
        if ($UtilsInput.Ver) { " (DriverVer $($UtilsInput.Ver))" }) - version pin waived (525-621 measured compatible)"
    Copy-Item (Join-Path $UtilsInput.Path 'gsp_ga10x.bin') (Join-Path $Out 'gsp_ga10x.bin') -Force
    Copy-Item (Join-Path $UtilsInput.Path 'gsp_tu10x.bin') (Join-Path $Out 'gsp_tu10x.bin') -Force
    Write-Host "[ok] gsp_ga10x.bin + gsp_tu10x.bin staged from the local driver (format gate = per-family gsp-sections)"
} elseif ($UtilsInput.Kind -eq "run") {
    # Capability probe: assets prints its usage on stderr with a nonzero
    # exit, which PS 5.1 + ErrorActionPreference=Stop turns into a
    # NativeCommandError on `2>&1` - capture under Continue instead.
    $usageLine = ""
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try { $usageLine = (& $AssetsExe 2>&1 | Out-String) } catch { $usageLine = "" }
    $ErrorActionPreference = $prevEap
    if ($usageLine -notmatch "fw-run") {
        throw "the nvidia-utils input is the first-party .run, but this assets build has no fw-run support. Update assets-x86_64-windows.exe (same place you got this script), or supply either .pkg.tar.zst container from the source group instead."
    }
    & $AssetsExe fw-run $UtilsInput.Path $Out
    if ($LASTEXITCODE) { throw "fw-run failed" }
} else {
    & $AssetsExe fw $UtilsInput.Path $Out
    if ($LASTEXITCODE) { throw "fw failed" }
}

if (@($Families | Where-Object { $_ -ne "tu10x" -and $_ -ne "tu11x" }).Count -gt 0) {
    Write-Host "[extract] SEC2 scrubber (shared: ad102/ga102)"
    & $AssetsExe extract-scrub $SrcTar $Out
    if ($LASTEXITCODE) { throw "extract-scrub failed" }
}

# ---------- per-family extraction ----------
$expected = @()
$unlockFiles = @()
$ga10xSectionsDone = $false
$tuSectionsDone = $false

# gsp-sections exports fwimage_<fam>.bin + fwsignature_<fam>.bin per wanted
# signature family. The .fwimage section is family-neutral (the same tens of
# MB get rewritten for every exported name), so only the families this build
# serves are requested - the ga10x ELF carries ga100/ga10x/ad10x signatures,
# the tu10x ELF tu10x/tu11x/ga100. The loader looks families up by device
# family name, hence the ad10x->ad102 / ga10x->ga102 copies below.
$ga10xSigFamilies = @()
if ($Families -contains "ga102") { $ga10xSigFamilies += "ga10x" }
if ($Families -contains "ad102") { $ga10xSigFamilies += "ad10x" }
$tuSigFamilies = @()
if ($Families -contains "tu10x") { $tuSigFamilies += "tu10x" }
if ($Families -contains "tu11x") { $tuSigFamilies += "tu11x" }

# The proven GspRmBoot archive is Turing-shared (the driver generates only a
# TU102 one), so tu10x and tu11x resolve and slice the same pinned bytes.
# Order: explicit path, next to this script, reference checkout, work dir,
# then the pinned URL.
function Resolve-ProvenBlob {
    if ($script:ProvenBlob -and -not (Test-Path $script:ProvenBlob)) {
        throw "-ProvenBlob not found: $($script:ProvenBlob)"
    }
    $blob = @(
        $script:ProvenBlob,
        (Join-Path $PSScriptRoot $script:ProvenBlobName),
        (Join-Path (Split-Path $PSScriptRoot -Parent) "reference\cmp50hx-unlock\efi-unlock\blobs\$($script:ProvenBlobName)"),
        (Join-Path $script:Work $script:ProvenBlobName)
    ) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
    if (-not $blob) {
        $blob = Join-Path $script:Work $script:ProvenBlobName
        # Same transport + hash gate as the pinned sources above; throws its
        # own verdict (transport failure vs wrong bytes) on total failure.
        Get-PinnedFile -Name $script:ProvenBlobName -Urls $script:ProvenBlobUrls `
            -Sha256 $script:ProvenBlobSrcSha256 -Dest $blob
    }
    if (-not (Test-Path $blob)) {
        throw "could not fetch $($script:ProvenBlobName) from any pinned transport; drop the exact file (sha256 $($script:ProvenBlobSrcSha256)) into $($script:Work) and re-run"
    }
    $blobHash = (Get-FileHash -Algorithm SHA256 $blob).Hash
    if ($blobHash -ne $script:ProvenBlobSrcSha256) {
        throw "proven GspRmBoot blob mismatch: got $blobHash want $($script:ProvenBlobSrcSha256) ($blob) - refusing (the 128 KiB cmp50hx archive is the only accepted source; the gsp_tu10x.bin ELF is NOT it)"
    }
    Write-Host "[ok] proven GspRmBoot archive SHA-256 verified ($((Get-Item $blob).Length) bytes)"
    return $blob
}

# Slice + byte-pin check for one Turing family's gsp_boot_<fam>.bin.
function Copy-ProvenSlice {
    param([string]$Family)
    $blob = Resolve-ProvenBlob
    Write-Host "[slice] first 0x6000 bytes -> gsp_boot_$Family.bin"
    & $script:AssetsExe slice-proven-rm-boot $blob $script:Out $Family
    if ($LASTEXITCODE) { throw "slice-proven-rm-boot failed" }
    # The proven slice is byte-pinned: exactly 0x6000 bytes, de676bae...
    # A 4 KiB v4 BINDATA image or the GSP ELF's first 0x6000 (c76c43b1...)
    # must never ship.
    $bootOut = Join-Path $script:Out "gsp_boot_$Family.bin"
    $len = (Get-Item $bootOut).Length
    if ($len -ne $script:ProvenSliceLen) {
        throw "gsp_boot_$Family.bin is $len bytes, want $($script:ProvenSliceLen) (0x6000): trimmed proven GspRmBoot slice only"
    }
    $bootHash = (Get-FileHash -Algorithm SHA256 $bootOut).Hash.ToLowerInvariant()
    if ($bootHash -ne $script:ProvenSliceSha256) {
        throw "gsp_boot_$Family.bin hash $bootHash != pinned $($script:ProvenSliceSha256)"
    }
    Write-Host "[ok] gsp_boot_$Family.bin = 0x6000 bytes, de676bae... (proven slice, byte-identical to the fired TU106 run)"
}

# Turing gsp-gate.txt record (one per stick; the family line lists who built it).
function Write-GspGateRecord {
    param([string[]]$TuringFamilies)
    @(
        "nv-uefi-unlocker Turing GSP gate"
        "time: $(Get-Date -Format o)"
        "host: $env:COMPUTERNAME"
        "family: $($TuringFamilies -join ',')"
        "gate: $gateStatus"
        "adapters:"
        Get-TuringAdapterState | ForEach-Object { "  $($_.Slot) $($_.Desc) EnableGpuFirmware=$($_.Gsp)" }
    ) | Set-Content -Path (Join-Path $Out "gsp-gate.txt") -Encoding Ascii
}
foreach ($fam in $Families) {
    switch ($fam) {
        "ad102" {
            Write-Host "[extract] AD102 booter / GSP boot"
            & $AssetsExe extract-ad102 $SrcTar $Out
            if ($LASTEXITCODE) { throw "extract-ad102 failed" }
            if (-not $ga10xSectionsDone) {
                Write-Host "[extract] fwimage / fwsignature sections (ga10x ELF: $($ga10xSigFamilies -join ', '))"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_ga10x.bin") $Out $ga10xSigFamilies
                if ($LASTEXITCODE) { throw "gsp-sections failed" }
                $ga10xSectionsDone = $true
            }
            Write-Host "[generate] V67 AD102 four-write payload"
            & $AssetsExe generate-ad102 (Join-Path $Out "v67-ad102-four-plm.bin")
            if ($LASTEXITCODE) { throw "generate-ad102 failed" }
            # gsp-sections emits the ELF section family name (ad10x); the loader
            # looks the device family up as ad102.
            Copy-Item (Join-Path $Out "fwimage_ad10x.bin")     (Join-Path $Out "fwimage_ad102.bin") -Force
            Copy-Item (Join-Path $Out "fwsignature_ad10x.bin") (Join-Path $Out "fwsignature_ad102.bin") -Force
            $expected += @(
                "booter_load_ad102.bin", "gsp_boot_desc_ad102.bin", "gsp_boot_ad102.bin",
                "fwimage_ad102.bin", "fwsignature_ad102.bin", "scrubber_ad10x.bin",
                "v67-ad102-four-plm.bin"
            )
            $unlockFiles += "v67-ad102-four-plm.bin"
        }
        "ga102" {
            Write-Host "[extract] GA102 booter / GSP boot"
            & $AssetsExe extract $SrcTar $Out
            if ($LASTEXITCODE) { throw "extract failed" }
            if (-not $ga10xSectionsDone) {
                Write-Host "[extract] fwimage / fwsignature sections (ga10x ELF: $($ga10xSigFamilies -join ', '))"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_ga10x.bin") $Out $ga10xSigFamilies
                if ($LASTEXITCODE) { throw "gsp-sections failed" }
                $ga10xSectionsDone = $true
            }
            Write-Host "[generate] V67 GA102 payload"
            & $AssetsExe generate (Join-Path $Out "v67-ga102-four-plm.bin")
            if ($LASTEXITCODE) { throw "generate failed" }
            Copy-Item (Join-Path $Out "fwimage_ga10x.bin")     (Join-Path $Out "fwimage_ga102.bin") -Force
            Copy-Item (Join-Path $Out "fwsignature_ga10x.bin") (Join-Path $Out "fwsignature_ga102.bin") -Force
            $expected += @(
                "booter_load_ga102.bin", "gsp_boot_desc_ga102.bin", "gsp_boot_ga102.bin",
                "fwimage_ga102.bin", "fwsignature_ga102.bin", "scrubber_ad10x.bin",
                "v67-ga102-four-plm.bin"
            )
            $unlockFiles += "v67-ga102-four-plm.bin"
        }
        "tu10x" {
            Write-Host "[extract] TU102 booter (BOOT_DIRECT) / generic BL"
            & $AssetsExe extract-tu10x $SrcTar $Out
            if ($LASTEXITCODE) { throw "extract-tu10x failed" }
            if (-not $tuSectionsDone) {
                Write-Host "[extract] fwimage / fwsignature sections (tu10x ELF: $($tuSigFamilies -join ', '))"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_tu10x.bin") $Out $tuSigFamilies
                if ($LASTEXITCODE) { throw "gsp-sections failed" }
                $tuSectionsDone = $true
            }
            Write-Host "[generate] V67 TU10x payload"
            & $AssetsExe generate-tu10x (Join-Path $Out "v67-tu10x-four-plm.bin")
            if ($LASTEXITCODE) { throw "generate-tu10x failed" }
            Copy-ProvenSlice "tu10x"
            $expected += @(
                "booter_load_tu10x.bin", "gsp_boot_desc_tu10x.bin", "gsp_boot_tu10x.bin",
                "fwimage_tu10x.bin", "fwsignature_tu10x.bin", "sec2_bl_tu10x.bin",
                "v67-tu10x-four-plm.bin"
            )
            $unlockFiles += "v67-tu10x-four-plm.bin"
            Write-GspGateRecord @("tu10x")
        }
        "tu11x" {
            Write-Host "[warn] TU11x route (GTX 16 series / CMP 30HX) is RE-derived and NOT"
            Write-Host "[warn] field-proven (issue #151): die-keyed TU116 booter archive + the"
            Write-Host "[warn] TU102 frame table with the park word re-anchored (0x7EA4 -> 0x7E9A)."
            Write-Host "[warn] Read-back is the only verdict; a clean failure is an expected outcome."
            Write-Host "[extract] TU116/TU117 booter (BOOT_DIRECT, die-keyed) / shared Turing BL"
            & $AssetsExe extract-tu11x $SrcTar $Out
            if ($LASTEXITCODE) { throw "extract-tu11x failed" }
            if (-not $tuSectionsDone) {
                Write-Host "[extract] fwimage / fwsignature sections (tu10x ELF: $($tuSigFamilies -join ', '))"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_tu10x.bin") $Out $tuSigFamilies
                if ($LASTEXITCODE) { throw "gsp-sections failed" }
                $tuSectionsDone = $true
            }
            Write-Host "[generate] V67 TU11x payload (RE-derived, unproven)"
            & $AssetsExe generate-tu11x (Join-Path $Out "v67-tu11x-four-plm.bin")
            if ($LASTEXITCODE) { throw "generate-tu11x failed" }
            Copy-ProvenSlice "tu11x"
            $expected += @(
                "booter_load_tu11x.bin", "gsp_boot_desc_tu11x.bin", "gsp_boot_tu11x.bin",
                "fwimage_tu11x.bin", "fwsignature_tu11x.bin", "sec2_bl_tu11x.bin",
                "v67-tu11x-four-plm.bin"
            )
            $unlockFiles += "v67-tu11x-four-plm.bin"
            Write-GspGateRecord @("tu11x")
        }
    }
}

# ---------- intermediates off the root ----------
# The full GSP ELFs (gsp_ga10x.bin / gsp_tu10x.bin) are extraction
# INTERMEDIATES: gsp-sections has consumed them and the loader reads only
# fwimage_*/fwsignature_*/booter_load_*/... from the volume root - the ELF
# files (84 + 29 MB) would be pure dead weight on the stick. The makers
# delete them here instead of asking users to skip "extra .bin files".
foreach ($elf in @("gsp_ga10x.bin", "gsp_tu10x.bin")) {
    $elfPath = Join-Path $Out $elf
    if (Test-Path $elfPath) { Remove-Item $elfPath -Force }
}

$expected += "BOOTX64.EFI"
Copy-Item $BootEfi (Join-Path $Out "BOOTX64.EFI") -Force

# The firmware only auto-boots \EFI\BOOT\BOOTX64.EFI; a flat copy of $Out puts the
# executable at the root and the boot menu silently bounces back (issue #142). Emit
# the real path too, so "copy the contents" is literally sufficient.
New-Item -ItemType Directory -Force -Path (Join-Path $Out "EFI/BOOT") | Out-Null
Copy-Item $BootEfi (Join-Path $Out "EFI/BOOT/BOOTX64.EFI") -Force

foreach ($f in $expected) {
    if (-not (Test-Path (Join-Path $Out $f))) { throw "missing: $f" }
    Write-Host ("  {0,-28} {1,10} bytes" -f $f, (Get-Item (Join-Path $Out $f)).Length)
}
Write-Host ("  {0,-28} {1,10} bytes" -f "EFI/BOOT/BOOTX64.EFI", (Get-Item (Join-Path $Out "EFI/BOOT/BOOTX64.EFI")).Length)

# Layout marker: identifies a volume that already hosts THIS project's files
# so a later run may upgrade them in place. Any other existing file is never
# overwritten (data-safety contract; see the install offer at the end).
@{
    project  = "nv-uefi-unlocker"
    maker    = "make-usbroot.ps1"
    families = ($Families -join ",")
    built    = (Get-Date -Format o)
} | ConvertTo-Json | Set-Content -Path (Join-Path $Out "nvuu-stick.json") -Encoding Ascii

$outFull = (Resolve-Path -LiteralPath $Out).Path
$outRoot = [System.IO.Path]::GetPathRoot($outFull)
if ($outRoot -and ($outRoot.TrimEnd('\') -ieq $outFull.TrimEnd('\'))) {
    Write-Host "`nDONE ($($Families -join ', ')). $outFull already IS the stick root - nothing to copy, boot it directly."
} else {
    Write-Host "`nDONE ($($Families -join ', ')). The stick root must receive the layout; for example (X: = the FAT32 stick):"
    Write-Host ("  xcopy `"{0}\*`" X:\ /E /H /Y" -f $outFull)
    Write-Host "Or re-run with -Out X:\ to write the stick root directly."
}
Write-Host "The layout already contains EFI/BOOT/BOOTX64.EFI - keep that path, do not flatten it."
Write-Host "Unlock mode requires the v67-*.bin payload for the target card's family"
Write-Host "($($unlockFiles -join ', ')); removing it switches the EFI into"
Write-Host "replay/diagnostic mode for that family."
if ($Families -contains "tu10x" -or $Families -contains "tu11x") {
    Write-Host ""
    Write-Host "Turing reminder: the target Windows must run EnableGpuFirmware=1 AND"
    Write-Host "the unlock session must follow a RESTART - a hibernate resume does not"
    Write-Host "re-initialize the driver (gsp-gate.txt on the stick records this build)."
}

# ---------- install-target probe + one-click persistent install ----------
# The generated layout can be copied into an EXISTING FAT32 partition so the
# unlock does not require a dedicated USB stick every time: an internal
# (non-removable) disk's non-boot data FAT32 partition is offered for a
# persistent install, and a USB stick's FAT32 partition for the same copy.
# Data-safety contract (the issue #73 ruling kept intact): this probe NEVER
# creates, resizes, reformats or repartitions anything, NEVER touches an
# ESP/system/boot volume, and NEVER overwrites or deletes an existing file -
# a differing file at the target refuses the whole target, and only a
# previous nv-uefi-unlocker layout (identified by nvuu-stick.json) may be
# upgraded in place. A layout on an internal partition boots through the
# firmware's ONE-TIME boot menu; no NVRAM/BootOrder entry is ever written.

function Format-Size([long]$Bytes) {
    if ($Bytes -ge 1GB) { "{0:N1} GB" -f ($Bytes / 1GB) }
    elseif ($Bytes -ge 1MB) { "{0:N1} MB" -f ($Bytes / 1MB) }
    else { "{0:N0} KB" -f ($Bytes / 1KB) }
}

function Get-InstallPayload {
    $files = @(Get-ChildItem -LiteralPath $Out -Recurse -File)
    $sum = 0
    foreach ($f in $files) { $sum += $f.Length }
    [pscustomobject]@{ Count = $files.Count; Bytes = $sum }
}

function Test-OurLayoutMarker([string]$Root) {
    $m = Join-Path $Root "nvuu-stick.json"
    if (-not (Test-Path -LiteralPath $m)) { return $false }
    try { return ((Get-Content -LiteralPath $m -Raw -ErrorAction Stop) -match 'nv-uefi-unlocker') } catch { return $false }
}

# Enumerate FAT32 volumes that could receive the layout. Removable = USB
# bus; everything else counts as internal. ESP/system/boot partitions and
# the volume $Out itself sits on are never candidates.
function Get-Fat32InstallTargets([long]$NeededBytes, [string]$ExcludeRoot) {
    $targets = @()
    $vols = Get-Volume -ErrorAction SilentlyContinue |
        Where-Object { $_.DriveLetter -and $_.FileSystem -eq 'FAT32' -and $_.SizeRemaining -ge ($NeededBytes + 16MB) }
    foreach ($v in $vols) {
        $root = "$($v.DriveLetter):\"
        if ($root -ieq $ExcludeRoot) { continue }
        $part = Get-Partition -DriveLetter $v.DriveLetter -ErrorAction SilentlyContinue
        if (-not $part) { continue }
        if ($part.IsBoot -or $part.IsSystem) { continue }
        if ("$($part.GptType)".ToLowerInvariant() -eq '{c12a7328-f81f-11d2-ba4b-00a0c93ec93b}') { continue }
        $disk = Get-Disk -Number $part.DiskNumber -ErrorAction SilentlyContinue
        if (-not $disk) { continue }
        if ($disk.IsOffline -or $disk.IsReadOnly) { continue }
        $targets += [pscustomobject]@{
            Root      = $root
            Removable = ($disk.BusType -eq 'USB')
            Label     = "$($v.FileSystemLabel)"
            Free      = [long]$v.SizeRemaining
            Size      = [long]$v.Size
        }
    }
    return $targets
}

# Copy the whole layout into an existing volume root WITHOUT touching what
# is already there: byte-identical files are skipped, a differing file
# refuses the target unless nvuu-stick.json marks it as our own earlier
# install (upgrade path). Every written file is verified by SHA-256
# read-back; a verification failure fails the function (read-back is the
# only success criterion, file domain included).
function Copy-UnlockRoot([string]$From, [string]$To) {
    $fromFull = (Resolve-Path -LiteralPath $From).Path.TrimEnd('\')
    $ours = Test-OurLayoutMarker $To
    $plan = @()
    $conflicts = @()
    foreach ($f in (Get-ChildItem -LiteralPath $fromFull -Recurse -File)) {
        $rel = $f.FullName.Substring($fromFull.Length + 1)
        $dest = Join-Path $To $rel
        if (Test-Path -LiteralPath $dest) {
            $same = ((Get-Item -LiteralPath $dest).Length -eq $f.Length) -and
                    ((Get-FileHash -LiteralPath $dest -Algorithm SHA256).Hash -eq
                     (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash)
            if ($same) { continue }
            if ($ours) { $plan += [pscustomobject]@{ Src = $f.FullName; Dest = $dest; Rel = $rel; Overwrite = $true }; continue }
            $conflicts += $rel
        } else {
            $plan += [pscustomobject]@{ Src = $f.FullName; Dest = $dest; Rel = $rel; Overwrite = $false }
        }
    }
    if ($conflicts.Count -gt 0) {
        Write-Host "[install] REFUSED: the target already holds different file(s) and is not a nv-uefi-unlocker layout:"
        foreach ($c in $conflicts) { Write-Host "[install]   existing: $(Join-Path $To $c)" }
        Write-Host "[install] nothing was written (the tool never overwrites existing files)."
        Write-Host "[install] If it is a stale copy of this tool, delete those files yourself and re-run."
        return $false
    }
    $copied = 0
    foreach ($item in $plan) {
        $dir = Split-Path $item.Dest -Parent
        if ($dir -and -not (Test-Path -LiteralPath $dir)) {
            New-Item -ItemType Directory -Force -Path $dir | Out-Null
        }
        Copy-Item -LiteralPath $item.Src -Destination $item.Dest -Force
        $copied++
    }
    $bad = @()
    foreach ($item in $plan) {
        $srcHash = (Get-FileHash -LiteralPath $item.Src -Algorithm SHA256).Hash
        $dstHash = (Get-FileHash -LiteralPath $item.Dest -Algorithm SHA256).Hash
        if ($srcHash -ne $dstHash) { $bad += $item.Rel }
    }
    if ($bad.Count -gt 0) {
        Write-Host "[install] READ-BACK MISMATCH on $($To) - do not boot from this copy:"
        foreach ($b in $bad) { Write-Host "[install]   $b" }
        return $false
    }
    $total = @(Get-ChildItem -LiteralPath $fromFull -Recurse -File).Count
    Write-Host ("[install] done: {0} files written ({1} refreshed), {2} already identical -> {3} (SHA-256 read-back verified)" -f
        $total, $plan.Count, ($total - $plan.Count), $To)
    return $true
}

function Write-InstallGuidance([long]$NeededBytes) {
    Write-Host "[install] no install target was picked (none found, or both offers declined)."
    Write-Host "[install]   Internal (persistent) install: add a data FAT32 partition on an"
    Write-Host "[install]     INTERNAL (non-USB) disk - NOT the EFI system partition, NOT the"
    Write-Host "[install]     Windows boot volume (Disk Management: shrink a volume -> new simple"
    Write-Host "[install]     volume -> FAT32; 500 MB is ample). This tool never creates, resizes"
    Write-Host "[install]     or reformats partitions - it only copies files into an existing one."
    Write-Host ("[install]   USB install: any stick keeping at least {0} free on a FAT32" -f (Format-Size $NeededBytes))
    Write-Host "[install]     partition; the files are added alongside its existing contents."
}

function Invoke-InstallOffer {
    $payload = Get-InstallPayload
    Write-Host ""
    Write-Host ("[install] generated layout: {0} files, {1}" -f $payload.Count, (Format-Size $payload.Bytes))
    $excludeRoot = [System.IO.Path]::GetPathRoot((Resolve-Path -LiteralPath $Out).Path)

    # Explicit non-interactive target (-InstallTo D:\): same eligibility
    # checks as the probe, no menu.
    if ($InstallTo) {
        $wantRoot = [System.IO.Path]::GetPathRoot($InstallTo)
        if (-not $wantRoot) { throw "-InstallTo '$InstallTo' is not a volume root (use e.g. D:\)" }
        $match = @(Get-Fat32InstallTargets $payload.Bytes $excludeRoot | Where-Object { $_.Root -ieq $wantRoot })
        if ($match.Count -eq 0) {
            throw "-InstallTo $wantRoot is not an eligible FAT32 target (FAT32 volume, at least $(Format-Size $payload.Bytes) free, not ESP/system/boot, not the -Out volume)."
        }
        Write-Host ("[install] target: {0} (label '{1}', {2} free)" -f $match[0].Root, $match[0].Label, (Format-Size $match[0].Free))
        if (Copy-UnlockRoot $Out $match[0].Root) { Write-InstallBootHints }
        return
    }
    if ($NoInstallPrompt) {
        Write-Host "[install] probe skipped (-NoInstallPrompt)"
        return
    }

    $targets = @(Get-Fat32InstallTargets $payload.Bytes $excludeRoot)
    $internal = @($targets | Where-Object { -not $_.Removable })
    $usb = @($targets | Where-Object { $_.Removable })
    if ($internal.Count -eq 0 -and $usb.Count -eq 0) {
        Write-InstallGuidance $payload.Bytes
        return
    }
    Write-Host "[install] FAT32 install-target probe (nothing already on a target is ever overwritten):"
    $chosen = $null
    foreach ($phase in @(
        @{ Kind = "internal, non-boot data partitions (persistent install)"; List = $internal },
        @{ Kind = "USB / removable media"; List = $usb })) {
        if ($phase.List.Count -eq 0) { continue }
        Write-Host ("[install]   {0}:" -f $phase.Kind)
        $i = 1
        foreach ($t in $phase.List) {
            Write-Host ("[install]     [{0}] {1,-4} label=""{2}""  {3} free of {4}" -f
                $i, $t.Root, $t.Label, (Format-Size $t.Free), (Format-Size $t.Size))
            $i++
        }
        $ans = $null
        try { $ans = Read-Host "[install] install there? number = install, Enter = skip" } catch { $ans = $null }
        $n = 0
        if ($ans -and [int]::TryParse("$ans", [ref]$n) -and $n -ge 1 -and $n -le $phase.List.Count) {
            $chosen = $phase.List[$n - 1]
            break
        }
    }
    if (-not $chosen) {
        Write-InstallGuidance $payload.Bytes
        return
    }
    Write-Host ("[install] target: {0} (label ""{1}"", {2} free)" -f $chosen.Root, $chosen.Label, (Format-Size $chosen.Free))
    if (Copy-UnlockRoot $Out $chosen.Root) { Write-InstallBootHints }
}

function Write-InstallBootHints {
    Write-Host "[install] boot path: firmware ONE-TIME boot menu -> the UEFI entry for that"
    Write-Host "[install] volume -> \EFI\BOOT\BOOTX64.EFI. No NVRAM/BootOrder entry was"
    Write-Host "[install] created; the unlock itself stays volatile by design and is replayed"
    Write-Host "[install] at every boot."
    Write-Host "[install] caveat: not every firmware lists internal FAT partitions in its boot"
    Write-Host "[install] menu - the USB stick remains the universal path."
}

Invoke-InstallOffer
