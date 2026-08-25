$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$project = Join-Path $repoRoot "hardware-monitor-helper\CoworkPal.HardwareMonitor.csproj"
$publishDirectory = Join-Path $repoRoot "hardware-monitor-helper\bin\publish\win-x64"
$binaryDirectory = Join-Path $repoRoot "src-tauri\binaries"
$sourceBinary = Join-Path $publishDirectory "coworkpal-hardware-monitor.exe"
$targetBinary = Join-Path $binaryDirectory "coworkpal-hardware-monitor-x86_64-pc-windows-msvc.exe"

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

$sizeMiB = [Math]::Round((Get-Item -LiteralPath $targetBinary).Length / 1MB, 2)
Write-Host "Built integrated hardware monitor: $targetBinary ($sizeMiB MiB)"
