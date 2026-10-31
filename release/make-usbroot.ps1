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
#   tu10x | rtx20 | 20   RTX 20 / 16 series (Turing)  - REQUIRES GSP firmware mode.
#                        The script refuses to build unless EnableGpuFirmware=1
#                        and nvidia-smi shows a running GSP firmware; the legacy
#                        driver path TDR-loops on the unlocked card. Building on a
#                        DIFFERENT machine than the target? Pass -SkipGspCheck.
#   localhost            enumerate the NVIDIA GPUs in THIS machine (display-class
#                        registry, else nvidia-smi) and build exactly the families
#                        found there; throws when no RTX 20/30/40 card is present.
#   all                  build all three 20-40 series families on one stick. The
#                        Turing GSP gate runs lenient: still refuses when a LOCAL
#                        Turing card sits in legacy mode, but skips (no prompt)
#                        when this host has no Turing adapter at all.
#
# Requires: PowerShell 5+ and internet access to the pinned URLs.

param(
    [ValidateSet("ad102", "ga102", "tu10x", "all", "localhost",
                 "rtx40", "rtx30", "rtx20", "40", "30", "20")]
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
    [string]$ClassKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}'
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
    default { $familyArg }   # localhost | all pass through
}

# Device id -> build family (same ranges as loader/src/main.rs).
function Get-UnlockFamily([string]$DevHex) {
    $dev = [Convert]::ToInt32($DevHex, 16)
    if (($dev -ge 0x1E02 -and $dev -le 0x1FFF) -or
        ($dev -ge 0x2182 -and $dev -le 0x21D1)) { return "tu10x" }
    if (($dev -ge 0x2200 -and $dev -le 0x22FF) -or
        ($dev -ge 0x2400 -and $dev -le 0x25FF)) { return "ga102" }
    if ($dev -ge 0x2600 -and $dev -le 0x28FF) { return "ad102" }
    return $null
}

# localhost: NVIDIA display adapters in THIS machine. The display-class
# registry is authoritative; nvidia-smi is the fallback when the class key
# has no matching entries.
function Get-LocalNvidiaAdapters {
    $adapters = @()
    Get-ChildItem $ClassKey -ErrorAction SilentlyContinue | ForEach-Object {
        $props = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
        if ($props.MatchingDeviceId -and
            $props.MatchingDeviceId -match '(?i)ven_10de&dev_([0-9a-f]{4})') {
            $adapters += [pscustomobject]@{ Dev = $Matches[1].ToLower(); Desc = "$($props.DriverDesc)" }
        }
    }
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
        throw "localhost: no supported NVIDIA adapter (RTX 20/30/40 series) found on
this machine. Use an explicit -Family (ad102 | ga102 | tu10x) or -Family all."
    }
    # Canonical build order regardless of enumeration order.
    $Families = @("tu10x", "ga102", "ad102") | Where-Object { $detected -contains $_ }
    Write-Host ("[localhost] building for: {0}" -f ($Families -join ", "))
} else {
    $Families = @($normalized)
}

