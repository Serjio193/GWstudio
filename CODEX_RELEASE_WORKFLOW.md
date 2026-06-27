# Codex Release Workflow

Use this file when publishing a new GW Studio version from this repository.

The updater expects the latest GitHub Release to contain exactly:

- `GWStudio.exe`
- `GWStudio.exe.sha256`
- `GWStudio.exe.sig`

The app update check reads the GitHub latest release API and uses the `GWStudio.exe`
asset `digest` as the primary SHA256 source. The `.sha256` asset is still uploaded
as a fallback and for manual verification.

## 1. Preflight

```powershell
git status --short --branch
gh auth status
gh release list --repo Serjio193/GWstudio --limit 8
```

Do not continue if the worktree has unrelated changes.

## 2. Bump Version

Update the version in all files:

- `README.md`
- `gw-studio-tauri/package.json`
- `gw-studio-tauri/package-lock.json`
- `gw-studio-tauri/src-tauri/Cargo.toml`
- `gw-studio-tauri/src-tauri/Cargo.lock`
- `gw-studio-tauri/src-tauri/tauri.conf.json`

Example target: `1.0.18`.

Also update `README.md` release notes summary if behavior changed.

## 3. Validate

```powershell
cd .\gw-studio-tauri
npm run build
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cd ..
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\check-release-safety.ps1
```

All three must pass before publishing.

## 4. Build Portable EXE

```powershell
cd .\gw-studio-tauri
npm run tauri:build
cd ..
```

The built file is:

```text
gw-studio-tauri\src-tauri\target\release\gw_studio_tauri.exe
```

## 5. Prepare Assets

Run from repo root:

```powershell
Copy-Item -LiteralPath .\gw-studio-tauri\src-tauri\target\release\gw_studio_tauri.exe -Destination .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe -Force
Remove-Item -LiteralPath .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sig -Force -ErrorAction SilentlyContinue
$sha = (Get-FileHash .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe -Algorithm SHA256).Hash.ToLower()
Set-Content -LiteralPath .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sha256 -Value $sha -NoNewline
```

Important: keep `.sha256` as the 64-character lowercase hash only. The updater can
parse both formats, but the current published assets use hash-only content.

## 6. Sign

```powershell
ssh-keygen -Y sign -f .\secrets\gwstudio_release_ed25519_v2 -n gwstudio-release -I gwstudio-release .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe
```

Verify:

```powershell
cmd /c 'type "E:\Game Watch\GWstudio\gw-studio-tauri\src-tauri\target\release\GWStudio.exe" | ssh-keygen -Y verify -f "E:\Game Watch\GWstudio\release_keys\allowed_signers" -I gwstudio-release -n gwstudio-release -s "E:\Game Watch\GWstudio\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sig"'
```

Required success line:

```text
Good "gwstudio-release" signature
```

If OpenSSH rejects the private key due to permissions, fix only the extra sandbox
ACL entry on the key. Do not replace the release key unless intentionally rotating
the public key in the app first.

## 7. Commit And Push

Stage only intended source/version/docs changes. Do not commit release binaries.

```powershell
git status --short
git add -- README.md gw-studio-tauri/src/App.jsx gw-studio-tauri/src/lib/appData.js gw-studio-tauri/package.json gw-studio-tauri/package-lock.json gw-studio-tauri/src-tauri/Cargo.toml gw-studio-tauri/src-tauri/Cargo.lock gw-studio-tauri/src-tauri/tauri.conf.json
git commit -m "Release 1.0.18"
git push origin main
```

Adjust the staged file list to the real diff. Never use `git add -A` if unrelated
files exist.

## 8. Create GitHub Release

Use the same version as the app metadata.

```powershell
$version = "1.0.18"
$sha = Get-Content .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sha256
$notes = @"
## Changes

- Describe user-visible changes here.

## Validation

- npm run build
- cargo check --manifest-path src-tauri/Cargo.toml --all-targets
- scripts/check-release-safety.ps1
- npm run tauri:build
- ED25519 release signature verified with release_keys/allowed_signers

SHA256: $sha
"@
$notesPath = Join-Path $env:TEMP "gwstudio-v$version-notes.md"
Set-Content -LiteralPath $notesPath -Value $notes -Encoding UTF8
gh release create "v$version" .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sha256 .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sig --repo Serjio193/GWstudio --target main --title "GW Studio $version" --notes-file $notesPath --latest
```

## 9. Verify Old App Can See The Update

Do not treat `gh api` as a complete updater smoke test. That only proves GitHub
metadata is correct from the CLI. The previous published `GWStudio.exe` must also
be launched and its Settings -> Update button must see the new version.

Check latest release metadata:

```powershell
gh release list --repo Serjio193/GWstudio --limit 6
gh api repos/Serjio193/GWstudio/releases/latest --jq '{tag_name,name,published_at,assets:[.assets[]|{name,digest,size}]}'
```

Required:

- latest tag is the new version, for example `v1.0.18`
- assets include `GWStudio.exe`, `GWStudio.exe.sha256`, `GWStudio.exe.sig`
- `GWStudio.exe` has `digest` beginning with `sha256:`
- asset names match exactly, including capitalization

Quick compare sanity:

```powershell
node -e "const a='1.0.18', b='1.0.17'; const pa=a.split('.').map(Number), pb=b.split('.').map(Number); console.log(pa.some((v,i)=>v>(pb[i]||0)))"
```

The result should be `true`.

Required app smoke test:

- Download or keep the previous published `GWStudio.exe`.
- Start that exact previous exe from a clean folder.
- Open Settings.
- Click the update button.
- Confirm the log shows `[update] Available: old -> new`.
- Confirm the confirmation dialog offers the new version.

If this fails, do not publish follow-up UI-only releases until the updater itself
is fixed. The updater metadata check must run through the Tauri backend first;
WebView `fetch()` is only acceptable as a fallback.

## 10. Optional Cleanup

After release, keep only the final local release assets if disk space matters:

```powershell
$target = Resolve-Path '.\gw-studio-tauri\src-tauri\target'
$root = Resolve-Path '.'
if (-not $target.Path.StartsWith($root.Path)) { throw "Refusing to clean outside workspace: $($target.Path)" }
$paths = @('debug','release\deps','release\build','release\.fingerprint','release\incremental','release\examples','release\native','release\gw_studio_tauri.exe','release\gw_studio_tauri.pdb','release\gw_studio_tauri.d','release\.cargo-lock','.rustc_info.json','CACHEDIR.TAG')
foreach ($relative in $paths) {
  $full = Join-Path $target.Path $relative
  if (Test-Path -LiteralPath $full) {
    $resolved = Resolve-Path -LiteralPath $full
    if (-not $resolved.Path.StartsWith($target.Path)) { throw "Refusing to remove outside target: $($resolved.Path)" }
    Remove-Item -LiteralPath $resolved.Path -Recurse -Force
  }
}
```

This cleanup preserves:

- `target\release\GWStudio.exe`
- `target\release\GWStudio.exe.sha256`
- `target\release\GWStudio.exe.sig`

## Do Not

- Do not publish releases without all three assets.
- Do not rename assets.
- Do not upload `secrets/`.
- Do not commit `target/`, generated firmware, stock firmware, BIOS files, ROMs, or user backups.
- Do not delete old releases unless explicitly requested.
- Do not rotate signing keys unless the app public key is updated before the next release.
