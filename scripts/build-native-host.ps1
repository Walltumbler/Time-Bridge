param([ValidateSet('debug', 'release')][string]$Profile = 'release')
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $PSScriptRoot 'local-env.ps1')
$arguments = @('build', '--manifest-path', (Join-Path $root 'native-host/Cargo.toml'))
if ($Profile -eq 'release') { $arguments += '--release' }
if ($Profile -eq 'debug') {
  $triple = (rustc -vV | Select-String '^host: (.+)$').Matches.Groups[1].Value
  if (-not $triple) { throw 'Could not determine Rust host target triple.' }
  $binDir = Join-Path $root 'src-tauri/binaries'
  New-Item -ItemType Directory -Path $binDir -Force | Out-Null
  $placeholder = Join-Path $binDir "timebridge-host-$triple.exe"
  if (-not (Test-Path -LiteralPath $placeholder)) { Set-Content -LiteralPath $placeholder -Value '' -NoNewline }
}
cargo @arguments
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
$triple = (rustc -vV | Select-String '^host: (.+)$').Matches.Groups[1].Value
if (-not $triple) { throw 'Could not determine Rust host target triple.' }
$source = Join-Path $root "native-host/target/$Profile/timebridge-native-host.exe"
if (-not (Test-Path -LiteralPath $source)) { throw "Native host executable not found at $source" }
$binDir = Join-Path $root 'src-tauri/binaries'
New-Item -ItemType Directory -Path $binDir -Force | Out-Null
Copy-Item -LiteralPath $source -Destination (Join-Path $binDir "timebridge-host-$triple.exe") -Force
if ($Profile -eq 'debug') {
  $debugTarget = Join-Path $root 'native-host/target/debug/timebridge-native-host.exe'
  if (Test-Path -LiteralPath $debugTarget) { Copy-Item -LiteralPath $debugTarget -Destination $placeholder -Force }
}
Write-Host "Prepared Tauri external binary for $triple. The host executable is at $source."
