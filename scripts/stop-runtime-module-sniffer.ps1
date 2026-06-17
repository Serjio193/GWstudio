param(
  [string]$OutputDir = "$PSScriptRoot\..\_runtime_module_trace"
)

$ErrorActionPreference = "Stop"

$out = Resolve-Path -LiteralPath $OutputDir
$stopPath = Join-Path $out.Path "stop.flag"
$statePath = Join-Path $out.Path "sniffer_state.json"
$modulesCsv = Join-Path $out.Path "runtime_modules.csv"
$processCsv = Join-Path $out.Path "runtime_processes.csv"
$summaryCsv = Join-Path $out.Path "runtime_module_summary.csv"
$toolsSummaryCsv = Join-Path $out.Path "runtime_tools_summary.csv"

Set-Content -LiteralPath $stopPath -Value "stop" -Encoding UTF8

if (Test-Path -LiteralPath $statePath) {
  $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
  if ($state.pid) {
    Wait-Process -Id ([int]$state.pid) -Timeout 10 -ErrorAction SilentlyContinue
  }
}

$modules = if (Test-Path -LiteralPath $modulesCsv) { Import-Csv -LiteralPath $modulesCsv } else { @() }

$modules |
  Group-Object ProcessName, ModuleName |
  ForEach-Object {
    $first = $_.Group | Select-Object -First 1
    [pscustomobject]@{
      ProcessModule = $_.Name
      Count = $_.Count
      ExamplePath = $first.ModulePath
    }
  } |
  Sort-Object ProcessModule |
  Export-Csv -LiteralPath $summaryCsv -NoTypeInformation -Encoding UTF8

$modules |
  Where-Object { $_.ModulePath -like "*\GWStudioRuntime\*" -or $_.ModulePath -like "*\tools\*" } |
  Group-Object ModulePath |
  ForEach-Object {
    [pscustomobject]@{
      ModulePath = $_.Name
      Count = $_.Count
      Processes = (($_.Group | Select-Object -ExpandProperty ProcessName -Unique) -join ";")
    }
  } |
  Sort-Object ModulePath |
  Export-Csv -LiteralPath $toolsSummaryCsv -NoTypeInformation -Encoding UTF8

Write-Host "Runtime module sniffer stopped:"
Write-Host "  Processes:     $processCsv"
Write-Host "  Modules:       $modulesCsv"
Write-Host "  Summary:       $summaryCsv"
Write-Host "  Tools summary: $toolsSummaryCsv"
