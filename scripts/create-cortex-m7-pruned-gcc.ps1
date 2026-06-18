param(
  [string]$FullGccRoot = "$PSScriptRoot\..\_portable_tools_stage\tools\gcc-arm-none-eabi",
  [string]$OutputRoot = "$PSScriptRoot\..\_portable_tools_stage_pruned\tools\gcc-arm-none-eabi"
)

$ErrorActionPreference = "Stop"

function Copy-TreeIfExists {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  if (Test-Path -LiteralPath $Source) {
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Copy-Item -Path (Join-Path $Source "*") -Destination $Destination -Recurse -Force
  }
}

function Copy-FileIfExists {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  if (Test-Path -LiteralPath $Source) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
  }
}

$full = Resolve-Path -LiteralPath $FullGccRoot
$fullRoot = $full.Path

if (Test-Path -LiteralPath $OutputRoot) {
  Remove-Item -LiteralPath $OutputRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

# Keep all front-end binaries first. They are small compared to multilibs, and this avoids
# fragile failures from gcc driver helper lookups.
Copy-TreeIfExists (Join-Path $fullRoot "bin") (Join-Path $OutputRoot "bin")

# Keep target binutils used directly or indirectly by gcc/collect2.
Copy-TreeIfExists (Join-Path $fullRoot "arm-none-eabi\bin") (Join-Path $OutputRoot "arm-none-eabi\bin")

# Keep target headers and linker scripts.
Copy-TreeIfExists (Join-Path $fullRoot "arm-none-eabi\include") (Join-Path $OutputRoot "arm-none-eabi\include")
Copy-TreeIfExists (Join-Path $fullRoot "arm-none-eabi\lib\ldscripts") (Join-Path $OutputRoot "arm-none-eabi\lib\ldscripts")

# GCC resolves specs and base newlib files from arm-none-eabi\lib before selecting
# the multilib variant. Keep root files, but not root architecture subdirectories.
Get-ChildItem -LiteralPath (Join-Path $fullRoot "arm-none-eabi\lib") -File | ForEach-Object {
  Copy-FileIfExists $_.FullName (Join-Path $OutputRoot "arm-none-eabi\lib\$($_.Name)")
}

# GW Studio builds STM32H7B0 as Cortex-M7 with fpv5-d16 and hard-float.
Copy-TreeIfExists (Join-Path $fullRoot "arm-none-eabi\lib\thumb\v7e-m+dp\hard") (Join-Path $OutputRoot "arm-none-eabi\lib\thumb\v7e-m+dp\hard")

foreach ($name in @("nano.specs", "nosys.specs")) {
  Copy-FileIfExists (Join-Path $fullRoot "arm-none-eabi\lib\$name") (Join-Path $OutputRoot "arm-none-eabi\lib\$name")
  Copy-FileIfExists (Join-Path $fullRoot "arm-none-eabi\lib\thumb\v7e-m+dp\hard\$name") (Join-Path $OutputRoot "arm-none-eabi\lib\thumb\v7e-m+dp\hard\$name")
}

foreach ($required in @(
  "arm-none-eabi\lib\nano.specs",
  "arm-none-eabi\lib\nosys.specs",
  "arm-none-eabi\lib\thumb\v7e-m+dp\hard\nano.specs",
  "arm-none-eabi\lib\thumb\v7e-m+dp\hard\libc_nano.a",
  "arm-none-eabi\lib\thumb\v7e-m+dp\hard\libnosys.a"
)) {
  $requiredPath = Join-Path $OutputRoot $required
  if (-not (Test-Path -LiteralPath $requiredPath)) {
    throw "Pruned GCC missing required file: $requiredPath"
  }
}

$gccVersionRoot = Join-Path $fullRoot "lib\gcc\arm-none-eabi\14.3.1"
$outGccVersionRoot = Join-Path $OutputRoot "lib\gcc\arm-none-eabi\14.3.1"

Copy-TreeIfExists (Join-Path $gccVersionRoot "include") (Join-Path $outGccVersionRoot "include")
Copy-TreeIfExists (Join-Path $gccVersionRoot "include-fixed") (Join-Path $outGccVersionRoot "include-fixed")
Copy-TreeIfExists (Join-Path $gccVersionRoot "install-tools") (Join-Path $outGccVersionRoot "install-tools")
Copy-TreeIfExists (Join-Path $gccVersionRoot "thumb\v7e-m+dp\hard") (Join-Path $outGccVersionRoot "thumb\v7e-m+dp\hard")

foreach ($name in @(
  "cc1.exe",
  "collect2.exe",
  "crtbegin.o",
  "crtend.o",
  "crtfastmath.o",
  "crti.o",
  "crtn.o",
  "libgcc.a",
  "liblto_plugin.dll",
  "sync-cp15dmb.specs",
  "sync-dmb.specs",
  "sync-none.specs"
)) {
  Copy-FileIfExists (Join-Path $gccVersionRoot $name) (Join-Path $outGccVersionRoot $name)
}

$files = Get-ChildItem -LiteralPath $OutputRoot -Recurse -File
$sum = ($files | Measure-Object -Property Length -Sum).Sum

Write-Host "Created Cortex-M7 pruned GCC:"
Write-Host "  $OutputRoot"
Write-Host "  Files:   $($files.Count)"
Write-Host "  Size MB: $([math]::Round($sum / 1MB, 2))"
