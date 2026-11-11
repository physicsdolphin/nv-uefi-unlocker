# Syntax gate for the maker scripts (usage: powershell -File parse-maker.ps1 [script ...]
# with no arguments checks both release makers). Returns nonzero on parse errors.
$repo = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$targets = if ($args.Count) { $args } else {
    @("$repo/release/make-usbroot.ps1", "$repo/release/make-usbroot.sh")
}
foreach ($t in $targets) {
    if ($t -notlike '*.ps1') {
        bash -n $t
        if ($LASTEXITCODE -ne 0) { exit 1 }
        Write-Host "SH SYNTAX OK: $t"
        continue
    }
    $errs = $null
    $null = [System.Management.Automation.Language.Parser]::ParseFile($t, [ref]$null, [ref]$errs)
    if ($errs.Count) {
        $errs | ForEach-Object { Write-Host ("line {0}: {1}" -f $_.Extent.StartLineNumber, $_.Message) }
        exit 1
    }
    Write-Host "PS1 SYNTAX OK: $t"
}
