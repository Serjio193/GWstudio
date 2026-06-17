param(
  [string]$Exe = "$PSScriptRoot\..\gw-studio-tauri\src-tauri\target\release\gw_studio_tauri.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Exe)) {
  throw "GW Studio release exe not found: $Exe"
}

Start-Process -FilePath $Exe -WorkingDirectory (Split-Path -Parent $Exe)

Write-Host "Started GW Studio:"
Write-Host "  $Exe"
