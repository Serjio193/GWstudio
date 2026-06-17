param(
  [string]$ToolsRoot = "$PSScriptRoot\..\_portable_tools_stage\tools",
  [string]$OutputZip = "$PSScriptRoot\..\gw-studio-tauri\src-tauri\portable\tools.zip"
)

$ErrorActionPreference = "Stop"

$toolsRootPath = Resolve-Path -LiteralPath $ToolsRoot
$outputDir = New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputZip)

$items = Get-ChildItem -LiteralPath $toolsRootPath -Force
if ($items.Count -eq 0) {
  throw "ToolsRoot is empty: $toolsRootPath"
}

Remove-Item -LiteralPath $OutputZip -Force -ErrorAction SilentlyContinue

Compress-Archive -LiteralPath $items.FullName -DestinationPath $OutputZip -CompressionLevel Optimal

$zip = Get-Item -LiteralPath $OutputZip
Write-Host "Created tools archive:"
Write-Host "  $($zip.FullName)"
Write-Host "  Size MB: $([math]::Round($zip.Length / 1MB, 2))"
