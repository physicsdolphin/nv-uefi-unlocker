# 仅执行宿主测试和编译，不操作 U 盘、GPU、ESP 或启动项。
param([string]$RustToolsRoot = '')

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
$toolRoot = if ($RustToolsRoot) { $RustToolsRoot } else { Join-Path (Split-Path $repoRoot -Parent) 'rust-tools' }
$env:RUSTUP_HOME = Join-Path $toolRoot 'rustup'
$env:CARGO_HOME = Join-Path $toolRoot 'cargo'
$env:RUSTUP_TOOLCHAIN = '1.99.1'
$env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH

$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$vsRoot = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsRoot) { throw '未找到 C++ 链接工具。' }
$devCmd = Join-Path $vsRoot 'Common7\Tools\VsDevCmd.bat'
$envLines = & cmd.exe /d /c "call `"$devCmd`" -arch=amd64 -host_arch=amd64 >nul && set"
if ($LASTEXITCODE -ne 0) { throw '编译环境初始化失败。' }
foreach ($line in $envLines) {
    $at = $line.IndexOf('=')
    if ($at -gt 0) {
        [Environment]::SetEnvironmentVariable($line.Substring(0, $at), $line.Substring($at + 1), 'Process')
    }
}

$artifacts = Join-Path $repoRoot 'local-artifacts'
New-Item -Path $artifacts -ItemType Directory -Force | Out-Null
Push-Location $repoRoot
try {
    & cargo xt 2>&1 | Tee-Object -FilePath (Join-Path $artifacts 'host-tests.log')
    if ($LASTEXITCODE -ne 0) { throw '宿主测试未通过。' }
    & cargo efi 2>&1 | Tee-Object -FilePath (Join-Path $artifacts 'build-readonly.log')
    if ($LASTEXITCODE -ne 0) { throw '只读 EFI 编译失败。' }
    $efi = Join-Path $repoRoot 'target\x86_64-unknown-uefi\release\loader.efi'
    Copy-Item -LiteralPath $efi -Destination (Join-Path $artifacts 'LOCAL-READONLY.EFI') -Force
    & cargo build --release -p loader --target x86_64-unknown-uefi --features tu102-c-compat 2>&1 | Tee-Object -FilePath (Join-Path $artifacts 'build-experimental.log')
    if ($LASTEXITCODE -ne 0) { throw '实验 EFI 编译失败。' }
    Copy-Item -LiteralPath $efi -Destination (Join-Path $artifacts 'LOCAL-EXPERIMENTAL-TX.EFI') -Force
    Get-FileHash -LiteralPath (Join-Path $artifacts 'LOCAL-READONLY.EFI'), (Join-Path $artifacts 'LOCAL-EXPERIMENTAL-TX.EFI') -Algorithm SHA256 |
        Select-Object Hash, Path | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $artifacts 'SHA256.json') -Encoding utf8
} finally {
    Pop-Location
}
