# make-provenance.ps1 - emit a SOURCE-MANIFEST + PROVENANCE pair for a built
# stick root (or any release directory), so a field-verified build is
# reproducible and checkable offline:
#
#   SOURCE-MANIFEST.json  every file under -Dir: relative path, size, SHA-256
#   PROVENANCE.json       manifest hash + optional field-proven EFI hash +
#                         rebuild declaration (rebuild via make-usbroot.ps1
#                         from pinned public inputs must match every row)
#
# Re-runs are idempotent: both outputs are excluded from the manifest.
#
# Usage:
#   .\make-provenance.ps1 -Dir .\usbroot
#   .\make-provenance.ps1 -Dir .\usbroot -EfiSha256 <sha-of-field-verified-efi> `
#                        -Note "v2.1.6 stable, TU106 1f07 full chain"
#
# Requires: PowerShell 5+.

param(
    [Parameter(Mandatory = $true)][string]$Dir,
    [string]$OutManifest = "",
    [string]$OutProvenance = "",
    # SHA-256 of the BOOTX64.EFI that actually passed on real hardware, if
    # known. Recorded verbatim so users can compare their rebuilt binary.
    [string]$EfiSha256 = "",
    [string]$Note = ""
)

$ErrorActionPreference = "Stop"
if (-not (Test-Path -LiteralPath $Dir -PathType Container)) {
    throw "directory not found: $Dir"
}
$Dir = (Resolve-Path -LiteralPath $Dir).Path
if (-not $OutManifest)   { $OutManifest   = Join-Path $Dir "SOURCE-MANIFEST.json" }
if (-not $OutProvenance) { $OutProvenance = Join-Path $Dir "PROVENANCE.json" }
$skip = @{}
foreach ($p in @($OutManifest, $OutProvenance)) {
    $skip[$p.ToLowerInvariant()] = $true
}

$files = Get-ChildItem -LiteralPath $Dir -Recurse -File | Sort-Object FullName
$rows = @()
$total = 0L
foreach ($f in $files) {
    $full = $f.FullName
    if ($skip.ContainsKey($full.ToLowerInvariant())) { continue }
    $hash = (Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash.ToLowerInvariant()
    $rel = $full.Substring($Dir.Length).Replace("\", "/").TrimStart("/")
    $rows += [PSCustomObject]@{ path = $rel; bytes = $f.Length; sha256 = $hash }
    $total += $f.Length
}
$rows = $rows | Sort-Object path

$stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

# Write UTF-8 without a BOM on every PS version: PS 5.1 rejects
# -Encoding utf8NoBOM at parameter-bind time, so .NET it is.
function Write-NoBom([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, (New-Object Text.UTF8Encoding($false)))
}

$manifest = [PSCustomObject]@{
    schema       = "nvuu-source-manifest/1"
    # No timestamp here on purpose: the manifest is purely content-derived,
    # so two runs over identical content produce byte-identical manifests
    # and the hash below doubles as a reproducibility check.
    root         = (Split-Path -Leaf $Dir)
    file_count   = $rows.Count
    files        = $rows
}
Write-NoBom $OutManifest ($manifest | ConvertTo-Json -Depth 4)

$manifestHash = (Get-FileHash -LiteralPath $OutManifest -Algorithm SHA256).Hash.ToLowerInvariant()
$provenance = [PSCustomObject]@{
    schema                 = "nvuu-provenance/1"
    generated_utc          = $stamp
    source_manifest        = (Split-Path -Leaf $OutManifest)
    source_manifest_sha256 = $manifestHash
    file_count             = $rows.Count
    total_bytes            = $total
    field_proven_efi_sha256 = if ($EfiSha256) { $EfiSha256.ToLowerInvariant() } else { $null }
    rebuild_declaration    = ("rebuilding this directory via release/make-usbroot.ps1 from the pinned public inputs " +
                              "must reproduce every sha256 in {0}" -f (Split-Path -Leaf $OutManifest))
    note                   = $Note
}
Write-NoBom $OutProvenance ($provenance | ConvertTo-Json -Depth 4)

Write-Host ("manifest : {0}  ({1} files, {2} bytes)" -f $OutManifest, $rows.Count, $total)
Write-Host ("provenance: {0}  (manifest sha256 {1})" -f $OutProvenance, $manifestHash)
