# Harness for Copy-UnlockRoot (the make-usbroot.ps1 install-copy engine).
# Splices the install-offer function block out of the real maker and runs the
# four data-safety scenarios against plain directories: fresh install,
# identical rerun (zero writes), foreign-file refusal (nothing written), and
# nvuu-stick.json upgrade (overwrite ours, preserve theirs).
# Usage: powershell -File test-copy.ps1 [-Root <usbroot to install from>]
# Build the fixture first, e.g.:
#   powershell release/make-usbroot.ps1 -Family tu10x -SkipGspCheck -NoInstallPrompt -Out .relcheck/usbroot-fixture
param([string]$Root = "")
$ErrorActionPreference = 'Stop'
$repo = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$src = Get-Content "$repo/release/make-usbroot.ps1" -Raw
$idx = $src.IndexOf('# ---------- install-target probe')
if ($idx -lt 0) { throw 'install-offer block marker not found' }
$block = $src.Substring($idx)
if (-not $Root) { $Root = "$repo/.relcheck/usbroot-fixture" }
if (-not (Test-Path "$Root/EFI/BOOT/BOOTX64.EFI")) { throw "fixture missing: $Root (build it with make-usbroot.ps1 -NoInstallPrompt first)" }
$Out = $Root
$InstallTo = ''
$NoInstallPrompt = $true
Invoke-Expression $block

$target = "$repo/.relcheck/copy-target"
Remove-Item -Recurse -Force $target -ErrorAction SilentlyContinue

# 1. fresh install
New-Item -ItemType Directory -Force -Path $target | Out-Null
$r = Copy-UnlockRoot $Out $target
Write-Host ("TEST1 fresh: ok={0}" -f $r)
if (-not $r) { exit 1 }
if (-not (Test-Path "$target/EFI/BOOT/BOOTX64.EFI")) { Write-Host 'TEST1 FAIL: EFI/BOOT missing'; exit 1 }

# 2. re-install: everything identical -> zero writes, still ok
$r = Copy-UnlockRoot $Out $target
Write-Host ("TEST2 identical rerun: ok={0}" -f $r)
if (-not $r) { exit 1 }

# 3. foreign target (no marker) with a differing file -> refuse, untouched
Remove-Item -Recurse -Force "$target/EFI" -ErrorAction SilentlyContinue
Set-Content -Path "$target/booter_load_tu10x.bin" -Value 'foreign bytes' -Encoding Ascii
Remove-Item -Path "$target/nvuu-stick.json" -Force -ErrorAction SilentlyContinue
foreach ($f in @('BOOTX64.EFI','fwimage_tu10x.bin','fwsignature_tu10x.bin','gsp-gate.txt',
                 'gsp_boot_desc_tu10x.bin','gsp_boot_tu10x.bin','sec2_bl_tu10x.bin',
                 'v67-tu10x-four-plm.bin')) {
    Remove-Item -Path "$target/$f" -Force -ErrorAction SilentlyContinue
}
Set-Content -Path "$target/some-user-file.txt" -Value 'user data' -Encoding Ascii
$before = (Get-FileHash "$target/some-user-file.txt" -Algorithm SHA256).Hash
$r = Copy-UnlockRoot $Out $target
$after = (Get-FileHash "$target/some-user-file.txt" -Algorithm SHA256).Hash
Write-Host ("TEST3 conflict refuse: ok={0} (expected False), user file untouched={1}" -f $r, ($before -eq $after))
if ($r -or ($before -ne $after)) { exit 1 }

# 4. our marker present -> upgrade overwrites the differing file, keeps theirs
@{ project = "nv-uefi-unlocker"; maker = "make-usbroot.ps1" } | ConvertTo-Json |
    Set-Content -Path "$target/nvuu-stick.json" -Encoding Ascii
$r = Copy-UnlockRoot $Out $target
$restored = (Get-FileHash "$target/booter_load_tu10x.bin" -Algorithm SHA256).Hash -eq
            (Get-FileHash "$Out/booter_load_tu10x.bin" -Algorithm SHA256).Hash
Write-Host ("TEST4 marker upgrade: ok={0}, overwritten-and-verified={1}" -f $r, $restored)
if (-not ($r -and $restored)) { exit 1 }

Write-Host 'ALL COPY HARNESS TESTS PASSED'
