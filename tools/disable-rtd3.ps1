# disable-rtd3.ps1 - one-shot fixer for "NVIDIA runtime D3 enabled" on Windows.
#
# The volatile unlock is wiped when runtime power management parks the dGPU in
# D3cold after the handoff (row 1 of the relock mechanism table, docs/faq.md
# #relock). The documented Windows-side fix (docs/faq.md#relock-winrtd3) is the
# display-adapter registry DWORD EnableRuntimePowerManagement = 0; this script
# applies the whole documented procedure in one command:
#
#   .\tools\disable-rtd3.ps1           apply the key to every NVIDIA display adapter
#   .\tools\disable-rtd3.ps1 -Check    read-only status report, no writes
#   .\tools\disable-rtd3.ps1 -Undo     remove the key again (restores driver default)
#
# What it writes: exactly one DWORD (EnableRuntimePowerManagement = 0) under the
# dynamically resolved driver key of each present NVIDIA display adapter - the
# key index (...\000x) changes whenever a driver is (re)installed, so it is
# resolved at run time, never hardcoded. It never touches other values, never
# uninstalls drivers, never reboots. Removal (-Undo) deletes the value, which
# restores the driver default (runtime power management allowed).
#
# Boundaries (docs/faq.md#relock-winrtd3): this blocks the DRIVER-initiated
# idle park only - sleep / S4 / a whole-machine power loss still clear the
# unlock (the AON island does not survive power loss), and with the key set the
# card never parks, so idle power rises. The key must be re-checked after a
# driver upgrade or reinstall (the driver key can be re-created). The setting
# takes effect at the next driver init (reboot or device re-enable); replay
# the USB unlock after that.
#
# SPDX-License-Identifier: GPL-2.0-only

param(
    [switch]$Check,
    [switch]$Undo
)

$ErrorActionPreference = 'Stop'
$ValueName = 'EnableRuntimePowerManagement'

function Say([string]$m)  { Write-Output $m }
function Warn([string]$m) { Write-Output "WARNING: $m" }
function Die([string]$m)  { Write-Output "ERROR: $m"; exit 1 }

# --- admin check for the write modes -----------------------------------------

$ident = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$admin = (New-Object System.Security.Principal.WindowsPrincipal($ident)).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $Check -and -not $admin) {
    Die "write modes need an elevated prompt - re-run from an admin PowerShell (or use -Check for the read-only report)"
}

# --- discovery ----------------------------------------------------------------

# Present NVIDIA display adapters (the GPU function; the audio function lives
# under a different class and is skipped by the class filter).
$adapters = @(Get-PnpDevice -Class Display | Where-Object {
    $_.InstanceId -like 'PCI\VEN_10DE*' -and $_.Present
})
if ($adapters.Count -eq 0) {
    Die "no present NVIDIA display adapter found - nothing to do"
}

# --- resolve + report / write -------------------------------------------------

$changed = 0
foreach ($dev in $adapters) {
    $id = $dev.InstanceId
    $name = $dev.FriendlyName
    $enumPath = "HKLM:\SYSTEM\CurrentControlSet\Enum\$id"
    $driverRel = (Get-ItemProperty -Path $enumPath -ErrorAction SilentlyContinue).Driver
    if (-not $driverRel) {
        Say "SKIP  $name`n      no Driver value under $enumPath (device not started?)"
        continue
    }
    $classKey = "HKLM:\SYSTEM\CurrentControlSet\Control\Class\$driverRel"
    if (-not (Test-Path $classKey)) {
        Say "SKIP  $name`n      driver key $classKey does not exist"
        continue
    }

    $prop = Get-ItemProperty -Path $classKey -ErrorAction SilentlyContinue
    $cur = $prop.$ValueName

    if ($Check) {
        $state = if ($null -eq $cur) { 'absent (driver default: runtime D3 allowed)' }
                 elseif ($cur -eq 0)  { '0 = runtime D3 DISABLED (unlock-retaining shape)' }
                 else { "$cur = runtime D3 enabled (the relock shape)" }
        Say "CHECK $name"
        Say "      driver key : $driverRel"
        Say "      $ValueName = $state"
        continue
    }

    if ($Undo) {
        if ($null -eq $cur) {
            Say "UNDO  $name : $ValueName already absent (driver default) - nothing to do"
            continue
        }
        Remove-ItemProperty -Path $classKey -Name $ValueName
        $after = (Get-ItemProperty -Path $classKey).$ValueName
        if ($null -ne $after) { Die "undo failed on ${classKey}: value still present after removal" }
        Say "UNDO  $name : $ValueName removed (previous value $cur) - driver default restored"
        continue
    }

    # apply
    if ($null -ne $cur -and $cur -eq 0) {
        Say "OK    $name : $ValueName already 0 - nothing to do"
        continue
    }
    if ($null -ne $cur -and $cur -ne 0) {
        Warn "$name : existing $ValueName = $cur will be overwritten with 0 (note the old value down if you need it back)"
    }
    New-ItemProperty -Path $classKey -Name $ValueName -Value 0 -PropertyType DWord -Force | Out-Null
    $after = (Get-ItemProperty -Path $classKey).$ValueName
    if ($after -ne 0) { Die "write failed on ${classKey}: readback is $after" }
    Say "SET   $name : $ValueName = 0 (readback ok)  [$id]"
    $changed++
}

# --- closing notes ------------------------------------------------------------

if ($Check) {
    Say ""
    Say "notes: value 0 = the unlock-retaining shape (docs/faq.md#relock-winrtd3);"
    Say "       absent = driver default, the runtime park that clears the unlock is allowed;"
    Say "       sleep / S4 / a whole-machine power loss still clear the unlock either way."
    exit 0
}
if ($Undo) {
    Say ""
    Say "done - driver default restored on all touched adapters."
    exit 0
}
Say ""
Say "done - $changed adapter(s) changed."
Say "takes effect at the next driver init: reboot (or disable+re-enable the adapter),"
Say "then replay the USB unlock and verify with ssdiag (UNLOCKED + canary intact)."
Say "re-run this script with -Check after any NVIDIA driver upgrade - the driver key"
Say "can be re-created by the installer and the value lost."
