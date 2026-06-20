param(
  [string]$Exe = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Exe)) {
  $releaseDir = Resolve-Path -LiteralPath "$PSScriptRoot\..\gw-studio-tauri\src-tauri\target\release" -ErrorAction SilentlyContinue
  if ($releaseDir) {
    $gwStudioExe = Join-Path $releaseDir "GWStudio.exe"
    $cargoExe = Join-Path $releaseDir "gw_studio_tauri.exe"
    if (Test-Path -LiteralPath $gwStudioExe) {
      $Exe = $gwStudioExe
    } else {
      $Exe = $cargoExe
    }
  }
}

if ([string]::IsNullOrWhiteSpace($Exe)) {
  throw "GW Studio release directory not found"
}

if (-not (Test-Path -LiteralPath $Exe)) {
  throw "GW Studio release exe not found: $Exe"
}

Start-Process -FilePath $Exe -WorkingDirectory (Split-Path -Parent $Exe)

Write-Host "Started GW Studio:"
Write-Host "  $Exe"
