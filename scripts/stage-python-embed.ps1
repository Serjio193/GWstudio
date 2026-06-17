param(
  [string]$OutputRoot = "$PSScriptRoot\..\_portable_tools_stage\tools",
  [string]$PythonVersion = "3.13.2"
)

$ErrorActionPreference = "Stop"

$outputPath = New-Item -ItemType Directory -Force -Path $OutputRoot
$downloadDir = New-Item -ItemType Directory -Force -Path "$PSScriptRoot\..\_portable_tools_stage\downloads"
$pythonDir = Join-Path $outputPath.FullName "python"
$zipPath = Join-Path $downloadDir.FullName "python-$PythonVersion-embed-amd64.zip"
$getPipPath = Join-Path $downloadDir.FullName "get-pip.py"

if (Test-Path -LiteralPath $pythonDir) {
  Remove-Item -LiteralPath $pythonDir -Recurse -Force
}

if (-not (Test-Path -LiteralPath $zipPath)) {
  $pythonUrl = "https://www.python.org/ftp/python/$PythonVersion/python-$PythonVersion-embed-amd64.zip"
  Invoke-WebRequest -Uri $pythonUrl -OutFile $zipPath
}

Expand-Archive -LiteralPath $zipPath -DestinationPath $pythonDir -Force

$pth = Join-Path $pythonDir "python313._pth"
if (Test-Path -LiteralPath $pth) {
  $pthText = Get-Content -LiteralPath $pth
  $pthText = $pthText | Where-Object { $_ -ne "#import site" -and $_ -ne "import site" -and $_ -ne "Lib\site-packages" }
  $pthText += "Lib\site-packages"
  # Do not import site in the final runtime: this prevents accidental use of the developer user's site-packages.
  Set-Content -LiteralPath $pth -Value $pthText -Encoding ASCII
}

if (-not (Test-Path -LiteralPath $getPipPath)) {
  Invoke-WebRequest -Uri "https://bootstrap.pypa.io/get-pip.py" -OutFile $getPipPath
}

$env:PYTHONNOUSERSITE = "1"
& (Join-Path $pythonDir "python.exe") $getPipPath --no-warn-script-location

& (Join-Path $pythonDir "python.exe") -m pip install --no-warn-script-location --ignore-installed `
  "gnwmanager==0.21.1" `
  "pyocd==0.44.1" `
  "tqdm" `
  "pyserial" `
  "pyyaml" `
  "pillow"

# GW Studio does not use gnwmanager's bundled unlock/patch payloads.
# Keep the Python modules and `firmware.bin`, which is required by `GnW.start_gnwmanager()`
# for SPI operations. Do not ship unlock or stock patch payloads that GW Studio does not call.
$gnwmanagerRoot = Join-Path $pythonDir "Lib\site-packages\gnwmanager"
$gnwPatchBinaries = Join-Path $gnwmanagerRoot "cli\gnw_patch\binaries"
if (Test-Path -LiteralPath $gnwPatchBinaries) {
  Remove-Item -LiteralPath $gnwPatchBinaries -Recurse -Force
}
$unlockPayload = Join-Path $gnwmanagerRoot "unlock.bin"
if (Test-Path -LiteralPath $unlockPayload) {
  Remove-Item -LiteralPath $unlockPayload -Force
}

& (Join-Path $pythonDir "python.exe") -m pip freeze |
  Set-Content -LiteralPath "$PSScriptRoot\..\licenses\python-pip-freeze.txt" -Encoding UTF8

$files = Get-ChildItem -LiteralPath $pythonDir -Recurse -File
$size = ($files | Measure-Object -Property Length -Sum).Sum

Write-Host "Staged Python embed:"
Write-Host "  Destination: $pythonDir"
Write-Host "  Files:       $($files.Count)"
Write-Host "  Size MB:     $([math]::Round($size / 1MB, 2))"
Write-Host ""
Write-Host "Smoke test:"
& (Join-Path $pythonDir "python.exe") -c "import sys; assert not any('AppData\\Roaming\\Python' in p for p in sys.path), sys.path; import gnwmanager, pyocd, serial, yaml, PIL; print('python runtime ok')"
