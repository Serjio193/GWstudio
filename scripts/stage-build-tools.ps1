param(
  [string]$OutputRoot = "$PSScriptRoot\..\_portable_tools_stage\tools",
  [string]$GitRoot = "C:\Program Files\Git"
)

$ErrorActionPreference = "Stop"

function Copy-CleanDirectory {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  if (-not (Test-Path -LiteralPath $Source)) {
    throw "Source folder not found: $Source"
  }
  if (Test-Path -LiteralPath $Destination) {
    Remove-Item -LiteralPath $Destination -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
  Copy-Item -LiteralPath $Source -Destination $Destination -Recurse
}

function Copy-MsysRuntimeFromGit {
  param(
    [Parameter(Mandatory = $true)][string]$GitRoot,
    [Parameter(Mandatory = $true)][string]$Destination
  )

  $usrBin = Join-Path $GitRoot "usr\bin"
  $etc = Join-Path $GitRoot "etc"
  if (-not (Test-Path -LiteralPath (Join-Path $usrBin "bash.exe"))) {
    throw "Git for Windows MSYS bash not found: $usrBin"
  }
  if (Test-Path -LiteralPath $Destination) {
    Remove-Item -LiteralPath $Destination -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path (Join-Path $Destination "bin") | Out-Null
  Copy-Item -Path (Join-Path $usrBin "*") -Destination (Join-Path $Destination "bin") -Recurse -Force
  if (Test-Path -LiteralPath $etc) {
    Copy-Item -LiteralPath $etc -Destination (Join-Path $Destination "etc") -Recurse -Force
  }
}

$outputPath = New-Item -ItemType Directory -Force -Path $OutputRoot

$gccPlugin = Get-ChildItem -LiteralPath "C:\ST\STM32CubeIDE_1.18.1\STM32CubeIDE\plugins" -Directory |
  Where-Object { $_.Name -like "com.st.stm32cube.ide.mcu.externaltools.gnu-tools-for-stm32.14.3.rel1.win32*" } |
  Select-Object -First 1
if (-not $gccPlugin) {
  throw "GNU Arm toolchain plugin not found"
}
$gccTools = Join-Path $gccPlugin.FullName "tools"

$makePlugin = Get-ChildItem -LiteralPath "C:\ST\STM32CubeIDE_1.18.1\STM32CubeIDE\plugins" -Directory |
  Where-Object { $_.Name -like "com.st.stm32cube.ide.mcu.externaltools.make.win32_2.2.100*" } |
  Select-Object -First 1
if (-not $makePlugin) {
  throw "GNU Make plugin not found"
}
$makeTools = Join-Path $makePlugin.FullName "tools"

Copy-MsysRuntimeFromGit -GitRoot $GitRoot -Destination (Join-Path $outputPath.FullName "git")
Copy-CleanDirectory -Source $makeTools -Destination (Join-Path $outputPath.FullName "make")
Copy-CleanDirectory -Source $gccTools -Destination (Join-Path $outputPath.FullName "gcc-arm-none-eabi")
New-Item -ItemType Directory -Force -Path (Join-Path $outputPath.FullName "tmp") | Out-Null

$makeShare = Join-Path $outputPath.FullName "make\share"
if (Test-Path -LiteralPath $makeShare) {
  Remove-Item -LiteralPath $makeShare -Recurse -Force
}

# Keep only the Cortex-M7 hard-float GCC multilib used by GW Studio firmware builds.
$gccStage = Join-Path $outputPath.FullName "gcc-arm-none-eabi"
$gccPruned = Join-Path $outputPath.FullName "gcc-arm-none-eabi-pruned"
& (Join-Path $PSScriptRoot "create-cortex-m7-pruned-gcc.ps1") -FullGccRoot $gccStage -OutputRoot $gccPruned
Remove-Item -LiteralPath $gccStage -Recurse -Force
Rename-Item -LiteralPath $gccPruned -NewName "gcc-arm-none-eabi"

# pip is not needed at runtime; all required Python packages are pre-bundled.
# Keep numpy: game-and-watch-patch uses it while building Bank1 dualboot images.
$pythonSitePackages = Join-Path $outputPath.FullName "python\Lib\site-packages"
foreach ($name in @(
  "pip",
  "pip-26.1.2.dist-info"
)) {
  $candidate = Join-Path $pythonSitePackages $name
  if (Test-Path -LiteralPath $candidate) {
    Remove-Item -LiteralPath $candidate -Recurse -Force
  }
}

# Runtime does not need Python bytecode caches or package test suites.
Get-ChildItem -LiteralPath $outputPath.FullName -Recurse -Directory -Filter "__pycache__" -ErrorAction SilentlyContinue |
  Remove-Item -Recurse -Force
foreach ($testDirName in @("tests", "test", "testing")) {
  Get-ChildItem -LiteralPath $outputPath.FullName -Recurse -Directory -Filter $testDirName -ErrorAction SilentlyContinue |
    Remove-Item -Recurse -Force
}

# Remove interactive MSYS/Git tools not used by GW Studio build/flash workflows.
$gitBin = Join-Path $outputPath.FullName "git\bin"
foreach ($name in @(
  "cygcheck.exe",
  "ex.exe",
  "gpg-agent.exe",
  "gpg-card.exe",
  "gpg-connect-agent.exe",
  "gpg-wks-client.exe",
  "gpg-wks-server.exe",
  "gpg.exe",
  "gpgconf.exe",
  "gpgparsemail.exe",
  "gpgscm.exe",
  "gpgsm.exe",
  "gpgsplit.exe",
  "gpgtar.exe",
  "gpgv.exe",
  "mintty.exe",
  "rebase.exe",
  "rview.exe",
  "rvim.exe",
  "scp.exe",
  "sftp.exe",
  "ssh-add.exe",
  "ssh-agent.exe",
  "ssh-keygen.exe",
  "ssh-keyscan.exe",
  "ssh.exe",
  "tig.exe",
  "view.exe",
  "vim.exe",
  "vimdiff.exe"
)) {
  $candidate = Join-Path $gitBin $name
  if (Test-Path -LiteralPath $candidate) {
    Remove-Item -LiteralPath $candidate -Force
  }
}

# Keep only shell utilities needed by Makefiles and source preparation scripts.
# The DLL list is derived from ldd for the kept executables, so unused SVN/SSL/Perl stacks are removed.
$keepGitExe = @(
  "bash.exe",
  "basename.exe",
  "cat.exe",
  "chmod.exe",
  "cmp.exe",
  "cp.exe",
  "cut.exe",
  "date.exe",
  "dd.exe",
  "diff.exe",
  "dirname.exe",
  "echo.exe",
  "env.exe",
  "expr.exe",
  "false.exe",
  "find.exe",
  "gawk.exe",
  "grep.exe",
  "gzip.exe",
  "head.exe",
  "install.exe",
  "ln.exe",
  "ls.exe",
  "mkdir.exe",
  "mv.exe",
  "patch.exe",
  "printf.exe",
  "pwd.exe",
  "readlink.exe",
  "realpath.exe",
  "rm.exe",
  "sed.exe",
  "sh.exe",
  "sleep.exe",
  "sort.exe",
  "stat.exe",
  "tail.exe",
  "tar.exe",
  "tee.exe",
  "test.exe",
  "touch.exe",
  "tr.exe",
  "true.exe",
  "uname.exe",
  "wc.exe",
  "which.exe",
  "xargs.exe"
)
$keepGitSet = @{}
foreach ($name in $keepGitExe) {
  $candidate = Join-Path $gitBin $name
  if (Test-Path -LiteralPath $candidate) {
    $keepGitSet[$name.ToLowerInvariant()] = $true
  }
}
$lddExe = Join-Path $gitBin "ldd.exe"
$keepGitDllSet = @{}
if (Test-Path -LiteralPath $lddExe) {
  foreach ($name in $keepGitSet.Keys) {
    $out = & $lddExe (Join-Path $gitBin $name) 2>$null
    foreach ($line in $out) {
      if ($line -match '=>\s+/(?:git|usr)/bin/([^\s]+)') {
        $keepGitDllSet[$matches[1].ToLowerInvariant()] = $true
      } elseif ($line -match '^\s*/(?:git|usr)/bin/([^\s]+)') {
        $keepGitDllSet[$matches[1].ToLowerInvariant()] = $true
      }
    }
  }
}
$keepGitDllSet["msys-2.0.dll"] = $true

Get-ChildItem -LiteralPath $gitBin -File -ErrorAction SilentlyContinue |
  Where-Object { @(".exe", ".pl", ".sh") -contains $_.Extension.ToLowerInvariant() } |
  Where-Object { -not $keepGitSet.ContainsKey($_.Name.ToLowerInvariant()) } |
  Remove-Item -Force
Get-ChildItem -LiteralPath $gitBin -File -Filter "*.dll" -ErrorAction SilentlyContinue |
  Where-Object { -not $keepGitDllSet.ContainsKey($_.Name.ToLowerInvariant()) } |
  Remove-Item -Force
Get-ChildItem -LiteralPath $gitBin -File -ErrorAction SilentlyContinue |
  Where-Object { $_.Extension -eq "" } |
  Remove-Item -Force

# Keep C compiler, assembler, linker, objcopy and ar. Drop C++/diagnostic tools.
$gccBin = Join-Path $outputPath.FullName "gcc-arm-none-eabi\bin"
foreach ($name in @(
  "arm-none-eabi-addr2line.exe",
  "arm-none-eabi-c++filt.exe",
  "arm-none-eabi-c++.exe",
  "arm-none-eabi-cpp.exe",
  "arm-none-eabi-elfedit.exe",
  "arm-none-eabi-g++.exe",
  "arm-none-eabi-gcc-ar.exe",
  "arm-none-eabi-gcc-nm.exe",
  "arm-none-eabi-gcc-ranlib.exe",
  "arm-none-eabi-gcov-dump.exe",
  "arm-none-eabi-gcov-tool.exe",
  "arm-none-eabi-gcov.exe",
  "arm-none-eabi-gdb-add-index.exe",
  "arm-none-eabi-gdb.exe",
  "arm-none-eabi-gprof.exe",
  "arm-none-eabi-lto-dump.exe",
  "arm-none-eabi-nm.exe",
  "arm-none-eabi-objdump.exe",
  "arm-none-eabi-ranlib.exe",
  "arm-none-eabi-readelf.exe",
  "arm-none-eabi-strings.exe",
  "arm-none-eabi-strip.exe"
)) {
  $candidate = Join-Path $gccBin $name
  if (Test-Path -LiteralPath $candidate) {
    Remove-Item -LiteralPath $candidate -Force
  }
}

# GW Studio builds C firmware only. Remove C++ headers/libs after dropping g++.
$gccRoot = Join-Path $outputPath.FullName "gcc-arm-none-eabi"
$gccCxxInclude = Join-Path $gccRoot "arm-none-eabi\include\c++"
if (Test-Path -LiteralPath $gccCxxInclude) {
  Remove-Item -LiteralPath $gccCxxInclude -Recurse -Force
}
Get-ChildItem -LiteralPath (Join-Path $gccRoot "arm-none-eabi\lib") -Recurse -File -ErrorAction SilentlyContinue |
  Where-Object { $_.Name -like "libstdc++*.a" -or $_.Name -like "libsupc++*.a" } |
  Remove-Item -Force

$checks = @(
  (Join-Path $outputPath.FullName "git\bin\bash.exe"),
  (Join-Path $outputPath.FullName "git\bin\sh.exe"),
  (Join-Path $outputPath.FullName "make\bin\make.exe"),
  (Join-Path $outputPath.FullName "gcc-arm-none-eabi\bin\arm-none-eabi-gcc.exe")
)
foreach ($check in $checks) {
  if (-not (Test-Path -LiteralPath $check)) {
    throw "Staged tool check failed: $check"
  }
}

$files = Get-ChildItem -LiteralPath $outputPath.FullName -Recurse -File
$size = ($files | Measure-Object -Property Length -Sum).Sum

Write-Host "Staged build tools:"
Write-Host "  Destination: $($outputPath.FullName)"
Write-Host "  Files:       $($files.Count)"
Write-Host "  Size MB:     $([math]::Round($size / 1MB, 2))"
Write-Host ""
Write-Host "Reminder: verify license files for the exact Git/Make/ARM GCC package versions before final release."