# Pinned public inputs (see AGENTS.md / assets/src/manifest.rs).
$Sources = @(
    @{
        Name = "NVIDIA-kernel-module-source-610.43.03.tar.xz"
        Url  = "https://download.nvidia.com/XFree86/NVIDIA-kernel-module-source/NVIDIA-kernel-module-source-610.43.03.tar.xz"
        Sha256 = "7E118923C7A23EDC36114D63273A46E3E04E9AF98695A42203E7AC2DFE9FC1DC"
    },
    @{
        Name = "nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst"
        Url  = "https://archive.cachyos.org/archive/cachyos-v3/nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst"
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
$ProvenBlobName = "gsp_rm_boot_dbg.bin"
$ProvenBlobSrcSha256 = "FA43239BCEE7B97CA62F007CC68487560A39E19F74F3DDE7486DB3F98DF8E471"
$ProvenBlobUrl = "https://raw.githubusercontent.com/xrip/cmp50hx-unlock/master/efi-unlock/blobs/gsp_rm_boot_dbg.bin"
$ProvenSliceLen = 0x6000
$ProvenSliceSha256 = "de676bae28a480011d3d012db14bef539324e62a841a9627863c689bea168af3"

# ---------- Turing GSP-mode gate (refuse without GSP firmware mode) ----------
# Turing's Windows driver only accepts a pre-OS FEAT unlock in GSP-RM firmware
# mode; the legacy path TDR-loops (Code 43 / watchdog storm) on the unlocked
# card. Building a tu10x stick on a machine that has not enabled GSP mode is
# how users brick their session - refuse here, point at the fix.

function Test-TuringDevId([string]$MatchingDeviceId) {
    if ($MatchingDeviceId -notmatch '(?i)ven_10de&dev_([0-9a-f]{4})') { return $false }
    $dev = [Convert]::ToInt32($Matches[1], 16)
    return (($dev -ge 0x1E02 -and $dev -le 0x1FFF) -or
            ($dev -ge 0x2182 -and $dev -le 0x21D1))
}

function Get-TuringAdapterState {
    Get-ChildItem $ClassKey -ErrorAction SilentlyContinue | ForEach-Object {
        $props = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
        if (-not $props.MatchingDeviceId -or -not (Test-TuringDevId $props.MatchingDeviceId)) { return }
        [pscustomobject]@{
            Slot = $_.PSChildName
            Desc = $props.DriverDesc
            Gsp  = $props.EnableGpuFirmware
        }
    }
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
if ($Families -contains "tu10x" -and -not $SkipGspCheck) {
    Write-Host "[gate] Turing family selected - checking Windows GSP firmware mode"
    $gateStatus = "passed"
    $turing = @(Get-TuringAdapterState)
    if ($turing.Count -eq 0) {
        if ($gateStrict) {
            Write-Warning "No NVIDIA Turing display adapter found on THIS machine."
            Write-Host "If the unlock target is a different machine, pass -SkipGspCheck."
            try {
                $answer = Read-Host "Continue anyway? (y/N)"
            } catch {
                $answer = "n"
            }
            if ($answer -notmatch '^(?i)y') { throw "Aborted by user." }
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
            Write-Host ""
            throw "Turing adapter(s) above are NOT in GSP firmware mode.
Fix BEFORE using this stick: run .\enable-turing-unlock.ps1 from an
elevated PowerShell, then RESTART (a hibernate resume does NOT
re-initialize the driver and will not apply the mode). Re-run this
script after the restart."
        }

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
                throw "Registry says EnableGpuFirmware=1 but nvidia-smi reports GSP firmware N/A.
The setting is not active in this driver session: RESTART (not hibernate
resume) and re-run. If it persists, the driver may be too old for GSP."
            }
        }
        Write-Host "[ok] all Turing adapters in GSP firmware mode"
    }
} elseif ($Families -contains "tu10x") {
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
    foreach ($attempt in 1..2) {
        if (-not (Test-Path $dest)) {
            # Never write to $dest directly: an interrupted transfer used to leave a
            # partial file there, and the "already downloaded" check below then made
            # every later run report a pinned-input SHA-256 mismatch instead of
            # re-fetching (issue #59).
            if (Test-Path $part) { Remove-Item $part -Force }
            Write-Host ("[fetch] {0}{1}" -f $s.Name, $(if ($attempt -gt 1) { " (retry)" } else { "" }))
            try {
                Invoke-WebRequest -Uri $s.Url -OutFile $part
                Move-Item -Force $part $dest
            } catch {
                Remove-Item $part -Force -ErrorAction SilentlyContinue
                if ($attempt -eq 2) { throw }
                continue
            }
        }
        $hash = (Get-FileHash -Algorithm SHA256 $dest).Hash
        if ($hash -eq $s.Sha256) { break }
        # A stale/corrupt local file must not masquerade as a pin mismatch: drop it
        # and fetch once more. The pinned hash still gates the result either way.
        Write-Host "[warn] $($s.Name) hash mismatch on attempt $attempt - discarding and re-fetching"
        Remove-Item $dest -Force
        if ($attempt -eq 2) {
            throw "SHA-256 mismatch for $($s.Name): got $hash want $($s.Sha256) - refusing (pinned inputs only)"
        }
    }
    # Reaching here means the loop exited via break on a matching hash - every
    # other exit path throws, so the [ok] line is unconditional.
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

if (@($Families | Where-Object { $_ -ne "tu10x" }).Count -gt 0) {
    Write-Host "[extract] SEC2 scrubber (shared: ad102/ga102)"
    & $AssetsExe extract-scrub $SrcTar $Out
    if ($LASTEXITCODE) { throw "extract-scrub failed" }
}

# ---------- per-family extraction ----------
$expected = @()
$unlockFiles = @()
$ga10xSectionsDone = $false
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
            Write-Host "[extract] fwimage / fwsignature sections (tu10x ELF)"
            & $AssetsExe gsp-sections (Join-Path $Out "gsp_tu10x.bin") $Out
            if ($LASTEXITCODE) { throw "gsp-sections failed" }
            Write-Host "[generate] V67 TU10x payload"
            & $AssetsExe generate-tu10x (Join-Path $Out "v67-tu10x-four-plm.bin")
            if ($LASTEXITCODE) { throw "generate-tu10x failed" }

            # Resolve the proven GspRmBoot archive: explicit path, next to this
            # script, reference checkout, work dir, then the pinned URL.
            if ($ProvenBlob -and -not (Test-Path $ProvenBlob)) {
                throw "-ProvenBlob not found: $ProvenBlob"
            }
            $blob = @(
                $ProvenBlob,
                (Join-Path $PSScriptRoot $ProvenBlobName),
                (Join-Path (Split-Path $PSScriptRoot -Parent) "reference\cmp50hx-unlock\efi-unlock\blobs\$ProvenBlobName"),
                (Join-Path $Work $ProvenBlobName)
            ) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
            if (-not $blob) {
                $blob = Join-Path $Work $ProvenBlobName
                Write-Host "[fetch] $ProvenBlobName (proven GspRmBoot archive)"
                Invoke-WebRequest -Uri $ProvenBlobUrl -OutFile $blob
            }
            $blobHash = (Get-FileHash -Algorithm SHA256 $blob).Hash
            if ($blobHash -ne $ProvenBlobSrcSha256) {
                throw "proven GspRmBoot blob mismatch: got $blobHash want $ProvenBlobSrcSha256 ($blob) - refusing (the 128 KiB cmp50hx archive is the only accepted source; the gsp_tu10x.bin ELF is NOT it)"
            }
            Write-Host "[ok] proven GspRmBoot archive SHA-256 verified ($((Get-Item $blob).Length) bytes)"
            Write-Host "[slice] first 0x6000 bytes -> gsp_boot_tu10x.bin"
            & $AssetsExe slice-proven-rm-boot $blob $Out
            if ($LASTEXITCODE) { throw "slice-proven-rm-boot failed" }

            # The proven slice is byte-pinned: exactly 0x6000 bytes, de676bae...
            # A 4 KiB v4 BINDATA image or the GSP ELF's first 0x6000 (c76c43b1...)
            # must never ship.
            $bootOut = Join-Path $Out "gsp_boot_tu10x.bin"
            $len = (Get-Item $bootOut).Length
            if ($len -ne $ProvenSliceLen) {
                throw "gsp_boot_tu10x.bin is $len bytes, want $ProvenSliceLen (0x6000): trimmed proven GspRmBoot slice only"
            }
            $bootHash = (Get-FileHash -Algorithm SHA256 $bootOut).Hash.ToLowerInvariant()
            if ($bootHash -ne $ProvenSliceSha256) {
                throw "gsp_boot_tu10x.bin hash $bootHash != pinned $ProvenSliceSha256"
            }
            Write-Host "[ok] gsp_boot_tu10x.bin = 0x6000 bytes, de676bae... (proven slice, byte-identical to the fired TU106 run)"

            $expected += @(
                "booter_load_tu10x.bin", "gsp_boot_desc_tu10x.bin", "gsp_boot_tu10x.bin",
                "fwimage_tu10x.bin", "fwsignature_tu10x.bin", "sec2_bl_tu10x.bin",
                "v67-tu10x-four-plm.bin"
            )
            $unlockFiles += "v67-tu10x-four-plm.bin"
            @(
                "nv-uefi-unlocker Turing GSP gate"
                "time: $(Get-Date -Format o)"
                "host: $env:COMPUTERNAME"
                "family: tu10x"
                "gate: $gateStatus"
                "adapters:"
                Get-TuringAdapterState | ForEach-Object { "  $($_.Slot) $($_.Desc) EnableGpuFirmware=$($_.Gsp)" }
            ) | Set-Content -Path (Join-Path $Out "gsp-gate.txt") -Encoding Ascii
        }
    }
}

$expected += "BOOTX64.EFI"
Copy-Item $BootEfi (Join-Path $Out "BOOTX64.EFI") -Force

foreach ($f in $expected) {
    if (-not (Test-Path (Join-Path $Out $f))) { throw "missing: $f" }
    Write-Host ("  {0,-28} {1,10} bytes" -f $f, (Get-Item (Join-Path $Out $f)).Length)
}

Write-Host "`nDONE ($($Families -join ', ')). Copy the CONTENTS of $Out to a FAT32 USB stick root."
Write-Host "Unlock mode requires the v67-*.bin payload for the target card's family"
Write-Host "($($unlockFiles -join ', ')); removing it switches the EFI into"
Write-Host "replay/diagnostic mode for that family."
if ($Families -contains "tu10x") {
    Write-Host ""
    Write-Host "Turing reminder: the target Windows must run EnableGpuFirmware=1 AND"
    Write-Host "the unlock session must follow a RESTART - a hibernate resume does not"
    Write-Host "re-initialize the driver (gsp-gate.txt on the stick records this build)."
}
