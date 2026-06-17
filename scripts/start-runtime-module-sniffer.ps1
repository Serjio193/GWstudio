param(
  [string]$OutputDir = "$PSScriptRoot\..\_runtime_module_trace",
  [int]$IntervalMs = 1000
)

$ErrorActionPreference = "Stop"

$out = New-Item -ItemType Directory -Force -Path $OutputDir
$statePath = Join-Path $out.FullName "sniffer_state.json"
$scriptPath = Join-Path $out.FullName "sniffer_worker.ps1"
$modulesCsv = Join-Path $out.FullName "runtime_modules.csv"
$processCsv = Join-Path $out.FullName "runtime_processes.csv"
$logPath = Join-Path $out.FullName "sniffer.log"

Remove-Item -LiteralPath $modulesCsv,$processCsv,$logPath,$statePath,$scriptPath -Force -ErrorAction SilentlyContinue

$worker = @'
param(
  [string]$OutputDir,
  [int]$IntervalMs
)

$ErrorActionPreference = "SilentlyContinue"
$modulesCsv = Join-Path $OutputDir "runtime_modules.csv"
$processCsv = Join-Path $OutputDir "runtime_processes.csv"
$logPath = Join-Path $OutputDir "sniffer.log"
$stopPath = Join-Path $OutputDir "stop.flag"

$interesting = @(
  "GW-Studio-Portable",
  "gw_studio_tauri",
  "python",
  "bash",
  "sh",
  "make",
  "mingw32-make",
  "arm-none-eabi-gcc",
  "arm-none-eabi-as",
  "arm-none-eabi-ld",
  "arm-none-eabi-objcopy",
  "arm-none-eabi-objdump"
)

$seenModules = New-Object 'System.Collections.Generic.HashSet[string]'
$seenProcesses = New-Object 'System.Collections.Generic.HashSet[string]'

"Timestamp,Pid,ProcessName,Path,CommandLine" | Set-Content -LiteralPath $processCsv -Encoding UTF8
"Timestamp,Pid,ProcessName,ModuleName,ModulePath" | Set-Content -LiteralPath $modulesCsv -Encoding UTF8
"Runtime module sniffer started $(Get-Date -Format o)" | Set-Content -LiteralPath $logPath -Encoding UTF8

function CsvEscape([string]$value) {
  if ($null -eq $value) { return '""' }
  return '"' + $value.Replace('"', '""') + '"'
}

while (-not (Test-Path -LiteralPath $stopPath)) {
  $timestamp = Get-Date -Format o
  $procs = Get-Process | Where-Object { $interesting -contains $_.ProcessName }
  foreach ($proc in $procs) {
    $procId = $proc.Id
    $processKey = "$procId|$($proc.ProcessName)"
    if ($seenProcesses.Add($processKey)) {
      $cmd = ""
      try {
        $cmd = (Get-CimInstance Win32_Process -Filter "ProcessId=$procId").CommandLine
      } catch {}
      $line = @(
        (CsvEscape $timestamp),
        $procId,
        (CsvEscape $proc.ProcessName),
        (CsvEscape $proc.Path),
        (CsvEscape $cmd)
      ) -join ","
      Add-Content -LiteralPath $processCsv -Value $line -Encoding UTF8
    }

    try {
      foreach ($module in $proc.Modules) {
        $modulePath = $module.FileName
        if (-not $modulePath) { continue }
        $moduleKey = "$procId|$modulePath"
        if (-not $seenModules.Add($moduleKey)) { continue }
        $line = @(
          (CsvEscape $timestamp),
          $procId,
          (CsvEscape $proc.ProcessName),
          (CsvEscape $module.ModuleName),
          (CsvEscape $modulePath)
        ) -join ","
        Add-Content -LiteralPath $modulesCsv -Value $line -Encoding UTF8
      }
    } catch {}
  }
  Start-Sleep -Milliseconds $IntervalMs
}

"Runtime module sniffer stopped $(Get-Date -Format o)" | Add-Content -LiteralPath $logPath -Encoding UTF8
'@

Set-Content -LiteralPath $scriptPath -Value $worker -Encoding UTF8

$arguments = @(
  "-NoProfile",
  "-ExecutionPolicy", "Bypass",
  "-WindowStyle", "Hidden",
  "-File", ('"{0}"' -f $scriptPath),
  "-OutputDir", ('"{0}"' -f $out.FullName),
  "-IntervalMs", $IntervalMs
) -join " "

$process = Start-Process -FilePath "powershell.exe" -ArgumentList $arguments -WindowStyle Hidden -PassThru

@{
  pid = $process.Id
  output_dir = $out.FullName
  modules_csv = $modulesCsv
  process_csv = $processCsv
  started_at = (Get-Date -Format o)
} | ConvertTo-Json | Set-Content -LiteralPath $statePath -Encoding UTF8

Write-Host "Runtime module sniffer started:"
Write-Host "  PID:        $($process.Id)"
Write-Host "  Output dir: $($out.FullName)"
Write-Host "Stop with:"
Write-Host "  .\scripts\stop-runtime-module-sniffer.ps1"
