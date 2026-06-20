param(
  [string]$TraceDir = "$PSScriptRoot\..\_file_trace",
  [string]$Procmon = "$PSScriptRoot\..\_audit_tools\procmon\Procmon64.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Procmon)) {
  throw "ProcMon not found: $Procmon"
}

$tracePath = Resolve-Path -LiteralPath $TraceDir
$pml = Join-Path $tracePath.Path "gwstudio_trace.pml"
$csv = Join-Path $tracePath.Path "gwstudio_trace.csv"
$usedCsv = Join-Path $tracePath.Path "gwstudio_used_files.csv"
$summaryCsv = Join-Path $tracePath.Path "gwstudio_used_summary.csv"

& $Procmon /Terminate | Out-Null
Start-Sleep -Seconds 2

if (-not (Test-Path -LiteralPath $pml)) {
  throw "Trace file was not created: $pml"
}

Remove-Item -LiteralPath $csv,$usedCsv,$summaryCsv -Force -ErrorAction SilentlyContinue

& $Procmon /AcceptEula /Quiet /OpenLog $pml /SaveAs $csv

if (-not (Test-Path -LiteralPath $csv)) {
  throw "CSV export failed: $csv"
}

$interestingProcesses = @(
  "GWStudio.exe",
  "gw_studio_tauri.exe",
  "bash.exe",
  "sh.exe",
  "make.exe",
  "mingw32-make.exe",
  "python.exe"
)

$rows = Import-Csv -LiteralPath $csv
$used = $rows | Where-Object {
  $process = $_."Process Name"
  $path = $_.Path
  if (-not $process -or -not $path) { return $false }
  $interestingProcesses -contains $process -and (
    $path -like "*\GWStudioRuntime\*" -or
    $path -like "*\_portable_tools_stage\tools\*" -or
    $path -like "*\GameWatchBuilderData\*"
  )
}

$used |
  Select-Object "Time of Day","Process Name","Operation","Path","Result" |
  Export-Csv -LiteralPath $usedCsv -NoTypeInformation -Encoding UTF8

$used |
  Group-Object "Process Name", Operation |
  ForEach-Object {
    [pscustomobject]@{
      ProcessOperation = $_.Name
      Count = $_.Count
    }
  } |
  Sort-Object Count -Descending |
  Export-Csv -LiteralPath $summaryCsv -NoTypeInformation -Encoding UTF8

Write-Host "ProcMon trace stopped and exported:"
Write-Host "  Full CSV:    $csv"
Write-Host "  Used files:  $usedCsv"
Write-Host "  Summary:     $summaryCsv"
