param(
    [string]$IdentityName = $env:TIMEBRIDGE_STORE_IDENTITY_NAME,
    [string]$Publisher = $env:TIMEBRIDGE_STORE_PUBLISHER,
    [string]$PublisherDisplayName = $env:TIMEBRIDGE_STORE_PUBLISHER_DISPLAY_NAME,
    [switch]$DevelopmentIdentity
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$packageJsonPath = Join-Path $repositoryRoot 'package.json'
$manifestTemplatePath = Join-Path $repositoryRoot 'store/msix/AppxManifest.xml.template'
$appExecutablePath = Join-Path $repositoryRoot 'src-tauri/target/release/timebridge.exe'
$hostCandidates = @(Get-ChildItem -LiteralPath (Join-Path $repositoryRoot 'src-tauri/binaries') -Filter 'timebridge-host-*.exe' -File)
$iconPath = Join-Path $repositoryRoot 'src-tauri/icons/128x128.png'
$artifactsDirectory = Join-Path $repositoryRoot 'artifacts'
$layoutDirectory = Join-Path $artifactsDirectory 'msix-layout'

if ($DevelopmentIdentity) {
    if (-not $IdentityName) { $IdentityName = 'Timebridge.Development' }
    if (-not $Publisher) { $Publisher = 'CN=Timebridge Development' }
    if (-not $PublisherDisplayName) { $PublisherDisplayName = 'Timebridge Development' }
}

if (-not $IdentityName -or -not $Publisher -or -not $PublisherDisplayName) {
    throw @'
The Microsoft Store identity is required. Copy the exact values from Partner Center and pass:
  -IdentityName 'Package/Identity/Name'
  -Publisher 'Package/Identity/Publisher'
  -PublisherDisplayName 'Package/Properties/PublisherDisplayName'

For a structural test package only, use -DevelopmentIdentity.
'@
}

if ($IdentityName -notmatch '^[A-Za-z0-9][A-Za-z0-9.-]{2,49}$') {
    throw "IdentityName '$IdentityName' is not a valid package identity name. Copy it exactly from Partner Center."
}

foreach ($requiredPath in @($packageJsonPath, $manifestTemplatePath, $appExecutablePath, $iconPath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Required file is missing: $requiredPath"
    }
}

if ($hostCandidates.Count -ne 1) {
    throw "Expected exactly one src-tauri/binaries/timebridge-host-*.exe, found $($hostCandidates.Count)."
}

$packageJson = Get-Content -LiteralPath $packageJsonPath -Raw | ConvertFrom-Json
$versionParts = @([string]$packageJson.version -split '\.')
if ($versionParts.Count -gt 4 -or $versionParts.Count -lt 1 -or ($versionParts | Where-Object { $_ -notmatch '^\d+$' })) {
    throw "package.json version '$($packageJson.version)' cannot be converted to an MSIX version."
}
while ($versionParts.Count -lt 4) { $versionParts += '0' }
$msixVersion = $versionParts -join '.'
$outputPath = Join-Path $artifactsDirectory "Timebridge_$($packageJson.version)_x64.msix"

New-Item -ItemType Directory -Path $artifactsDirectory -Force | Out-Null
$resolvedArtifacts = (Resolve-Path -LiteralPath $artifactsDirectory).Path.TrimEnd([IO.Path]::DirectorySeparatorChar)
$layoutFullPath = [IO.Path]::GetFullPath($layoutDirectory).TrimEnd([IO.Path]::DirectorySeparatorChar)
if (-not $layoutFullPath.StartsWith($resolvedArtifacts + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clear a layout directory outside artifacts: $layoutFullPath"
}
if (Test-Path -LiteralPath $layoutFullPath) { Remove-Item -LiteralPath $layoutFullPath -Recurse -Force }

$assetsDirectory = Join-Path $layoutFullPath 'Assets'
New-Item -ItemType Directory -Path $assetsDirectory -Force | Out-Null
Copy-Item -LiteralPath $appExecutablePath -Destination (Join-Path $layoutFullPath 'timebridge.exe')
Copy-Item -LiteralPath $hostCandidates[0].FullName -Destination (Join-Path $layoutFullPath 'timebridge-host.exe')

Add-Type -AssemblyName System.Drawing
$sourceImage = [System.Drawing.Image]::FromFile($iconPath)
try {
    foreach ($asset in @(
        @{ Name = 'Square44x44Logo.png'; Size = 44 },
        @{ Name = 'Square150x150Logo.png'; Size = 150 },
        @{ Name = 'StoreLogo.png'; Size = 50 }
    )) {
        $bitmap = New-Object System.Drawing.Bitmap($asset.Size, $asset.Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        try {
            $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
            try {
                $graphics.Clear([System.Drawing.Color]::Transparent)
                $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
                $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
                $graphics.DrawImage($sourceImage, 0, 0, $asset.Size, $asset.Size)
            }
            finally { $graphics.Dispose() }
            $bitmap.Save((Join-Path $assetsDirectory $asset.Name), [System.Drawing.Imaging.ImageFormat]::Png)
        }
        finally { $bitmap.Dispose() }
    }
}
finally { $sourceImage.Dispose() }

$manifest = Get-Content -LiteralPath $manifestTemplatePath -Raw
$manifest = $manifest.Replace('__IDENTITY_NAME__', [System.Security.SecurityElement]::Escape($IdentityName))
$manifest = $manifest.Replace('__PUBLISHER__', [System.Security.SecurityElement]::Escape($Publisher))
$manifest = $manifest.Replace('__PUBLISHER_DISPLAY_NAME__', [System.Security.SecurityElement]::Escape($PublisherDisplayName))
$manifest = $manifest.Replace('__VERSION__', $msixVersion)
$manifestPath = Join-Path $layoutFullPath 'AppxManifest.xml'
[System.IO.File]::WriteAllText($manifestPath, $manifest, (New-Object System.Text.UTF8Encoding($false)))

$makeAppxCommand = Get-Command 'makeappx.exe' -ErrorAction SilentlyContinue
if ($makeAppxCommand) {
    $makeAppxPath = $makeAppxCommand.Source
}
else {
    $bundledCandidates = @(Get-ChildItem -LiteralPath (Join-Path $repositoryRoot '.tools/sdk') -Filter 'makeappx.exe' -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '[\\/]x64[\\/]makeappx\.exe$' } |
        Sort-Object FullName -Descending)
    if ($bundledCandidates.Count -eq 0) {
        throw 'makeappx.exe was not found. Install the Windows SDK or place the existing SDK tools under .tools/sdk.'
    }
    $makeAppxPath = $bundledCandidates[0].FullName
}

if (Test-Path -LiteralPath $outputPath) { Remove-Item -LiteralPath $outputPath -Force }
& $makeAppxPath pack /d $layoutFullPath /p $outputPath /o
if ($LASTEXITCODE -ne 0) { throw "makeappx.exe failed with exit code $LASTEXITCODE." }

$hash = Get-FileHash -LiteralPath $outputPath -Algorithm SHA256
Write-Host ''
Write-Host "Created: $outputPath"
Write-Host "SHA256: $($hash.Hash)"
Write-Warning 'This package is unsigned and intended for Microsoft Store submission or structural testing only. Do not distribute it directly.'
