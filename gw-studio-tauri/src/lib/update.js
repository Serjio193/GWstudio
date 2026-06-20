export const GITHUB_REPOSITORY_URL = "https://github.com/Serjio193/GWstudio";
export const GITHUB_LATEST_RELEASE_API = "https://api.github.com/repos/Serjio193/GWstudio/releases/latest";
export const PAYPAL_THANKS_URL = "https://www.paypal.com/paypalme/SerhiiTarnopovych";
export const RELEASE_EXE_ASSET_NAME = "GWStudio.exe";
export const RELEASE_SHA256_ASSET_NAME = `${RELEASE_EXE_ASSET_NAME}.sha256`;
export const RELEASE_SIGNATURE_ASSET_NAME = `${RELEASE_EXE_ASSET_NAME}.sig`;

function parseVersion(value) {
  return String(value ?? "")
    .trim()
    .replace(/^v/i, "")
    .split(/[^\d]+/)
    .filter(Boolean)
    .map((part) => Number(part));
}

export function compareVersions(a, b) {
  const left = parseVersion(a);
  const right = parseVersion(b);
  const length = Math.max(left.length, right.length, 3);
  for (let index = 0; index < length; index += 1) {
    const delta = (left[index] ?? 0) - (right[index] ?? 0);
    if (delta !== 0) {
      return delta > 0 ? 1 : -1;
    }
  }
  return 0;
}

export function parseSha256Text(text) {
  const match = String(text ?? "").match(/\b[a-fA-F0-9]{64}\b/);
  return match ? match[0].toLowerCase() : "";
}
