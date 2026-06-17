param(
  [string]$TraceDir = "$PSScriptRoot\..\_file_trace",
  [string]$Procmon = "$PSScriptRoot\..\_audit_tools\procmon\Procmon64.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Procmon)) {
  throw "ProcMon not found: $Procmon"
}

$tracePath = New-Item -ItemType Directory -Force -Path $TraceDir
$pml = Join-Path $tracePath.FullName "gwstudio_trace.pml"

Remove-Item -LiteralPath $pml -Force -ErrorAction SilentlyContinue

& $Procmon /AcceptEula /Quiet /Minimized /BackingFile $pml

Write-Host "ProcMon trace started:"
Write-Host "  $pml"
Write-Host ""
Write-Host "Now run GW Studio and complete the test workflow."
Write-Host "When finished, run:"
Write-Host "  .\scripts\stop-file-trace.ps1"
