$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$project = Join-Path $repoRoot "hardware-monitor-helper\CoworkPal.HardwareMonitor.csproj"
$publishDirectory = Join-Path $repoRoot "hardware-monitor-helper\bin\publish\win-x64"
$binaryDirectory = Join-Path $repoRoot "src-tauri\binaries"
$sourceBinary = Join-Path $publishDirectory "coworkpal-hardware-monitor.exe"
$targetBinary = Join-Path $binaryDirectory "coworkpal-hardware-monitor-x86_64-pc-windows-msvc.exe"
$pawnIoSource = Join-Path $repoRoot "hardware-monitor-helper\vendor\PawnIO_setup.exe"
$pawnIoTarget = Join-Path $binaryDirectory "pawnio-setup-x86_64-pc-windows-msvc.exe"
$pawnIoSha256 = "1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032"

$sha256 = [System.Security.Cryptography.SHA256]::Create()
try {
    $pawnIoActualSha256 = [BitConverter]::ToString(
        $sha256.ComputeHash([System.IO.File]::ReadAllBytes($pawnIoSource))
    ).Replace("-", "")
} finally {
    $sha256.Dispose()
}
if ($pawnIoActualSha256 -ne $pawnIoSha256) {
    throw "PawnIO installer integrity check failed: $pawnIoSource"
}

dotnet publish $project `
    --configuration Release `
    --runtime win-x64 `
    --output $publishDirectory `
    -p:RestoreLockedMode=true
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

New-Item -ItemType Directory -Force -Path $binaryDirectory | Out-Null
Copy-Item -LiteralPath $sourceBinary -Destination $targetBinary -Force
Copy-Item -LiteralPath $pawnIoSource -Destination $pawnIoTarget -Force

$sizeMiB = [Math]::Round((Get-Item -LiteralPath $targetBinary).Length / 1MB, 2)
Write-Host "Built integrated hardware monitor: $targetBinary ($sizeMiB MiB)"
Write-Host "Bundled PawnIO installer: $pawnIoTarget"
