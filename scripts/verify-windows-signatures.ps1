param(
  [Parameter(Mandatory=$true)][string[]]$Path,
  [string]$Thumbprint = $env:TIMEBRIDGE_WINDOWS_CERTIFICATE_THUMBPRINT
)
$ErrorActionPreference = 'Stop'
$files = foreach ($item in $Path) {
  if (Test-Path -LiteralPath $item -PathType Container) { Get-ChildItem -LiteralPath $item -Recurse -File | Where-Object Extension -In '.exe','.msi' }
  elseif (Test-Path -LiteralPath $item -PathType Leaf) { Get-Item -LiteralPath $item }
  else { throw "Signature target does not exist: $item" }
}
if (-not $files) { throw 'No Windows executables or installers were found to verify.' }
foreach ($file in $files | Sort-Object FullName -Unique) {
  $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
  if ($signature.Status -ne 'Valid') { throw "$($file.FullName) has invalid Authenticode status: $($signature.Status)." }
  if ($Thumbprint -and $signature.SignerCertificate.Thumbprint -ne $Thumbprint) { throw "$($file.FullName) was signed by an unexpected certificate." }
  Write-Host "Verified $($file.Name): $($signature.SignerCertificate.Subject)"
}
