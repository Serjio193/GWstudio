# Release Signing

GW Studio release artifacts can be signed with the local Ed25519 release key.

Tracked public files:

- `release_keys/gwstudio_release_ed25519.pub`
- `release_keys/allowed_signers`

Local private file, never commit:

- `secrets/gwstudio_release_ed25519_v2`

Sign the release executable:

```powershell
Copy-Item .\gw-studio-tauri\src-tauri\target\release\gw_studio_tauri.exe .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe -Force
ssh-keygen -Y sign -f .\secrets\gwstudio_release_ed25519_v2 -n gwstudio-release .\gw-studio-tauri\src-tauri\target\release\GWStudio.exe
```

This creates:

```text
GWStudio.exe.sig
```

Verify the signature:

```powershell
cmd /c "type ""%CD%\gw-studio-tauri\src-tauri\target\release\GWStudio.exe"" | ssh-keygen -Y verify -f ""%CD%\release_keys\allowed_signers"" -I gwstudio-release -n gwstudio-release -s ""%CD%\gw-studio-tauri\src-tauri\target\release\GWStudio.exe.sig"""
```

Release upload should include:

- `GWStudio.exe`
- `GWStudio.exe.sha256`
- `GWStudio.exe.sig`

Security rule:

- Do not copy `secrets/` into release archives.
- Do not push private keys.
- If the private key is lost, create a new key and update the public key in the app before publishing signed updates.
