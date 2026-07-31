$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$configPath = Join-Path $repoRoot "src-tauri\tauri.windows.conf.json"

if ([string]::IsNullOrWhiteSpace($env:WINDOWS_CERTIFICATE) -or [string]::IsNullOrWhiteSpace($env:WINDOWS_CERTIFICATE_PASSWORD)) {
    Write-Warning "WINDOWS_CERTIFICATE or WINDOWS_CERTIFICATE_PASSWORD is not configured. Windows bundles will be built unsigned."
    return
}

$timestampUrl = $env:WINDOWS_SIGNING_TIMESTAMP_URL
if ([string]::IsNullOrWhiteSpace($timestampUrl)) {
    $timestampUrl = "http://timestamp.digicert.com"
}

$tempRoot = $env:RUNNER_TEMP
if ([string]::IsNullOrWhiteSpace($tempRoot)) {
    $tempRoot = [System.IO.Path]::GetTempPath()
}

$certificateDir = Join-Path $tempRoot "coworkpal-windows-signing"
New-Item -ItemType Directory -Path $certificateDir -Force | Out-Null

$encodedPath = Join-Path $certificateDir "certificate.base64"
$pfxPath = Join-Path $certificateDir "certificate.pfx"

Set-Content -Path $encodedPath -Value $env:WINDOWS_CERTIFICATE -NoNewline
certutil -decode $encodedPath $pfxPath | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "Failed to decode WINDOWS_CERTIFICATE. Ensure the secret contains a base64 encoded .pfx certificate."
}
Remove-Item -LiteralPath $encodedPath -Force

$securePassword = ConvertTo-SecureString -String $env:WINDOWS_CERTIFICATE_PASSWORD -Force -AsPlainText
$importedCertificates = Import-PfxCertificate `
    -FilePath $pfxPath `
    -CertStoreLocation Cert:\CurrentUser\My `
    -Password $securePassword

Remove-Item -LiteralPath $pfxPath -Force

$signingCertificate = $importedCertificates | Where-Object { $_.HasPrivateKey } | Select-Object -First 1
if (-not $signingCertificate) {
    throw "The imported certificate does not contain a private key and cannot sign Windows binaries."
}

$thumbprint = ($signingCertificate.Thumbprint -replace "\s", "").ToUpperInvariant()
if ([string]::IsNullOrWhiteSpace($thumbprint)) {
    throw "Unable to read the imported Windows signing certificate thumbprint."
}

$signingConfig = @{
    bundle = @{
        windows = @{
            certificateThumbprint = $thumbprint
            digestAlgorithm       = "sha256"
            timestampUrl          = $timestampUrl
        }
    }
}

$signingConfig | ConvertTo-Json -Depth 10 | Set-Content -Path $configPath -Encoding utf8

Write-Host "Windows signing certificate imported."
Write-Host "Generated Tauri Windows signing config at $configPath."
Write-Host "Certificate thumbprint: $thumbprint"
