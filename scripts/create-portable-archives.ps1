param(
  [Parameter(Mandatory = $true)]
  [string]$ToolsRoot,

  [Parameter(Mandatory = $true)]
  [string]$SourcesRoot,

  [string]$OutputDir = "$PSScriptRoot\..\gw-studio-tauri\src-tauri\portable"
)

$ErrorActionPreference = "Stop"

$toolsRootPath = Resolve-Path -LiteralPath $ToolsRoot
$sourcesRootPath = Resolve-Path -LiteralPath $SourcesRoot
$outputPath = New-Item -ItemType Directory -Force -Path $OutputDir

$toolsZip = Join-Path $outputPath.FullName "tools.zip"
$sourcesZip = Join-Path $outputPath.FullName "sources.zip"

Remove-Item -LiteralPath $toolsZip -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $sourcesZip -Force -ErrorAction SilentlyContinue

$toolsItems = Get-ChildItem -LiteralPath $toolsRootPath -Force
$sourceItems = Get-ChildItem -LiteralPath $sourcesRootPath -Force

if ($toolsItems.Count -eq 0) {
  throw "ToolsRoot is empty: $toolsRootPath"
}
if ($sourceItems.Count -eq 0) {
  throw "SourcesRoot is empty: $sourcesRootPath"
}

Compress-Archive -LiteralPath $toolsItems.FullName -DestinationPath $toolsZip -CompressionLevel Optimal
Compress-Archive -LiteralPath $sourceItems.FullName -DestinationPath $sourcesZip -CompressionLevel Optimal

Write-Host "Created:"
Write-Host "  $toolsZip"
Write-Host "  $sourcesZip"
