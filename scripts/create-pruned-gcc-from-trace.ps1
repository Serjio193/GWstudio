param(
  [Parameter(Mandatory = $true)]
  [string]$UsedFilesCsv,

  [string]$FullToolsRoot = "$PSScriptRoot\..\_portable_tools_stage\tools",
  [string]$OutputRoot = "$PSScriptRoot\..\_portable_tools_pruned\tools"
)

$ErrorActionPreference = "Stop"

function Copy-ParentedFile {
  param(
    [Parameter(Mandatory = $true)][string]$SourcePath,
    [Parameter(Mandatory = $true)][string]$SourceRoot,
    [Parameter(Mandatory = $true)][string]$DestinationRoot
  )

  $sourceItem = Get-Item -LiteralPath $SourcePath -ErrorAction SilentlyContinue
  if (-not $sourceItem -or $sourceItem.PSIsContainer) {
    return
  }

  $sourceRootPath = (Resolve-Path -LiteralPath $SourceRoot).Path
  $relative = $sourceItem.FullName.Substring($sourceRootPath.Length).TrimStart("\", "/")
  $destination = Join-Path $DestinationRoot $relative
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
  Copy-Item -LiteralPath $sourceItem.FullName -Destination $destination -Force
}

if (-not (Test-Path -LiteralPath $UsedFilesCsv)) {
  throw "Used files CSV not found: $UsedFilesCsv"
}

$fullToolsPath = Resolve-Path -LiteralPath $FullToolsRoot
$fullGccRoot = Join-Path $fullToolsPath.Path "gcc-arm-none-eabi"
if (-not (Test-Path -LiteralPath $fullGccRoot)) {
  throw "Full GCC root not found: $fullGccRoot"
}

$outputPath = New-Item -ItemType Directory -Force -Path $OutputRoot
if (Test-Path -LiteralPath $outputPath.FullName) {
  Remove-Item -LiteralPath $outputPath.FullName -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $outputPath.FullName | Out-Null

# Copy non-GCC tools whole. They are smaller and less risky than GCC multilib pruning.
foreach ($name in @("python", "git", "make", "release_licenses")) {
  $src = Join-Path $fullToolsPath.Path $name
  if (Test-Path -LiteralPath $src) {
    Copy-Item -LiteralPath $src -Destination (Join-Path $outputPath.FullName $name) -Recurse
  }
}

$rows = Import-Csv -LiteralPath $UsedFilesCsv
$usedGccFiles = $rows |
  Where-Object { $_.Path -and $_.Path -like "*\gcc-arm-none-eabi\*" } |
  ForEach-Object { $_.Path } |
  Sort-Object -Unique

if (-not $usedGccFiles -or $usedGccFiles.Count -eq 0) {
  throw "Trace contains no gcc-arm-none-eabi file usage. Run a firmware build before exporting the trace."
}

$prunedGccRoot = Join-Path $outputPath.FullName "gcc-arm-none-eabi"
New-Item -ItemType Directory -Force -Path $prunedGccRoot | Out-Null

# Always include critical compiler executables. Trace may miss files that are only needed on later targets.
$alwaysKeep = @(
  "bin\arm-none-eabi-gcc.exe",
  "bin\arm-none-eabi-gcc-14.3.1.exe",
  "bin\arm-none-eabi-as.exe",
  "bin\arm-none-eabi-ar.exe",
  "bin\arm-none-eabi-ranlib.exe",
  "bin\arm-none-eabi-ld.exe",
  "bin\arm-none-eabi-ld.bfd.exe",
  "bin\arm-none-eabi-objcopy.exe",
  "bin\arm-none-eabi-objdump.exe",
  "bin\arm-none-eabi-size.exe",
  "bin\arm-none-eabi-nm.exe",
  "bin\arm-none-eabi-strip.exe",
  "bin\arm-none-eabi-readelf.exe",
  "bin\arm-none-eabi-cpp.exe",
  "bin\arm-none-eabi-g++.exe",
  "bin\arm-none-eabi-c++.exe",
  "bin\arm-none-eabi-gcc-ar.exe",
  "bin\arm-none-eabi-gcc-nm.exe",
  "bin\arm-none-eabi-gcc-ranlib.exe"
)

foreach ($relative in $alwaysKeep) {
  $src = Join-Path $fullGccRoot $relative
  if (Test-Path -LiteralPath $src) {
    Copy-ParentedFile -SourcePath $src -SourceRoot $fullGccRoot -DestinationRoot $prunedGccRoot
  }
}

foreach ($path in $usedGccFiles) {
  if (Test-Path -LiteralPath $path) {
    Copy-ParentedFile -SourcePath $path -SourceRoot $fullGccRoot -DestinationRoot $prunedGccRoot
  }
}

$files = Get-ChildItem -LiteralPath $outputPath.FullName -Recurse -File
$sum = ($files | Measure-Object -Property Length -Sum).Sum

Write-Host "Created pruned tools folder:"
Write-Host "  $($outputPath.FullName)"
Write-Host "  Files:   $($files.Count)"
Write-Host "  Size MB: $([math]::Round($sum / 1MB, 2))"
Write-Host ""
Write-Host "Next:"
Write-Host "  1. Build tools.zip from this folder."
Write-Host "  2. Rebuild GW Studio."
Write-Host "  3. Run the same firmware build matrix."
