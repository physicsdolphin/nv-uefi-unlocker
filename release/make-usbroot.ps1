# make-usbroot.ps1 - regenerate the unlock USB stick root from pinned public
# inputs. NVIDIA firmware blobs are NOT distributed with this project; this
# script downloads the pinned public packages, verifies their SHA-256, and
# regenerates every file the EFI application needs for the chosen GPU family.
#
# Usage:
#   .\make-usbroot.ps1                     (localhost: detect this machine's GPUs)
#   .\make-usbroot.ps1 -Family tu10x -Out E:\
#   .\make-usbroot.ps1 -Family all         (every RTX 20/30/40 family)
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
    # Display-class registry key to inspect (override for tests/offline hives).
    [string]$ClassKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}',
    # PnP's bound-device list for the NVIDIA driver (override for tests/offline
    # hives). Only devnodes the driver actually bound to this boot appear here.
    [string]$NvServiceEnumKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\nvlddmkm\Enum'
)

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
    },
    @{
        Name = "nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst"
        Urls = @("https://archive.cachyos.org/archive/cachyos-v3/nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst")
        Sha256 = "E17E7C92EEFFA178B0FE1F40EB17A47095F8FC3CE4A77BF115CF5BD5A4DF86E9"
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
New-Item -ItemType Directory -Force -Path $Work, $Out | Out-Null

foreach ($s in $Sources) {
    $dest = Join-Path $Work $s.Name
    $part = "$dest.part"
    # A file already in the work dir with the pinned hash is accepted as-is:
    # that is the sanctioned offline path (obtain the exact bytes by any
    # transport, drop them here, build).
    if ((Test-Path $dest) -and (Get-FileHash -Algorithm SHA256 $dest).Hash -eq $s.Sha256) {
        Write-Host "[ok] $($s.Name) SHA-256 verified (cached)"
        continue
    }
    Remove-Item $dest -Force -ErrorAction SilentlyContinue
    # The SHA-256 pin is the authority, not the URL: try every transport.
    # Download to .part and hash it there - a killed or corrupted transfer
    # never lands at $dest (issue #59).
    $done = $false
    foreach ($pass in 1..2) {
        foreach ($u in $s.Urls) {
            if (Test-Path $part) { Remove-Item $part -Force }
            Write-Host ("[fetch] {0} <- {1}{2}" -f $s.Name, $u, $(if ($pass -gt 1) { " (retry)" } else { "" }))
            try {
                Invoke-WebRequest -Uri $u -OutFile $part
            } catch {
                Remove-Item $part -Force -ErrorAction SilentlyContinue
                continue
            }
            $hash = (Get-FileHash -Algorithm SHA256 $part).Hash
            if ($hash -eq $s.Sha256) {
                Move-Item -Force $part $dest
                $done = $true
                break
            }
            Write-Host "[warn] $($s.Name) hash mismatch from $u - refusing this copy, trying next source"
            Remove-Item $part -Force
        }
        if ($done) { break }
    }
    if (-not $done) {
        Remove-Item $part -Force -ErrorAction SilentlyContinue
        throw "download failed for $($s.Name): no pinned source served the pinned bytes ($($s.Sha256)).
Drop the exact file into $Work yourself and re-run to build offline."
    }
    Write-Host "[ok] $($s.Name) SHA-256 verified"
}

$SrcTar = Join-Path $Work $Sources[0].Name
$UtilsZst = Join-Path $Work $Sources[1].Name

# Remove assets from an earlier run so a root cannot end up as a half-old,
# half-new mix (e.g. a 40-series v67 payload next to a 20-series booter).
Get-ChildItem $Out -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '^(booter_load_|gsp_boot|gsp_|fwimage_|fwsignature_|sec2_bl_|scrubber_|v67-|gsp-gate\.txt$|BOOTX64\.EFI$)' } |
    ForEach-Object { Remove-Item $_.FullName -Force }

# ---------- shared extraction (one pass for the whole build) ----------
# fw always extracts BOTH GSP ELFs (gsp_ga10x.bin + gsp_tu10x.bin) in a
# single zst pass, so it runs once no matter how many families build. The
# scrubber is family-neutral too and shared by the ad102/ga102 branches.
Write-Host "[extract] GSP firmware images (gsp_ga10x + gsp_tu10x)"
& $AssetsExe fw $UtilsZst $Out
if ($LASTEXITCODE) { throw "fw failed" }

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
        foreach ($u in $script:ProvenBlobUrls) {
            Write-Host "[fetch] $($script:ProvenBlobName) <- $u"
            try {
                Invoke-WebRequest -Uri $u -OutFile $blob
                break
            } catch {
                Remove-Item $blob -Force -ErrorAction SilentlyContinue
            }
        }
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
                Write-Host "[extract] fwimage / fwsignature sections (ga10x ELF)"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_ga10x.bin") $Out
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
                Write-Host "[extract] fwimage / fwsignature sections (ga10x ELF)"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_ga10x.bin") $Out
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
                Write-Host "[extract] fwimage / fwsignature sections (tu10x ELF)"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_tu10x.bin") $Out
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
                Write-Host "[extract] fwimage / fwsignature sections (tu10x ELF)"
                & $AssetsExe gsp-sections (Join-Path $Out "gsp_tu10x.bin") $Out
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
