param(
  [switch]$IncludeIgnored
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Get-GitFiles {
  $tracked = git ls-files
  if ($LASTEXITCODE -ne 0) {
    throw "git ls-files failed"
  }

  $untrackedArgs = @("ls-files", "--others")
  if (-not $IncludeIgnored) {
    $untrackedArgs += "--exclude-standard"
  }
  $untracked = git @untrackedArgs
  if ($LASTEXITCODE -ne 0) {
    throw "git ls-files --others failed"
  }

  @($tracked + $untracked) |
    Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) } |
    Sort-Object -Unique
}

function Normalize-PathText([string]$PathText) {
  $PathText.Replace("\", "/")
}

$allowedExact = @(
  "README_LICENSE_BLOCK.md",
  "RELEASE_AUDIT.md",
  "RELEASE_CHECKLIST.md",
  "RELEASE_SIGNING.md",
  "THIRD_PARTY_NOTICES.md",
  "THIRD_PARTY_TOOLS.md",
  "LICENSE_MANIFEST.md",
  "gw-studio-tauri/src-tauri/Cargo.lock",
  "gw-studio-tauri/package-lock.json"
)

$allowedPrefixes = @(
  "licenses/",
  "third_party_licenses/",
  "release_keys/"
)

$forbiddenExtensions = @(
  ".nes", ".sfc", ".smc", ".gb", ".gbc", ".gba", ".gg", ".sms", ".pce", ".col", ".msx",
  ".rom", ".sav", ".bin", ".elf", ".hex", ".uf2", ".map", ".sig", ".secret", ".key"
)

$forbiddenPathPatterns = @(
  "/secrets/",
  "/backups/",
  "/GameWatchBuilderData/",
  "/GWStudioRuntime/",
  "/build_workspaces/",
  "/StockFirmware/",
  "/stock_firmware/",
  "/firmware/",
  "/coleco_bios/",
  "/msx_bios/"
)

$forbiddenNamePatterns = @(
  "internal_flash",
  "external_flash",
  "flash_backup",
  "bank1",
  "bank2",
  "spi_",
  "stock",
  "backup",
  "dualboot",
  "firmware",
  "bios",
  "private"
)

$violations = New-Object System.Collections.Generic.List[string]

foreach ($file in Get-GitFiles) {
  $normalized = Normalize-PathText $file
  $lower = $normalized.ToLowerInvariant()
  $name = [System.IO.Path]::GetFileName($normalized).ToLowerInvariant()
  $extension = [System.IO.Path]::GetExtension($normalized).ToLowerInvariant()

  if ($allowedExact -contains $normalized) {
    continue
  }
  if ($allowedPrefixes | Where-Object { $lower.StartsWith($_.ToLowerInvariant()) }) {
    continue
  }
  if ($lower.StartsWith("scripts/") -and $extension -eq ".ps1") {
    continue
  }
  if ($lower.StartsWith("gw-studio-tauri/src-tauri/src/")) {
    continue
  }
  if ($lower.StartsWith("gw-studio-tauri/src/")) {
    continue
  }
  if ($lower.StartsWith("gw-studio-tauri/public/") -and $extension -in @(".png", ".jpg", ".jpeg", ".webp", ".svg", ".ico")) {
    continue
  }
  if ($lower.StartsWith("game-and-watch-retro-go-sylverb/") -and $extension -in @(".c", ".h", ".cpp", ".hpp", ".s", ".ld", ".mk", ".txt", ".md", ".sh", ".py", ".json", ".yml", ".yaml", ".toml", ".cmake", "")) {
    continue
  }
  if ($lower.StartsWith("game-and-watch-retro-go-sylverb/linux/makefile.")) {
    continue
  }

  foreach ($pattern in $forbiddenPathPatterns) {
    if ($lower.Contains($pattern.ToLowerInvariant())) {
      $violations.Add("forbidden path: $normalized")
      break
    }
  }

  if ($forbiddenExtensions -contains $extension) {
    $violations.Add("forbidden extension $extension`: $normalized")
    continue
  }

  foreach ($pattern in $forbiddenNamePatterns) {
    if ($name.Contains($pattern)) {
      $violations.Add("suspicious name '$pattern': $normalized")
      break
    }
  }
}

if ($violations.Count -gt 0) {
  Write-Host "Release safety check failed:" -ForegroundColor Red
  $violations | Sort-Object -Unique | ForEach-Object { Write-Host " - $_" -ForegroundColor Red }
  exit 1
}

Write-Host "Release safety check passed: no forbidden release files found." -ForegroundColor Green
