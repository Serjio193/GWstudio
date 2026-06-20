export function isUnknownValue(value) {
  if (value == null) return true;
  const normalized = String(value).trim().toUpperCase();
  return normalized === "" || normalized === "UNKNOWN";
}

export function isDeviceReadEmpty(info = {}) {
  return [
    info.programmer,
    info.device_uid,
    info.target_voltage,
    info.detected_firmware,
    info.external_flash,
    info.protection,
    info.filesystem,
  ].every(isUnknownValue);
}

export function isValidDeviceUid(value) {
  const uid = String(value ?? "").trim().toUpperCase();
  return /^[0-9A-F]{24}$/.test(uid) && !/^0+$/.test(uid) && !/^F+$/.test(uid);
}

export function deviceUidErrorText(value) {
  if (isValidDeviceUid(value)) {
    return "";
  }
  const text = String(value ?? "").trim();
  if (!text || text.toUpperCase() === "UNKNOWN") {
    return "Device UID не прочитан. Проверьте подключение ST-LINK и питание консоли, нажмите кнопку включения на консоли и повторите Read Device Info.";
  }
  return "Device UID выглядит некорректно. Проверьте подключение ST-LINK, питание консоли и повторите Read Device Info.";
}

export function normalizeFirmwareAlias(value) {
  const normalized = String(value ?? "").trim().toUpperCase();
  if (normalized.startsWith("Z")) {
    return "Z";
  }
  if (normalized.startsWith("M")) {
    return "M";
  }
  return null;
}

export function formatFirmwareLabel(value) {
  const normalized = normalizeFirmwareAlias(value);
  const fallback = String(value ?? "UNKNOWN").trim() || "UNKNOWN";
  return normalized ?? fallback;
}
