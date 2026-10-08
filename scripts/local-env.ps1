# Optional project-local toolchain used on the development workstation.
# Standard Rust + Visual Studio installations do not need this script.
$root = Split-Path $PSScriptRoot -Parent
$env:CARGO_HOME = Join-Path $root '.tools/cargo'
$env:RUSTUP_HOME = Join-Path $root '.tools/rustup'
$compiler = Join-Path $root '.tools/msvc/Contents/VC/Tools/MSVC/14.44.35207'
$sdk = Join-Path $root '.tools/sdk/c'
$sdkLib = Join-Path $root '.tools/sdk-x64/c'
$env:PATH = "$env:CARGO_HOME\bin;$compiler\bin\Hostx64\x64;$sdk\bin\10.0.26100.0\x64;$env:PATH"
$env:INCLUDE = "$compiler\include;$sdk\Include\10.0.26100.0\ucrt;$sdk\Include\10.0.26100.0\shared;$sdk\Include\10.0.26100.0\um;$sdk\Include\10.0.26100.0\winrt"
$env:LIB = "$compiler\lib\x64;$sdkLib\ucrt\x64;$sdkLib\um\x64"
$env:CC = Join-Path $compiler 'bin/Hostx64/x64/cl.exe'
$env:CXX = $env:CC
$env:RC = Join-Path $sdk 'bin/10.0.26100.0/x64/rc.exe'
