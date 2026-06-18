import { useEffect, useMemo, useRef, useState } from "react";
import clsx from "clsx";
import { getCurrentWindow, listen, safeInvoke } from "./lib/tauri";
import { MODES, TRANSLATIONS, EMULATORS, EMULATOR_FILE_ACCEPT } from "./lib/mock";

const APP_VERSION = "1.0.5";
const GITHUB_REPOSITORY_URL = "https://github.com/Serjio193/GWstudio";
const GITHUB_LATEST_RELEASE_API = "https://api.github.com/repos/Serjio193/GWstudio/releases/latest";
const PAYPAL_THANKS_URL = "https://www.paypal.com/paypalme/SerhiiTarnopovych";

const NEUTRAL_MODE = {
  name: "Unknown Console",
  accentText: "text-zinc-400",
  accentSoftText: "text-zinc-300",
  accentBg: "bg-zinc-700",
  accentBgSoft: "bg-zinc-900/35",
  accentBorder: "border-zinc-700",
  glow: "rgba(148, 163, 184, 0.16)",
  grid: "rgba(148, 163, 184, 0.12)",
  wallGrid: "rgba(148, 163, 184, 0.12)",
  floorGrid: "rgba(148, 163, 184, 0.28)",
  sceneTuning: {
    wallGrid: 1.0,
    floorGrid: 1.0,
    spotGlow: 0.45,
  },
  screenFrame: MODES.M.screenFrame,
  quickIcon: "⚡",
};

function decodeRepoSlug(base64Value) {
  return atob(base64Value);
}

function buildThumbSource(base64Slug, label) {
  const repoSlug = decodeRepoSlug(base64Slug);
  const staticSlug = repoSlug.replaceAll("_-_", " - ").replaceAll("_", " ");
  return {
    label,
    repoSlug,
    indexUrl: `https://api.github.com/repos/libretro-thumbnails/${repoSlug}/git/trees/master?recursive=1`,
    snapsBase: `https://raw.githubusercontent.com/libretro-thumbnails/${repoSlug}/master/Named_Snaps`,
    snapsBases: [
      `https://raw.githubusercontent.com/libretro-thumbnails/${repoSlug}/master/Named_Snaps`,
      `https://thumbnails.libretro.com/${encodeURIComponent(staticSlug)}/Named_Snaps`,
    ],
  };
}

const THUMBNAIL_SOURCES = {
  nes: buildThumbSource("TmludGVuZG9fLV9OaW50ZW5kb19FbnRlcnRhaW5tZW50X1N5c3RlbQ==", "NES"),
  gb: buildThumbSource("TmludGVuZG9fLV9HYW1lX0JveQ==", "Game Boy"),
  gbc: buildThumbSource("TmludGVuZG9fLV9HYW1lX0JveV9Db2xvcg==", "Game Boy Color"),
  sms: buildThumbSource("U2VnYV8tX01hc3Rlcl9TeXN0ZW1fLV9NYXJrX0lJSQ==", "Master System"),
  gg: buildThumbSource("U2VnYV8tX0dhbWVfR2Vhcg==", "Game Gear"),
  pce: buildThumbSource("TkVDXy1fUENfRW5naW5lXy1fVHVyYm9HcmFmeF8xNg==", "PC Engine"),
  col: buildThumbSource("Q29sZWNvXy1fQ29sZWNvVmlzaW9u", "ColecoVision"),
  md: buildThumbSource("U2VnYV8tX01lZ2FfRHJpdmVfLV9HZW5lc2lz", "Mega Drive / Genesis"),
  sg: buildThumbSource("U2VnYV8tX1NHLTEwMDA=", "SG-1000"),
  msx: buildThumbSource("TWljcm9zb2Z0Xy1fTVNY", "MSX"),
  wsv: buildThumbSource("V2F0YXJhXy1fU3VwZXJ2aXNpb24=", "Watara Supervision"),
  a7800: buildThumbSource("QXRhcmlfLV83ODAw", "Atari 7800"),
};

const THUMBNAIL_INDEX_CACHE = new Map();
const REGION_ALIASES = {
  u: "usa",
  usa: "usa",
  us: "usa",
  e: "europe",
  eur: "europe",
  europe: "europe",
  j: "japan",
  jp: "japan",
  japan: "japan",
  w: "world",
  world: "world",
};
const DEFAULT_REGION_PRIORITY = ["europe", "world", "usa", "japan"];
const WEAK_VARIANT_WORDS = new Set(["version", "the", "game"]);
const BAD_VARIANT_WORDS = new Set(["beta", "proto", "prototype", "demo", "sample", "hack", "pirate", "unl", "unlicensed", "collection"]);

function cx(...classes) {
  return classes.filter(Boolean).join(" ");
}

function scaleRgbaAlpha(rgbaValue, multiplier) {
  const match = rgbaValue.match(/rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*\)/i);
  if (!match) {
    return rgbaValue;
  }
  const [, r, g, b, a] = match;
  const nextAlpha = Math.max(0, Math.min(1, Number(a) * multiplier));
  return `rgba(${r}, ${g}, ${b}, ${nextAlpha.toFixed(3)})`;
}

function polarToCartesian(cx, cy, radius, angleDeg) {
  const angleRad = ((angleDeg - 90) * Math.PI) / 180;
  return {
    x: cx + radius * Math.cos(angleRad),
    y: cy + radius * Math.sin(angleRad),
  };
}

function describePieSlice(cx, cy, radius, startAngle, endAngle) {
  const start = polarToCartesian(cx, cy, radius, endAngle);
  const end = polarToCartesian(cx, cy, radius, startAngle);
  const largeArcFlag = endAngle - startAngle <= 180 ? "0" : "1";
  return `M ${cx} ${cy} L ${start.x} ${start.y} A ${radius} ${radius} 0 ${largeArcFlag} 0 ${end.x} ${end.y} Z`;
}

function bytesToMb(bytes) {
  return Number(bytes ?? 0) / (1024 * 1024);
}

function formatMbValue(value) {
  const numeric = Number(value ?? 0);
  if (!Number.isFinite(numeric)) {
    return "0";
  }
  if (numeric >= 10) {
    return `${Math.round(numeric)}`;
  }
  if (numeric >= 1) {
    return numeric.toFixed(1).replace(/\.0$/, "");
  }
  if (numeric <= 0) {
    return "0";
  }
  return numeric.toFixed(2).replace(/0$/, "").replace(/\.0$/, "");
}

function parseVersion(value) {
  return String(value ?? "")
    .trim()
    .replace(/^v/i, "")
    .split(/[^\d]+/)
    .filter(Boolean)
    .map((part) => Number(part));
}

function compareVersions(a, b) {
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

function parseSha256Text(text) {
  const match = String(text ?? "").match(/\b[a-fA-F0-9]{64}\b/);
  return match ? match[0].toLowerCase() : "";
}

function isSupportedImportPath(path) {
  const lower = String(path ?? "").toLowerCase();
  const acceptedExtensions = new Set(
    Object.values(EMULATOR_FILE_ACCEPT)
      .flatMap((value) => value.split(","))
      .map((value) => value.trim().toLowerCase())
      .filter(Boolean),
  );
  return Array.from(acceptedExtensions).some((ext) => lower.endsWith(ext));
}

function isSupportedImagePath(path) {
  const lower = String(path ?? "").toLowerCase();
  return [".png", ".jpg", ".jpeg", ".webp"].some((ext) => lower.endsWith(ext));
}

function toImportedGame(emulatorId, entry, index) {
  return {
    id: `${emulatorId}-${entry.title}-${index}-${entry.path}`,
    emulatorId,
    title: entry.title,
    path: entry.path,
    sizeBytes: Number(entry.size_bytes ?? 0),
    imageLoaded: false,
    imageStatus: "idle",
    imageProgress: 0,
    imageUrl: null,
  };
}

function normalizeGamePathKey(path) {
  return String(path ?? "").trim().replace(/\//g, "\\").toLowerCase();
}

function gameIdentityKey(game) {
  const emulatorId = String(game?.emulatorId ?? "").trim().toLowerCase();
  const title = String(game?.title ?? "").trim().toLowerCase();
  const path = normalizeGamePathKey(game?.path);
  return path ? `${emulatorId}::${path}` : `${emulatorId}::${title}`;
}

function dedupeGames(games) {
  const seen = new Set();
  return games.filter((game) => {
    const key = gameIdentityKey(game);
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function mergeUniqueGames(existingGames, importedGames) {
  const seenKeys = new Set(existingGames.map((game) => gameIdentityKey(game)));
  const uniqueImported = importedGames.filter((game) => {
    const key = gameIdentityKey(game);
    if (seenKeys.has(key)) {
      return false;
    }
    seenKeys.add(key);
    return true;
  });
  return {
    merged: [...existingGames, ...uniqueImported],
    uniqueImported,
  };
}

function normalizeThumbnailName(name) {
  return String(name ?? "")
    .replace(/&/g, "_")
    .replace(/\*/g, "_")
    .replace(/:/g, " -")
    .replace(/\//g, "_")
    .replace(/\\/g, "_")
    .replace(/"/g, "")
    .replace(/</g, "")
    .replace(/>/g, "")
    .replace(/\?/g, "")
    .replace(/\|/g, "")
    .trim();
}

function normalizeImageMatchName(name) {
  return normalizeThumbnailName(
    String(name ?? "")
      .replace(/\.[^.]+$/g, "")
      .replace(/\s*\[[^\]]*]/g, "")
      .replace(/\s*\((rev[^)]*|prg[^)]*|beta[^)]*|proto[^)]*)\)/gi, ""),
  )
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

function removeDiacritics(value) {
  return String(value ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function extractImageNameMeta(name) {
  const raw = removeDiacritics(String(name ?? "").replace(/\.[^.]+$/g, ""));
  const regions = new Set();
  let variantPenalty = 0;
  for (const match of raw.matchAll(/\(([^)]*)\)|\[([^\]]*)]/g)) {
    const text = String(match[1] ?? match[2] ?? "").toLowerCase();
    let hasRegion = false;
    let hasNonRegion = false;
    for (const part of text.split(/[,/+\s-]+/).filter(Boolean)) {
      const region = REGION_ALIASES[part];
      if (region) {
        regions.add(region);
        hasRegion = true;
      } else {
        hasNonRegion = true;
      }
      if (/^rev|^v\d|beta|proto|prototype|demo|sample|hack|pirate|unl|collection/.test(part)) {
        variantPenalty += 8;
      }
    }
    if (hasNonRegion) {
      variantPenalty += hasRegion ? 3 : 6;
    }
  }
  return { regions, variantPenalty };
}

function normalizeImageSearchName(name) {
  let text = removeDiacritics(String(name ?? ""))
    .replace(/\.[^.]+$/g, "")
    .replace(/\[[^\]]*]/g, " ")
    .replace(/\([^)]*\)/g, " ")
    .replace(/,\s*the\b/gi, " ")
    .replace(/\bthe\s+/gi, " ")
    .replace(/&/g, " and ")
    .replace(/\+/g, " plus ")
    .replace(/['’]/g, "")
    .replace(/[^a-z0-9]+/gi, " ")
    .replace(/\b(and|the|a|an)\b/gi, " ")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
  text = text.replace(/\bpokemon\b/g, "pokemon");
  return text;
}

function imageSearchTokens(value) {
  return normalizeImageSearchName(value)
    .split(" ")
    .filter((token) => token.length > 0);
}

function editDistanceLimited(a, b, limit) {
  if (Math.abs(a.length - b.length) > limit) {
    return limit + 1;
  }
  let previous = Array.from({ length: b.length + 1 }, (_, index) => index);
  for (let i = 1; i <= a.length; i += 1) {
    const current = [i];
    let rowMin = current[0];
    for (let j = 1; j <= b.length; j += 1) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      const value = Math.min(previous[j] + 1, current[j - 1] + 1, previous[j - 1] + cost);
      current[j] = value;
      rowMin = Math.min(rowMin, value);
    }
    if (rowMin > limit) {
      return limit + 1;
    }
    previous = current;
  }
  return previous[b.length];
}

function regionScore(romMeta, imageMeta) {
  const romRegions = Array.from(romMeta.regions);
  const priority = romRegions.length > 0 ? romRegions : DEFAULT_REGION_PRIORITY;
  for (let index = 0; index < priority.length; index += 1) {
    if (imageMeta.regions.has(priority[index])) {
      return index;
    }
  }
  return priority.length + (imageMeta.regions.size > 0 ? 1 : 0);
}

function chooseBestImageMatch(matches, romMeta) {
  return [...matches].sort((a, b) => {
    const regionDelta = regionScore(romMeta, a.meta) - regionScore(romMeta, b.meta);
    if (regionDelta !== 0) return regionDelta;
    const penaltyDelta = a.meta.variantPenalty - b.meta.variantPenalty;
    if (penaltyDelta !== 0) return penaltyDelta;
    return a.fileName.localeCompare(b.fileName);
  })[0] ?? null;
}

function findFuzzyImageMatch(index, romName, romMeta) {
  const target = normalizeImageSearchName(romName);
  const targetTokens = imageSearchTokens(romName);
  if (!target || target.length < 4) {
    return null;
  }

  const subsetMatches = index.filter((entry) => {
    const entryTokens = entry.normalized.split(" ").filter(Boolean);
    if (targetTokens.length === 0 || entryTokens.length === 0) return false;
    const targetSet = new Set(targetTokens);
    const entrySet = new Set(entryTokens);
    const targetInsideEntry = targetTokens.every((token) => entrySet.has(token));
    const entryInsideTarget = entryTokens.every((token) => targetSet.has(token));
    if (!targetInsideEntry && !entryInsideTarget) return false;
    const extra = (targetInsideEntry ? entryTokens : targetTokens).filter(
      (token) => !(targetInsideEntry ? targetSet : entrySet).has(token),
    );
    return extra.length <= 2 && extra.every((token) => WEAK_VARIANT_WORDS.has(token));
  });
  if (subsetMatches.length > 0) {
    return chooseBestImageMatch(subsetMatches, romMeta);
  }

  const distanceLimit = Math.max(1, Math.floor(target.length * 0.12));
  const distanceMatches = index
    .map((entry) => ({
      entry,
      distance: editDistanceLimited(target, entry.normalized, distanceLimit),
    }))
    .filter((item) => item.distance <= distanceLimit && !item.entry.normalized.split(" ").some((token) => BAD_VARIANT_WORDS.has(token)))
    .sort((a, b) => a.distance - b.distance || regionScore(romMeta, a.entry.meta) - regionScore(romMeta, b.entry.meta));

  if (distanceMatches.length === 0) {
    return null;
  }
  const bestDistance = distanceMatches[0].distance;
  const sameDistanceMatches = distanceMatches.filter((item) => item.distance === bestDistance).map((item) => item.entry);
  const best = chooseBestImageMatch(sameDistanceMatches, romMeta);
  const bestRank = best ? `${regionScore(romMeta, best.meta)}:${best.meta.variantPenalty}` : "";
  const equallyRanked = sameDistanceMatches.filter(
    (entry) => `${regionScore(romMeta, entry.meta)}:${entry.meta.variantPenalty}` === bestRank,
  );
  if (equallyRanked.length > 1) {
    return null;
  }
  return best;
}

async function loadThumbnailIndex(source) {
  if (!source?.indexUrl) {
    return [];
  }
  if (THUMBNAIL_INDEX_CACHE.has(source.indexUrl)) {
    return THUMBNAIL_INDEX_CACHE.get(source.indexUrl);
  }
  const promise = fetch(source.indexUrl, { cache: "force-cache" })
    .then((response) => {
      if (!response.ok) {
        throw new Error(`index HTTP ${response.status}`);
      }
      return response.json();
    })
    .then((payload) => {
      if (payload?.truncated) {
        throw new Error("thumbnail index truncated");
      }
      return (payload?.tree ?? [])
        .map((entry) => String(entry?.path ?? ""))
        .filter((path) => path.startsWith("Named_Snaps/") && path.toLowerCase().endsWith(".png"))
        .map((path) => {
          const fileName = path.slice("Named_Snaps/".length);
          return {
            fileName,
            normalized: normalizeImageSearchName(fileName),
            meta: extractImageNameMeta(fileName),
          };
        })
        .filter((entry) => entry.normalized);
    });
  THUMBNAIL_INDEX_CACHE.set(source.indexUrl, promise);
  return promise;
}

async function resolveIndexedThumbnail(source, gameTitle) {
  const index = await loadThumbnailIndex(source);
  const romMeta = extractImageNameMeta(gameTitle);
  const normalizedRom = normalizeImageSearchName(gameTitle);
  const exactMatches = index.filter((entry) => entry.normalized === normalizedRom);
  return chooseBestImageMatch(exactMatches, romMeta) ?? findFuzzyImageMatch(index, gameTitle, romMeta);
}

function buildGameImageMatchKeys(title) {
  return Array.from(
    new Set(
      buildThumbnailCandidates(title)
        .map((value) => normalizeImageMatchName(value))
        .filter(Boolean),
    ),
  );
}

function buildThumbnailCandidates(title) {
  const regionMap = {
    U: "USA",
    J: "Japan",
    E: "Europe",
    K: "Korea",
    A: "Australia",
    F: "France",
    G: "Germany",
    S: "Spain",
    I: "Italy",
    B: "Brazil",
    C: "China",
    W: "World",
  };

  const original = String(title ?? "").trim();
  const withoutBrackets = original.replace(/\s*\[[^\]]*]/g, "").trim();
  const candidates = new Set();

  function push(value) {
    const normalized = normalizeThumbnailName(value).replace(/\s+/g, " ").trim();
    if (normalized) {
      candidates.add(normalized);
    }
  }

  push(original);
  push(withoutBrackets);

  const mappedRegions = withoutBrackets.replace(/\(([A-Z]{1,4})\)/g, (_, code) => {
    const expanded = code
      .split("")
      .map((part) => regionMap[part] ?? part)
      .join(", ");
    return `(${expanded})`;
  });
  push(mappedRegions);

  const withoutRev = withoutBrackets.replace(/\s*\((Rev[^)]*|PRG[^)]*|Beta[^)]*|Proto[^)]*)\)/gi, "").trim();
  push(withoutRev);
  push(
    withoutRev.replace(/\(([A-Z]{1,4})\)/g, (_, code) => {
      const expanded = code
        .split("")
        .map((part) => regionMap[part] ?? part)
        .join(", ");
      return `(${expanded})`;
    }),
  );

  return Array.from(candidates);
}

function detectInitialLanguage() {
  const candidates = [
    ...(Array.isArray(navigator.languages) ? navigator.languages : []),
    navigator.language,
  ].filter(Boolean);
  for (const candidate of candidates) {
    const normalized = String(candidate).toLowerCase();
    if (normalized.startsWith("ru")) return "ru";
    if (normalized.startsWith("uk") || normalized.startsWith("ua")) return "uk";
    if (normalized.startsWith("en")) return "en";
  }
  return "ru";
}

function Panel({ children, className = "" }) {
  return (
    <div
      className={cx(
        "rounded-3xl border border-zinc-800 bg-gradient-to-b from-zinc-950 to-[#090909] shadow-panel",
        className,
      )}
    >
      {children}
    </div>
  );
}

function StatRow({ label, value, valueClass = "text-zinc-100" }) {
  return (
    <div className="flex items-center justify-between gap-4 text-sm">
      <span className="text-zinc-400">{label}</span>
      <span className={cx("font-semibold", valueClass)}>{value}</span>
    </div>
  );
}

function StartupOverlay({ sha256, progress, message }) {
  const shortSha = sha256 ? `${sha256.slice(0, 16)}...${sha256.slice(-12)}` : "calculating...";
  const safeProgress = Math.max(0, Math.min(100, Number(progress ?? 0)));
  return (
    <div className="fixed inset-0 z-[200] flex items-center justify-center bg-black">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(34,197,94,0.18),transparent_42%)]" />
      <div className="relative flex flex-col items-center rounded-[32px] border border-emerald-400/30 bg-zinc-950/90 px-10 py-9 text-center shadow-[0_0_70px_rgba(16,185,129,0.18)]">
        <div className="gw-startup-logo flex h-24 w-24 items-center justify-center rounded-[28px] border border-emerald-300/40 bg-emerald-400 text-3xl font-black text-black shadow-[0_0_45px_rgba(52,255,176,0.34)]">
          GW
        </div>
        <div className="mt-7 text-3xl font-black tracking-wide text-white">GW Studio</div>
        <div className="mt-2 text-sm font-bold uppercase tracking-[0.32em] text-emerald-300">version {APP_VERSION}</div>
        <div className="mt-5 rounded-2xl border border-zinc-800 bg-black/60 px-5 py-3 font-mono text-xs text-zinc-300">
          SHA256: {shortSha}
        </div>
        <div className="mt-5 w-[360px] max-w-[72vw]">
          <div className="mb-2 flex items-center justify-between font-mono text-[11px] uppercase tracking-[0.2em] text-zinc-500">
            <span>{message || "Preparing runtime"}</span>
            <span>{Math.round(safeProgress)}%</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full border border-emerald-400/25 bg-black">
            <div
              className="h-full rounded-full bg-emerald-300 shadow-[0_0_18px_rgba(52,255,176,0.65)] transition-[width] duration-200"
              style={{ width: `${safeProgress}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

function isUnknownValue(value) {
  if (value == null) return true;
  const normalized = String(value).trim().toUpperCase();
  return normalized === "" || normalized === "UNKNOWN";
}

function isDeviceReadEmpty(info = {}) {
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

function isValidDeviceUid(value) {
  const uid = String(value ?? "").trim().toUpperCase();
  return /^[0-9A-F]{24}$/.test(uid) && !/^0+$/.test(uid) && !/^F+$/.test(uid);
}

function deviceUidErrorText(value) {
  if (isValidDeviceUid(value)) {
    return "";
  }
  const text = String(value ?? "").trim();
  if (!text || text.toUpperCase() === "UNKNOWN") {
    return "Device UID не прочитан. Проверьте подключение ST-LINK и питание консоли, нажмите кнопку включения на консоли и повторите Read Device Info.";
  }
  return "Device UID выглядит некорректно. Проверьте подключение ST-LINK, питание консоли и повторите Read Device Info.";
}

function normalizeFirmwareAlias(value) {
  const normalized = String(value ?? "").trim().toUpperCase();
  if (normalized.startsWith("Z")) {
    return "Z";
  }
  if (normalized.startsWith("M")) {
    return "M";
  }
  return null;
}

function formatFirmwareLabel(value) {
  const normalized = normalizeFirmwareAlias(value);
  const fallback = String(value ?? "UNKNOWN").trim() || "UNKNOWN";
  return normalized ?? fallback;
}

function ModeButton({ active, mode, onClick, children }) {
  const activeClass =
    mode === "Z"
      ? "border-emerald-400 bg-emerald-950/70 text-emerald-200 shadow-[0_0_26px_rgba(16,185,129,0.35)]"
      : "border-red-400 bg-red-950/70 text-red-100 shadow-[0_0_26px_rgba(239,68,68,0.35)]";

  const idleClass =
    mode === "Z"
      ? "border-emerald-800/60 bg-black/30 text-emerald-300 hover:bg-emerald-950/40"
      : "border-red-800/60 bg-black/30 text-red-300 hover:bg-red-950/40";

  return (
    <button
      type="button"
      onClick={onClick}
      className={cx(
        "h-11 px-5 rounded-2xl border font-bold transition-all duration-300 hover:-translate-y-0.5",
        active ? activeClass : idleClass,
      )}
    >
      {children}
    </button>
  );
}

function DeviceMockup({ mode, modeName, builderSettingsOpen, selectedEmulator, selectedGame, sceneTuning, screenFrame, showConsole = true, readInfoIssue = "" }) {
  const isZMode = modeName === "Z";
  const consoleImage = isZMode ? "/assets/z-console.png" : "/assets/m-console.png";
  const emulatorInfo = EMULATORS.find(([id]) => id === selectedEmulator);
  const emulatorLabel = emulatorInfo?.[1];
  const emulatorIcon = emulatorInfo?.[2];
  const emulatorDescription = emulatorInfo?.[3];
  const tuning = sceneTuning ?? mode.sceneTuning ?? { wallGrid: 1, floorGrid: 1, spotGlow: 1 };
  const wallGridColor = scaleRgbaAlpha(mode.wallGrid ?? mode.grid, tuning.wallGrid);
  const floorGridColor = scaleRgbaAlpha(mode.floorGrid ?? mode.grid, tuning.floorGrid);
  const spotGlowColor = scaleRgbaAlpha(mode.glow, tuning.spotGlow);
  const frame = screenFrame ?? mode.screenFrame ?? { left: 32.6, top: 27.7, width: 36.2, height: 32.4, radius: 10 };
  const showScreenOverlay = Boolean(selectedEmulator || selectedGame?.imageLoaded);

  return (
    <div className="gw-scene gw-hero-panel relative h-full min-h-[430px] overflow-hidden rounded-[28px] border border-zinc-800/80 bg-[#060606] shadow-[0_0_80px_rgba(0,0,0,0.55)]">
      <div
        className="absolute inset-x-0 top-0 h-[62%] opacity-35"
        style={{
          backgroundImage: `linear-gradient(${wallGridColor} 1px, transparent 1px), linear-gradient(90deg, ${wallGridColor} 1px, transparent 1px)`,
          backgroundSize: "30px 30px",
        }}
      />
      <div
        className="absolute inset-x-[-6%] bottom-[-16%] h-[46%] overflow-hidden"
        style={{
          transform: "perspective(1200px) rotateX(45deg)",
          transformOrigin: "center top",
          opacity: 0.68,
        }}
      >
        <div
          className="h-full w-full"
          style={{
            backgroundImage: `linear-gradient(${floorGridColor} 1px, transparent 1px), linear-gradient(90deg, ${floorGridColor} 1px, transparent 1px)`,
            backgroundSize: "48px 48px",
            backgroundPosition: "center top",
            maskImage:
              "radial-gradient(ellipse at center top, rgba(255,255,255,1) 0%, rgba(255,255,255,0.92) 22%, rgba(255,255,255,0.58) 54%, rgba(255,255,255,0.10) 100%)",
            WebkitMaskImage:
              "radial-gradient(ellipse at center top, rgba(255,255,255,1) 0%, rgba(255,255,255,0.92) 22%, rgba(255,255,255,0.58) 54%, rgba(255,255,255,0.10) 100%)",
          }}
        />
      </div>
      <div
        className="absolute inset-x-[10%] top-[57.2%] h-[12%] opacity-75"
        style={{
          background: `radial-gradient(ellipse at center, ${floorGridColor} 0%, rgba(255,255,255,0.09) 22%, rgba(255,255,255,0) 72%)`,
          filter: "blur(14px)",
        }}
      />
      <div
        className="absolute inset-0"
        style={{ background: `radial-gradient(circle at 67% 48%, ${spotGlowColor}, transparent 48%)` }}
      />
      <div
        className="absolute inset-x-0 top-0 h-[62%] opacity-80"
        style={{
          background:
            "radial-gradient(circle at 50% 64%, rgba(255,255,255,0.06) 0%, rgba(255,255,255,0.02) 24%, rgba(255,255,255,0) 58%)",
        }}
      />
      <div
        className="absolute inset-x-0 top-[54.5%] h-[19%]"
        style={{
          background:
            "linear-gradient(to bottom, rgba(255,255,255,0.03) 0%, rgba(0,0,0,0.18) 22%, rgba(0,0,0,0.55) 58%, rgba(0,0,0,0.88) 100%)",
        }}
      />
      <div
        className="absolute inset-x-[4%] top-[58.5%] h-[2px]"
        style={{
          background: `linear-gradient(90deg, transparent 0%, rgba(255,255,255,0.08) 12%, ${spotGlowColor} 50%, rgba(255,255,255,0.08) 88%, transparent 100%)`,
          boxShadow: `0 0 18px ${spotGlowColor}, 0 0 42px ${spotGlowColor}`,
          opacity: 0.95,
        }}
      />
      <div
        className="absolute inset-x-[18%] top-[58.2%] h-[8px] opacity-60"
        style={{
          background: `radial-gradient(ellipse at center, ${spotGlowColor} 0%, rgba(255,255,255,0.18) 22%, rgba(255,255,255,0) 74%)`,
          filter: "blur(10px)",
        }}
      />
      <div
        className="absolute inset-x-0 bottom-0 h-[45%]"
        style={{
          background:
            "linear-gradient(to top, rgba(0,0,0,0.58) 0%, rgba(0,0,0,0.24) 38%, transparent 100%)",
        }}
      />
      <div
        className="absolute inset-x-[8%] bottom-[10%] h-[1px] opacity-60"
        style={{
          boxShadow: `0 0 140px 34px ${mode.glow}`,
        }}
      />

      <div className="relative z-10 flex h-full items-center justify-center p-6">
        {!showConsole && (
          <div className={cx(
            "max-w-[520px] rounded-3xl border bg-black/35 px-8 py-6 text-center",
            readInfoIssue ? "border-amber-500/50 text-amber-200" : "border-zinc-800 text-zinc-500",
          )}>
            <div className="text-sm font-black uppercase tracking-[0.24em]">Mainboard unknown</div>
            <div className="mt-2 text-xs leading-5">
              {readInfoIssue || "Run Read Device Info to identify console hardware"}
            </div>
          </div>
        )}
        {showConsole && (
        <div className="relative flex h-full w-full items-center justify-center">
          <div
            className="gw-console-shadow absolute bottom-[9%] left-1/2 h-[14%] w-[48%] -translate-x-1/2 rounded-[999px] bg-black/80"
            style={{
              boxShadow: `0 0 65px 14px ${spotGlowColor}`,
            }}
          />
          <div
            className="pointer-events-none absolute bottom-[15%] left-1/2 z-0 h-[16%] w-[44%] -translate-x-1/2 rounded-[999px] opacity-95"
            style={{
              background: `radial-gradient(ellipse at center, ${spotGlowColor} 0%, rgba(255,255,255,0.28) 18%, rgba(255,255,255,0.07) 42%, rgba(255,255,255,0) 76%)`,
              filter: "blur(30px)",
            }}
          />
          <div className="gw-console-hero relative">
            <div className="gw-console-spin relative">
              <div
                className="pointer-events-none absolute inset-0 rounded-[34px] opacity-70"
                style={{
                  transform: "translateZ(-1px)",
                  boxShadow: `0 26px 70px rgba(0,0,0,0.58), 0 0 85px ${spotGlowColor}`,
                }}
              />
              <div
                className="pointer-events-none absolute inset-[5%] rounded-[28px] opacity-60"
                style={{
                  background:
                    "linear-gradient(115deg, rgba(255,255,255,0.22) 0%, rgba(255,255,255,0.03) 26%, rgba(255,255,255,0) 52%)",
                  mixBlendMode: "screen",
                }}
              />
              <img
                src={consoleImage}
                alt="Game & Watch Console"
                className="gw-console-image relative z-10 max-h-full w-[720px] max-w-[84vw] object-contain select-none drop-shadow-[0_22px_50px_rgba(0,0,0,0.72)]"
              />
              {showScreenOverlay && (
                <div
                  className="pointer-events-none absolute z-20 overflow-hidden border border-black/40 bg-black"
                  style={{
                    left: `${frame.left}%`,
                    top: `${frame.top}%`,
                    width: `${frame.width}%`,
                    height: `${frame.height}%`,
                    borderRadius: `${frame.radius}px`,
                    transform:
                      `perspective(900px) rotateX(${frame.rotateX ?? 0}deg) rotateY(${frame.rotateY ?? 0}deg) rotateZ(${frame.rotateZ ?? 0}deg)`,
                    transformOrigin: "center center",
                  }}
                >
                  {selectedGame?.imageLoaded ? (
                    <div className="relative flex h-full w-full items-center justify-center bg-gradient-to-b from-zinc-950 via-black to-zinc-950">
                      <div
                        className="absolute inset-0 opacity-55"
                        style={{
                          background: `radial-gradient(circle at center, ${spotGlowColor} 0%, rgba(255,255,255,0.04) 26%, rgba(255,255,255,0) 72%)`,
                        }}
                      />
                      <div className="absolute inset-0 opacity-20 mix-blend-screen">
                        <div className="h-full w-full bg-[linear-gradient(rgba(255,255,255,0.08)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.06)_1px,transparent_1px)] bg-[size:14px_14px]" />
                      </div>
                      {selectedGame.imageUrl ? (
                        <img
                          src={selectedGame.imageUrl}
                          alt={selectedGame.title}
                          className="relative z-10 h-full w-full object-cover"
                        />
                      ) : (
                        <div className="relative px-3 text-center">
                          <div className={cx("text-sm font-black tracking-wide", mode.accentText)}>
                            {selectedGame.title}
                          </div>
                          <div className="mt-1 text-[10px] text-zinc-400">Loaded game screenshot</div>
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="relative flex h-full w-full items-center justify-center bg-gradient-to-b from-zinc-950 via-black to-zinc-950">
                      <div
                        className="absolute inset-0 opacity-50"
                        style={{
                          background: `radial-gradient(circle at center, ${spotGlowColor} 0%, rgba(255,255,255,0.03) 28%, rgba(255,255,255,0) 76%)`,
                        }}
                      />
                      <div className="absolute inset-0 opacity-15 mix-blend-screen">
                        <div className="h-full w-full bg-[linear-gradient(rgba(255,255,255,0.08)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.06)_1px,transparent_1px)] bg-[size:14px_14px]" />
                      </div>
                      <div className="relative px-3 text-center">
                        <div className="mx-auto flex h-12 w-12 items-center justify-center overflow-hidden rounded-xl border border-zinc-800 bg-black/40 p-1">
                          <img
                            src={emulatorIcon ?? "/emulators/nes.png"}
                            alt={emulatorLabel ?? "Emulator"}
                            className="h-full w-full object-contain"
                          />
                        </div>
                        <div className={cx("mt-2 text-[28px] font-black tracking-widest leading-none", mode.accentText)}>
                          {emulatorLabel}
                        </div>
                        <div className="mt-2 text-[11px] font-bold text-zinc-200">
                          {emulatorDescription}
                        </div>
                        <div className="mt-2 text-[10px] text-zinc-500">
                          Select a game to preview screenshot
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
        )}
      </div>
    </div>
  );
}

function DeviceStatus({ mode, onReadInfo, isReadingInfo, readInfoProgress, deviceInfo, t, showUidWarning = true }) {
  const protection = deviceInfo.protection ?? "UNKNOWN";
  const firmwareLabel = formatFirmwareLabel(deviceInfo.detected_firmware);
  const noDeviceRead = showUidWarning && isDeviceReadEmpty(deviceInfo);
  const uidIssue = showUidWarning ? deviceUidErrorText(deviceInfo.device_uid) : "";
  const protectionClass =
    protection === "UNLOCKED"
      ? "text-emerald-400"
      : protection === "LOCKED"
        ? "text-red-400"
        : "text-yellow-400";

  const headline = !isUnknownValue(deviceInfo.mcu_profile)
    ? deviceInfo.mcu_profile
    : !isUnknownValue(deviceInfo.detected_firmware)
      ? `${firmwareLabel} BASE`
      : "STM32H7 DETECTED";

  return (
    <Panel className="gw-device-status-panel group h-full min-h-[430px] p-6 transition-all duration-300 hover:border-red-500/80 hover:shadow-[0_0_45px_rgba(239,68,68,0.20)]">
        <div className="flex items-center justify-between">
          <div className={cx("text-sm font-black uppercase tracking-wide", mode.accentText)}>
            {t.deviceStatus}
          </div>
        </div>

          <div className="mt-7 flex items-center gap-5">
          <div
            className={cx(
              "flex h-20 w-20 shrink-0 items-center justify-center rounded-full border-4 text-4xl transition-transform duration-300 group-hover:scale-105",
              mode.accentBorder,
              mode.accentText,
            )}
          >
            {isReadingInfo ? "…" : "✓"}
            </div>
            <div className="text-lg font-semibold text-zinc-200">
              {headline}
            </div>
          </div>

          <div className="mt-7 space-y-3">
            <StatRow label={t.programmer} value={deviceInfo.programmer ?? "UNKNOWN"} />
            <StatRow label={t.deviceUid} value={deviceInfo.device_uid ?? "UNKNOWN"} />
            <StatRow label={t.voltage} value={deviceInfo.target_voltage ?? "UNKNOWN"} />
            <StatRow label={t.detectedFirmware} value={firmwareLabel} />
            <StatRow label={t.externalFlash} value={deviceInfo.external_flash ?? "UNKNOWN"} />
            <StatRow
              label={t.protection}
              value={isReadingInfo ? "READING..." : protection}
              valueClass={isReadingInfo ? "text-yellow-400" : protectionClass}
            />
          </div>

          {(noDeviceRead || uidIssue) && (
            <div className="mt-5 rounded-2xl border border-amber-500/60 bg-amber-950/20 px-4 py-3 text-sm leading-5 text-amber-200">
              {noDeviceRead
                ? t.deviceReadFailedHint
                : uidIssue}
            </div>
          )}

          <button
            type="button"
            onClick={onReadInfo}
            disabled={isReadingInfo}
            className={cx(
              "relative mt-6 w-full overflow-hidden rounded-xl border px-5 py-3 text-left text-sm font-bold transition hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-100",
              mode.accentBorder,
              mode.accentText,
            )}
          >
            {isReadingInfo && (
              <span
                className="absolute inset-y-0 left-0 transition-[width] duration-150"
                style={{ width: `${readInfoProgress}%`, background: mode.glow }}
              />
            )}
            <span className="relative z-10">
              {isReadingInfo ? `↻ ${t.readingDeviceInfo} ${readInfoProgress}%` : `↻ ${t.readDeviceInfo}`}
            </span>
          </button>
        </Panel>
  );
}

function ActionCard({ title, desc, icon, accent, mode, tone, disabled = false, active = false, onClick }) {
  const toneClass =
    tone === "blue"
      ? "text-sky-400 border-sky-800/70 bg-sky-950/20"
      : tone === "gold"
        ? "text-amber-400 border-amber-700/70 bg-amber-950/20"
        : tone === "purple"
          ? "text-fuchsia-400 border-fuchsia-800/70 bg-fuchsia-950/20"
          : cx(mode.accentText, mode.accentBorder, mode.accentBgSoft);

  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={cx(
        "gw-action-card group relative overflow-hidden min-h-[106px] rounded-[24px] border bg-gradient-to-b from-zinc-950 via-[#090909] to-black p-5 text-left transition-all duration-300",
        disabled
          ? "opacity-35 cursor-not-allowed border-zinc-900 grayscale"
          : "hover:-translate-y-1 hover:shadow-[0_0_35px_rgba(0,0,0,0.55)]",
        active ? "ring-2 ring-white/20" : "",
        accent ? toneClass : "border-zinc-800 hover:border-zinc-700",
      )}
    >
      <div
        className="absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-100"
        style={{ background: `radial-gradient(circle at top left, ${mode.glow}, transparent 55%)` }}
      />
      <div className="relative z-10 flex items-center gap-5">
        <div className={cx("text-4xl", accent ? "" : "text-zinc-300")}>{icon}</div>
        <div className="min-w-0 flex-1">
          <div className="text-lg font-black tracking-wide text-white">{title}</div>
          <div className="mt-1 text-sm text-zinc-500">{desc}</div>
        </div>
        <div className="text-2xl opacity-70 transition-transform group-hover:translate-x-1">→</div>
      </div>
    </button>
  );
}

function RecoveryPrompt({ mode, t, deviceUid, onConfirm, onDecline }) {
  return (
    <Panel className="p-6">
      <div className="text-sm font-black uppercase tracking-wide text-amber-300">
        {t.firmwareRecovery}
      </div>
      <div className="mt-3 text-2xl font-black text-zinc-100">
        {t.firmwareRecoveryTitle}
      </div>
      <div className="mt-2 max-w-3xl text-sm leading-6 text-zinc-400">
        {t.firmwareRecoveryText.replace("{uid}", deviceUid)}
      </div>
      <div className="mt-5 grid max-w-xl grid-cols-2 gap-3">
        <button
          type="button"
          onClick={onConfirm}
          className={cx("rounded-2xl border px-5 py-4 text-left font-black transition hover:bg-zinc-900", mode.accentBorder, mode.accentBgSoft, mode.accentText)}
        >
          {t.yesRecover}
          <div className="mt-1 text-xs font-normal text-zinc-500">{t.findBackupOrManual}</div>
        </button>
        <button
          type="button"
          onClick={onDecline}
          className="rounded-2xl border border-zinc-800 bg-black/35 px-5 py-4 text-left font-black text-zinc-300 transition hover:border-zinc-600 hover:bg-zinc-900"
        >
          {t.no}
          <div className="mt-1 text-xs font-normal text-zinc-500">{t.leaveBlocked}</div>
        </button>
      </div>
    </Panel>
  );
}

function OriginalFirmwarePicker({ mode, t, onSelect, onCancel }) {
  return (
    <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/70 px-6 backdrop-blur-sm">
      <div className="w-full max-w-2xl rounded-[28px] border border-amber-600/60 bg-[#080808] p-6 shadow-[0_0_55px_rgba(0,0,0,0.65)]">
        <div className="text-sm font-black uppercase tracking-wide text-amber-300">
          {t.restoreOriginalFirmware}
        </div>
        <div className="mt-3 text-2xl font-black text-zinc-100">
          {t.selectHardwareModel}
        </div>
        <div className="mt-2 text-sm leading-6 text-zinc-400">
          {t.restoreOriginalHint}
        </div>
        <div className="mt-6 grid grid-cols-2 gap-3">
          <button
            type="button"
            onClick={() => onSelect("M")}
            className="rounded-2xl border border-red-600/70 bg-red-950/20 px-5 py-5 text-left transition hover:bg-zinc-900"
          >
            <div className="text-xl font-black text-red-200">{t.marioHardware}</div>
            <div className="mt-1 text-xs text-zinc-500">Flash stock_bank1m + stock_spi_m</div>
          </button>
          <button
            type="button"
            onClick={() => onSelect("Z")}
            className={cx("rounded-2xl border px-5 py-5 text-left transition hover:bg-zinc-900", mode.accentBorder, mode.accentBgSoft)}
          >
            <div className="text-xl font-black text-emerald-200">{t.zeldaHardware}</div>
            <div className="mt-1 text-xs text-zinc-500">Flash stock_bank1z + stock_spi_z</div>
          </button>
        </div>
        <button
          type="button"
          onClick={onCancel}
          className="mt-4 w-full rounded-2xl border border-zinc-800 bg-black/40 px-5 py-3 text-left text-sm font-bold text-zinc-400 transition hover:border-zinc-600 hover:bg-zinc-900"
        >
          {t.cancel}
        </button>
      </div>
    </div>
  );
}

function ConfirmDialog({ mode, t, title, message, confirmText, cancelText, tone = "emerald", onConfirm, onCancel }) {
  const resolvedConfirmText = confirmText ?? t.yesRecover;
  const resolvedCancelText = cancelText ?? t.no;
  const toneClass = tone === "amber" ? "border-amber-600/70 bg-amber-950/20 text-amber-300" : cx(mode.accentBorder, mode.accentBgSoft, mode.accentText);
  return (
    <div className="fixed inset-0 z-[130] flex items-center justify-center bg-black/70 px-6 backdrop-blur-sm">
      <div className="w-full max-w-xl rounded-[28px] border border-zinc-800 bg-[#080808] p-6 shadow-[0_0_55px_rgba(0,0,0,0.65)]">
        <div className={cx("text-sm font-black uppercase tracking-wide", tone === "amber" ? "text-amber-300" : mode.accentText)}>
          {title}
        </div>
        <div className="mt-3 text-sm leading-6 text-zinc-400">{message}</div>
        <div className="mt-6 grid grid-cols-2 gap-3">
          <button
            type="button"
            onClick={onConfirm}
            className={cx("rounded-2xl border px-5 py-4 text-left text-sm font-black transition hover:bg-zinc-900", toneClass)}
          >
            {resolvedConfirmText}
          </button>
          <button
            type="button"
            onClick={onCancel}
            className="rounded-2xl border border-zinc-800 bg-black/35 px-5 py-4 text-left text-sm font-black text-zinc-300 transition hover:border-zinc-600 hover:bg-zinc-900"
          >
            {resolvedCancelText}
          </button>
        </div>
      </div>
    </div>
  );
}

function BiosDropDialog({ mode, t, status, title, hint, onCancel }) {
  return (
    <div className="fixed inset-0 z-[135] flex items-center justify-center bg-black/75 px-6 backdrop-blur-sm">
      <div className="w-full max-w-2xl rounded-[28px] border border-sky-700/70 bg-[#080808] p-6 shadow-[0_0_55px_rgba(0,0,0,0.65)]">
        <div className={cx("text-sm font-black uppercase tracking-wide", mode.accentText)}>
          {title}
        </div>
        <div className="mt-3 text-sm leading-6 text-zinc-400">
          {hint}
        </div>
        <div className="mt-4 rounded-2xl border border-zinc-800 bg-black/35 p-4">
          <div className="text-xs font-black uppercase tracking-wide text-zinc-500">{t.missingBios}</div>
          <div className="mt-2 font-mono text-xs leading-5 text-sky-200">
            {(status?.missing ?? []).join(", ") || t.checking}
          </div>
          <div className="mt-2 truncate font-mono text-[11px] text-zinc-600">{status?.dir ?? ""}</div>
        </div>
        <button
          type="button"
          onClick={onCancel}
          className="mt-4 w-full rounded-2xl border border-zinc-800 bg-black/40 px-5 py-3 text-left text-sm font-bold text-zinc-400 transition hover:border-zinc-600 hover:bg-zinc-900"
        >
          {t.cancel}
        </button>
      </div>
    </div>
  );
}

function StockFirmwareDropDialog({ mode, t, gate, onSelectMcu, onSelectSpi, onCancel }) {
  return (
    <div className="fixed inset-0 z-[136] flex items-center justify-center bg-black/75 px-6 backdrop-blur-sm">
      <Panel className="w-full max-w-xl border-amber-600/70 bg-[#080605] p-6 shadow-[0_0_60px_rgba(245,158,11,0.12)]">
        <div className="text-sm font-black uppercase tracking-wide text-amber-300">
          {t.stockBackupRequired}
        </div>
        <div className="mt-2 text-sm leading-relaxed text-zinc-400">
          {gate?.message || t.importStockFiles}
        </div>
        <div className="mt-4 rounded-2xl border border-dashed border-amber-700/70 bg-black/30 p-4 text-center text-xs text-zinc-500">
          {t.stockFirmwareDropHint}
        </div>
        <div className="mt-4 grid grid-cols-2 gap-3">
          <button
            type="button"
            onClick={onSelectMcu}
            className={cx(
              "rounded-xl border px-4 py-4 text-left transition",
              gate?.mcuReady
                ? "border-emerald-700 bg-emerald-950/15 text-emerald-300"
                : "border-amber-600 bg-black/30 text-amber-200 hover:bg-zinc-900",
            )}
          >
            <div className="text-sm font-black">{t.mcuBank1}</div>
            <div className="mt-1 truncate text-[11px] text-zinc-500">
              {gate?.mcuReady ? gate.mcuName : t.selectBackup}
            </div>
          </button>
          <button
            type="button"
            onClick={onSelectSpi}
            className={cx(
              "rounded-xl border px-4 py-4 text-left transition",
              gate?.spiReady
                ? "border-emerald-700 bg-emerald-950/15 text-emerald-300"
                : "border-amber-600 bg-black/30 text-amber-200 hover:bg-zinc-900",
            )}
          >
            <div className="text-sm font-black">{t.spiFlash}</div>
            <div className="mt-1 truncate text-[11px] text-zinc-500">
              {gate?.spiReady ? gate.spiName : t.selectBackup}
            </div>
          </button>
        </div>
        <button
          type="button"
          onClick={onCancel}
          className="mt-4 w-full rounded-xl border border-zinc-800 px-4 py-3 text-left text-sm font-black text-zinc-300 hover:bg-zinc-900"
        >
          {t.cancel}
        </button>
      </Panel>
    </div>
  );
}

function EmulatorPicker({ mode, t, selectedEmulator, onSelect, onAddGames }) {
  return (
    <Panel className="flex h-full min-h-0 flex-col overflow-hidden p-5">
      <div className={cx("mb-4 text-sm font-black uppercase tracking-wide", mode.accentText)}>
        {t.emulatorBuilder}
      </div>

      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
        {EMULATORS.map(([id, label, icon, description]) => (
          <button
            key={id}
            type="button"
            onClick={() => onSelect(id)}
            className={cx(
              "w-full rounded-2xl border bg-black/35 p-4 text-left transition-all duration-300 hover:-translate-y-0.5",
              selectedEmulator === id
                ? cx(mode.accentBorder, mode.accentBgSoft)
                : "border-zinc-900 hover:border-zinc-700",
            )}
            >
              <div className="flex items-center gap-4">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-zinc-800 bg-black/40 p-1">
                <img src={icon} alt={label} className="h-full w-full object-contain" />
              </div>
              <div className="min-w-0 flex-1">
                <div className="text-sm font-black">{label}</div>
                <div className="mt-1 truncate text-[11px] leading-4 text-zinc-500">{description}</div>
              </div>
              <span
                onClick={(event) => {
                  event.stopPropagation();
                  onSelect(id);
                  onAddGames(id);
                }}
                className={cx(
                  "flex h-10 w-10 items-center justify-center rounded-xl border text-xl font-black transition",
                  selectedEmulator === id
                    ? cx(mode.accentBorder, mode.accentText, "bg-black/30")
                    : "border-zinc-800 text-zinc-300 hover:border-zinc-600 hover:text-white",
                )}
              >
                +
              </span>
            </div>
          </button>
        ))}
      </div>
    </Panel>
  );
}

function GameListPanel({
  mode,
  t,
  selectedEmulator,
  games,
  selectedGameId,
  onSelectGame,
  onRemoveGame,
  onClearGames,
  onImportImagesPack,
  onScanImagesFolder,
  onSetGameImage,
  sortMode,
  onSortModeChange,
}) {
  const emulatorLabels = Object.fromEntries(EMULATORS.map(([id, label]) => [id, label]));
  const emulatorIcons = Object.fromEntries(EMULATORS.map(([id, , icon]) => [id, icon]));
  return (
    <Panel className="flex h-full min-h-0 flex-col p-5">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div className={cx("text-sm font-black uppercase tracking-wide", mode.accentText)}>
          {t.loadedGames}
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => onSortModeChange(sortMode === "title-asc" ? "title-desc" : "title-asc")}
            disabled={games.length === 0}
            className={cx(
              "rounded-xl border px-3 py-2 text-xs font-black uppercase tracking-wide transition",
              games.length > 0
                ? cx(
                    sortMode.startsWith("title") ? mode.accentBorder : "border-zinc-700",
                    sortMode.startsWith("title") ? mode.accentText : "text-zinc-300",
                    "bg-black/35 hover:border-zinc-500 hover:bg-zinc-900",
                  )
                : "cursor-not-allowed border-zinc-900 bg-black/20 text-zinc-600",
            )}
          >
            {sortMode === "title-desc" ? t.nameZA : t.nameAZ}
          </button>
          <button
            type="button"
            onClick={() => onSortModeChange(sortMode === "size-asc" ? "size-desc" : "size-asc")}
            disabled={games.length === 0}
            className={cx(
              "rounded-xl border px-3 py-2 text-xs font-black uppercase tracking-wide transition",
              games.length > 0
                ? cx(
                    sortMode.startsWith("size") ? mode.accentBorder : "border-zinc-700",
                    sortMode.startsWith("size") ? mode.accentText : "text-zinc-300",
                    "bg-black/35 hover:border-zinc-500 hover:bg-zinc-900",
                  )
                : "cursor-not-allowed border-zinc-900 bg-black/20 text-zinc-600",
            )}
          >
            {sortMode === "size-desc" ? t.sizeDown : t.sizeUp}
          </button>
          <button
            type="button"
            onClick={onImportImagesPack}
            disabled={games.length === 0}
            className={cx(
              "rounded-xl border px-3 py-2 text-xs font-black uppercase tracking-wide transition",
              games.length > 0
                ? "border-zinc-700 bg-black/35 text-zinc-300 hover:border-zinc-500 hover:bg-zinc-900"
                : "cursor-not-allowed border-zinc-900 bg-black/20 text-zinc-600",
            )}
          >
            {t.importImages}
          </button>
          <button
            type="button"
            onClick={onScanImagesFolder}
            disabled={games.length === 0}
            className={cx(
              "rounded-xl border px-3 py-2 text-xs font-black uppercase tracking-wide transition",
              games.length > 0
                ? "border-zinc-700 bg-black/35 text-zinc-300 hover:border-zinc-500 hover:bg-zinc-900"
                : "cursor-not-allowed border-zinc-900 bg-black/20 text-zinc-600",
            )}
          >
            {t.scanFolder}
          </button>
          <button
            type="button"
            onClick={onClearGames}
            disabled={games.length === 0}
            className={cx(
              "rounded-xl border px-3 py-2 text-xs font-black uppercase tracking-wide transition",
              games.length > 0
                ? "border-zinc-700 bg-black/35 text-zinc-300 hover:border-red-700 hover:bg-red-950/20 hover:text-red-300"
                : "cursor-not-allowed border-zinc-900 bg-black/20 text-zinc-600",
            )}
          >
            {t.clearRoms}
          </button>
        </div>
      </div>

      {games.length === 0 ? (
        <div className="rounded-2xl border border-zinc-900 bg-black/35 p-4">
          <div className="text-sm font-black text-zinc-200">{t.noGamesLoaded}</div>
          <div className="mt-1 text-xs text-zinc-500">
            {selectedEmulator ? t.openRomFolder : t.addRomsAny}
          </div>
        </div>
      ) : (
        <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
          {games.map((game) => (
            <button
              key={game.id}
              type="button"
              onClick={() => onSelectGame(game.id)}
              className={cx(
                "w-full rounded-2xl border bg-black/35 p-3 text-left transition hover:border-zinc-700 hover:bg-zinc-900",
                selectedGameId === game.id ? cx(mode.accentBorder, mode.accentBgSoft) : "border-zinc-900",
              )}
            >
              <div className="flex items-center gap-4">
                <div
                  className={cx(
                    "flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-xl border bg-black/40 p-1",
                    selectedGameId === game.id ? mode.accentBorder : "border-zinc-800",
                  )}
                >
                  <img
                    src={emulatorIcons[game.emulatorId] ?? "/emulators/nes.png"}
                    alt={emulatorLabels[game.emulatorId] ?? game.emulatorId ?? "Emulator"}
                    className="h-full w-full object-contain"
                  />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-black text-zinc-100">{game.title}</div>
                  <div className="mt-1 text-[11px] uppercase tracking-wide text-zinc-600">
                    {emulatorLabels[game.emulatorId] ?? game.emulatorId ?? t.unknownEmulator}
                  </div>
                  <div className="mt-1 text-xs text-zinc-500">
                    {game.imageLoaded
                      ? t.screenshotLoaded
                      : game.imageStatus === "downloading"
                        ? `${t.downloadingImage} ${Math.round(game.imageProgress ?? 0)}%`
                        : game.imageStatus === "missing"
                          ? t.noScreenshotFound
                          : t.noImage}
                  </div>
                  {game.imageStatus === "downloading" && (
                    <div className="mt-2 h-1.5 overflow-hidden rounded-full border border-zinc-800 bg-black">
                      <div
                        className={cx("h-full transition-all duration-300", mode.accentBg)}
                        style={{ width: `${Math.round(game.imageProgress ?? 0)}%` }}
                      />
                    </div>
                  )}
                </div>
                <span
                  onClick={(event) => {
                    event.stopPropagation();
                    onSetGameImage(game.id);
                  }}
                  className="flex h-10 shrink-0 items-center justify-center rounded-xl border border-zinc-800 px-3 text-[11px] font-black uppercase tracking-wide text-zinc-300 transition hover:border-zinc-500 hover:bg-zinc-900 hover:text-white"
                  title={t.setImageManual}
                >
                  {t.setImage}
                </span>
                <span
                  onClick={(event) => {
                    event.stopPropagation();
                    onRemoveGame(game.id);
                  }}
                  className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-zinc-800 text-lg text-zinc-400 transition hover:border-red-700 hover:bg-red-950/20 hover:text-red-300"
                  title={t.removeFromList}
                >
                  ×
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </Panel>
  );
}

function BuildActionsPanel({
  t,
  mode,
  selectedEmulator,
  hasLoadedGames,
  isBuildingFirmware,
  buildFirmwareProgress,
  buildFirmwareMessage,
  buildFirmwareError,
  isDownloadingImages,
  imageDownloadProgress,
  canDownloadImages,
  isDeviceWritable,
  isDeviceIdentified,
  detectedFirmware,
  hasReadDeviceInfo,
  sdEnabled,
  spiUsedMb,
  spiTotalMb,
  sdUsedMb,
  sdTotalMb,
  firmwareBaseMb,
  emulatorUsedMb,
  estimatedEmulatorUsedMb,
  builtExtflashMb,
  romUsedMb,
  imageUsedMb,
  romCount,
  imageCount,
  onDownloadImages,
  onBuildFirmware,
}) {
  const spiPercent = spiTotalMb > 0 ? Math.min(100, Math.round((spiUsedMb / spiTotalMb) * 100)) : 0;
  const sdPercent = Math.min(100, Math.round((sdUsedMb / sdTotalMb) * 100));
  const hasBuildSelection = Boolean(selectedEmulator);
  const hasBuildMetrics = hasReadDeviceInfo;
  const hasBuiltExtflash = builtExtflashMb > 0;
  const freeMb = hasBuildMetrics ? Math.max(0, spiTotalMb - firmwareBaseMb - emulatorUsedMb) : 0;
  const usedMb = hasBuildMetrics ? firmwareBaseMb + emulatorUsedMb : 0;
  const overflowMb = hasBuildMetrics ? Math.max(0, usedMb - spiTotalMb) : 0;
  const buildOverflowLimitMb = hasBuildMetrics ? spiTotalMb * 1.3 : Infinity;
  const isBuildOverLimit = hasBuildMetrics && spiTotalMb > 0 && usedMb > buildOverflowLimitMb;
  const canBuildFirmware = hasLoadedGames && isDeviceWritable && !isBuildOverLimit && !isBuildingFirmware && !isDownloadingImages && isDeviceIdentified;
  const buildDisabledReason = !hasLoadedGames
    ? t.loadRomsToBuild
    : !isDeviceIdentified
      ? `${t.deviceNotIdentified}: ${detectedFirmware || "UNKNOWN"}`
      : !isDeviceWritable
        ? t.protectedDevice
        : isDownloadingImages
          ? t.waitImageDownload
          : isBuildOverLimit
            ? t.buildTooLarge
            : isBuildingFirmware
              ? (buildFirmwareMessage || t.packingBundle)
              : "";
  const chartTotalMb = hasBuildMetrics ? Math.max(spiTotalMb, usedMb, 1) : 1;
  const chartSegments = hasBuildMetrics
    ? [
        { key: "firmware", label: "FIRMWARE", value: firmwareBaseMb, color: "#ef4444" },
        { key: "emulator", label: "EMULATOR", value: emulatorUsedMb, color: "#71717a" },
        { key: "free", label: "FREE", value: freeMb, color: "#65a30d" },
      ]
    : [];
  const metricSegments = chartSegments.map((segment) => ({
    ...segment,
    percent: chartTotalMb > 0 ? Math.round((segment.value / chartTotalMb) * 100) : 0,
  }));
  const emulatorRomsPercent = chartTotalMb > 0 ? Math.max(0, Math.min(100, (romUsedMb / chartTotalMb) * 100)) : 0;
  const emulatorImagesPercent = chartTotalMb > 0 ? Math.max(0, Math.min(100, (imageUsedMb / chartTotalMb) * 100)) : 0;

  return (
    <Panel className="flex h-full min-h-0 flex-col p-5">
      <div className={cx("mb-4 text-sm font-black uppercase tracking-wide", mode.accentText)}>
        {t.buildOptions}
      </div>
      <div className="grid grid-cols-2 gap-3">
        <button
          type="button"
          onClick={onDownloadImages}
          disabled={!hasLoadedGames || isDownloadingImages || !canDownloadImages}
          className={cx(
            "relative overflow-hidden rounded-2xl border px-4 py-4 text-left transition",
            hasLoadedGames && !isDownloadingImages && canDownloadImages
              ? cx(mode.accentBorder, mode.accentText, "hover:bg-zinc-900")
              : "cursor-not-allowed border-zinc-900 text-zinc-600",
          )}
        >
          {isDownloadingImages && (
            <span
              className={cx("absolute inset-y-0 left-0 rounded-r-2xl opacity-80", mode.accentBgSoft)}
              style={{ width: `${Math.max(4, Math.min(100, Math.round(imageDownloadProgress ?? 0)))}%` }}
            />
          )}
          <div className="relative z-10 text-lg font-black">
            {isDownloadingImages ? `${t.downloadingImages} ${Math.round(imageDownloadProgress ?? 0)}%` : t.downloadImages}
          </div>
          <div className="relative z-10 mt-1 text-xs text-zinc-500">
            {isDownloadingImages
              ? t.downloadingMenuCovers
              : canDownloadImages
                ? t.downloadImagesHint
                : t.noPreviewSource}
          </div>
        </button>
        <button
          type="button"
          onClick={onBuildFirmware}
          disabled={!canBuildFirmware}
          className={cx(
            "relative overflow-hidden rounded-2xl border px-4 py-4 text-left transition-all duration-300",
            isBuildingFirmware
              ? cx("cursor-wait", mode.accentBorder, mode.accentBgSoft, mode.accentText)
              : hasBuiltExtflash
                ? "border-emerald-600 bg-emerald-950/20 text-emerald-300"
                : canBuildFirmware
                  ? cx(mode.accentBorder, mode.accentBgSoft, mode.accentText, "hover:bg-zinc-900")
                  : "cursor-not-allowed border-zinc-900 text-zinc-600",
          )}
        >
          {isBuildingFirmware && (
            <span className="absolute inset-0 overflow-hidden rounded-2xl">
              <span className="gw-copy-marquee" />
            </span>
          )}
          <div className="relative z-10 text-lg font-black">
            {isBuildingFirmware
              ? `${t.buildingFirmware} ${Math.round(buildFirmwareProgress ?? 0)}%`
              : hasBuiltExtflash
                ? t.firmwareBuilt
                : t.buildFirmware}
          </div>
          <div className="relative z-10 mt-1 text-xs text-zinc-500">
            {hasBuiltExtflash
              ? `${t.firmwareBuiltCanFlash}. ${t.retroGoStart}`
              : canBuildFirmware
                ? `${t.packRomsImages}: ${romCount} ROM / ${imageCount} image`
                : buildDisabledReason}
          </div>
          {isBuildingFirmware && (
            <div className="relative z-10 mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-900">
              <div
                className={cx("h-full rounded-full transition-all duration-300", mode.accentBg)}
                style={{ width: `${Math.max(4, Math.min(100, Math.round(buildFirmwareProgress ?? 0)))}%` }}
              />
            </div>
          )}
        </button>
      </div>

      {buildFirmwareError && (
        <div className="mt-4 rounded-2xl border border-red-700/70 bg-red-950/15 p-4">
          <div className="text-sm font-black uppercase tracking-wide text-red-300">
            {t.buildFailed}
          </div>
          <div className="mt-2 whitespace-pre-wrap text-xs leading-relaxed text-zinc-400">
            {buildFirmwareError}
          </div>
        </div>
      )}

      <div className="mt-6 min-h-0 flex-1 space-y-5 overflow-y-auto pr-1">
        <div className="rounded-3xl border border-zinc-900 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.04),transparent_58%),linear-gradient(180deg,#080808_0%,#050505_100%)] p-5">
          <div className="mb-4 flex items-start justify-between gap-4">
            <div>
              <div className="text-[13px] font-black uppercase tracking-[0.18em] text-zinc-300">{t.memoryUsage}</div>
              {hasBuiltExtflash && (
                <div className="mt-1.5 text-xs font-bold uppercase tracking-wide text-emerald-400">
                  {t.builtSpiImage}: {formatMbValue(builtExtflashMb)} MB
                </div>
              )}
              {overflowMb > 0 && (
                <div className="mt-1.5 text-xs font-bold uppercase tracking-wide text-red-400">
                  {t.overflow}: {formatMbValue(overflowMb)} MB
                </div>
              )}
            </div>
            <div className="text-right">
              <div className="text-[28px] font-black leading-none text-white">
                {hasBuildMetrics ? `${formatMbValue(usedMb)} MB` : "—"}
              </div>
              <div className="mt-1 text-xs uppercase tracking-wide text-zinc-500">{t.used}</div>
            </div>
          </div>

          {hasBuildMetrics ? (
            <>
              <div className="space-y-2.5">
                {metricSegments.map((segment) => (
                  <div key={segment.key} className="rounded-2xl border border-zinc-900 bg-black/35 px-4 py-2.5">
                    <div className="flex items-center justify-between gap-3">
                      <div className="flex items-center gap-2 text-[12px] uppercase tracking-wide text-zinc-400">
                        <span className="h-3 w-3 rounded-full" style={{ backgroundColor: segment.color }} />
                        {segment.key === "emulator" ? t.emulatorRomsImage : segment.label}
                      </div>
                      <div className="text-right">
                        <div className="text-2xl font-black text-zinc-100">{formatMbValue(segment.value)} MB</div>
                        <div className="text-[12px] uppercase tracking-wide text-zinc-400">{segment.percent}%</div>
                      </div>
                    </div>
                    <div className="mt-2.5 h-3 overflow-hidden rounded-full border border-zinc-800 bg-black">
                      {segment.key === "emulator" ? (
                        <div className="flex h-full w-full">
                          <div
                            className="h-full transition-all"
                            style={{ width: `${emulatorRomsPercent}%`, backgroundColor: "#71717a" }}
                          />
                          <div
                            className="h-full transition-all"
                            style={{ width: `${emulatorImagesPercent}%`, backgroundColor: "#eab308" }}
                          />
                        </div>
                      ) : (
                        <div
                          className="h-full rounded-full transition-all"
                          style={{ width: `${Math.max(2, segment.percent)}%`, backgroundColor: segment.color }}
                        />
                      )}
                    </div>
                  </div>
                ))}
              </div>

              <div className="mt-3">
                <div className="rounded-2xl border border-zinc-900 bg-black/35 px-4 py-2.5">
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2 text-[12px] uppercase tracking-wide text-zinc-400">
                      <span className="h-3 w-3 rounded-full bg-sky-500" />
                      {t.builtExtflash}
                    </div>
                    <div className="text-right">
                      <div className="text-2xl font-black text-zinc-100">
                        {hasBuiltExtflash ? `${formatMbValue(builtExtflashMb)} MB` : "—"}
                      </div>
                      <div className="text-[12px] uppercase tracking-wide text-zinc-400">
                        {hasBuiltExtflash ? t.realSize : t.runBuildFirmware}
                      </div>
                    </div>
                  </div>
                  <div className="mt-2.5 h-3 overflow-hidden rounded-full border border-zinc-800 bg-black">
                    <div
                      className="h-full rounded-full bg-sky-500 transition-all"
                      style={{
                        width: `${hasBuiltExtflash && chartTotalMb > 0 ? Math.max(2, Math.round((builtExtflashMb / chartTotalMb) * 100)) : 2}%`,
                      }}
                    />
                  </div>
                </div>
              </div>
            </>
          ) : (
            <div className="rounded-[28px] border border-zinc-900 bg-black/30 p-6 text-center text-sm text-zinc-500">
              {t.readDeviceInfoMemory}
            </div>
          )}
        </div>

        {sdEnabled && (
          <div>
            <div className="mb-2 flex justify-between text-xs">
              <span className="text-zinc-500">{t.sdCard}</span>
              <span className="font-bold text-zinc-200">
                {sdUsedMb} MB / {sdTotalMb} MB
              </span>
            </div>
            <div className="h-4 overflow-hidden rounded-full border border-zinc-800 bg-black">
              <div className="h-full bg-sky-500" style={{ width: `${sdPercent}%` }} />
            </div>
          </div>
        )}
      </div>
    </Panel>
  );
}

function FlashPanel({
  t,
  mode,
  isDeviceTransferActive,
  isDeviceWritable,
  flashOperation,
  restoreDisplayActive,
  activeFlashPhase,
  flashPhaseLabel,
  flashStage,
  flashProgress,
  flashResult,
  flashCompletionStatus,
  flashDisplayRows,
  advancedFlashEnabled,
  recoveryMode,
  hasCurrentFirmwareBuild,
  onAutoFlash,
  onRestoreDevice,
  onRestoreOriginalFirmware,
  onSelectMcuFirmware,
  onSelectBank2Firmware,
  onSelectSpiFirmware,
  onWriteMcuBackup,
  onWriteBank2Backup,
  onWriteSpiBackup,
  mcuBackupFile,
  bank2BackupFile,
  spiBackupFile,
  mcuFirmwareFile,
  mcuFirmwarePath,
  bank2FirmwareFile,
  bank2FirmwarePath,
  spiFirmwareFile,
  spiFirmwarePath,
  firmwareDirectoryHint,
}) {
  const mcuBackupReady = Boolean(mcuBackupFile);
  const spiBackupReady = Boolean(spiBackupFile);
  const isRestoreRunning = flashOperation === "restore" || restoreDisplayActive === "restore";
  const isStockRestoreRunning = flashOperation === "stock-restore" || restoreDisplayActive === "stock-restore";
  const recoveryBank1Ready = Boolean(mcuFirmwareFile);
  const recoverySpiReady = Boolean(spiFirmwareFile);
  const showAdvancedFlash = advancedFlashEnabled || recoveryMode;
  const canAutoFlash = !recoveryMode && hasCurrentFirmwareBuild && isDeviceWritable && !isDeviceTransferActive && Boolean(spiFirmwareFile && bank2FirmwareFile && mcuFirmwareFile);
  const canRestoreOriginalFirmware = isDeviceWritable && !isDeviceTransferActive;
  const canRestoreDevice = isDeviceWritable && !isDeviceTransferActive && (
    recoveryMode ? recoveryBank1Ready && recoverySpiReady : mcuBackupReady && spiBackupReady
  );
  const isAutoFlashRunning = flashOperation === "auto";
  const isAutoFlashDone = flashCompletionStatus === "auto-success" && !isAutoFlashRunning;
  const mcuBackupName = mcuFirmwareFile ? String(mcuFirmwareFile).split(/[\\/]/).pop() : "";
  const bank2Name = bank2FirmwareFile ? String(bank2FirmwareFile).split(/[\\/]/).pop() : "";
  const spiBackupName = spiFirmwareFile ? String(spiFirmwareFile).split(/[\\/]/).pop() : "";
  const writeButtonState = (kind, idleTitle, idleText) => {
    const isRunning = flashOperation === kind || activeFlashPhase === kind;
    const result = flashResult?.[kind];
    if (isRunning) {
      const isErase = flashStage === "erase" && kind === "spi";
      const isCheck = (flashStage === "prepare" || flashStage === "verify") && kind === "spi";
      return {
        title: flashPhaseLabel || idleTitle,
        text: flashStage === "prepare" && kind === "spi"
          ? t.checkingSpiSectors
          : flashStage === "verify" && kind === "spi"
            ? t.verifyingSpi
            : isErase ? t.erasingSpi : t.writingFirmware,
        tone: isCheck ? "verify" : isErase ? "erase" : "active",
      };
    }
    if (result) {
      return {
        title: result.status === "success" ? `${idleTitle} OK` : `${idleTitle} ERROR`,
        text: result.message,
        tone: result.status,
      };
    }
    return { title: idleTitle, text: idleText, tone: "idle" };
  };
  const bank1WriteState = writeButtonState("bank1", t.writeBank1, isDeviceWritable ? t.writeSelectedBank1 : t.protectedDevice);
  const bank2WriteState = writeButtonState("bank2", t.writeBank2, isDeviceWritable ? t.writeBank2Payload : t.protectedDevice);
  const spiWriteState = writeButtonState("spi", t.writeSpiFlash, isDeviceWritable ? t.writeSelectedSpi : t.protectedDevice);
  const autoRows = [
    { label: "Bank1", name: hasCurrentFirmwareBuild ? mcuBackupName : "", state: bank1WriteState },
    { label: "Bank2", name: hasCurrentFirmwareBuild ? bank2Name : "", state: bank2WriteState },
    { label: "SPI", name: hasCurrentFirmwareBuild ? spiBackupName : "", state: spiWriteState },
  ];
  const restoreRows = flashDisplayRows.length > 0 ? flashDisplayRows.map((row) => ({
    ...row,
    state: row.kind === "bank1" ? bank1WriteState : row.kind === "bank2" ? bank2WriteState : spiWriteState,
  })) : [
    { label: "Bank1", name: mcuBackupReady ? mcuBackupFile : "", state: bank1WriteState },
    { label: "Bank2", name: bank2BackupFile, state: bank2WriteState },
    { label: "SPI", name: spiBackupReady ? spiBackupFile : "", state: spiWriteState },
  ];
  const stockRestoreRows = flashDisplayRows.length > 0 ? flashDisplayRows.map((row) => ({
    ...row,
    state: row.kind === "bank1" ? bank1WriteState : row.kind === "bank2" ? bank2WriteState : spiWriteState,
  })) : [
    { label: "Bank1", name: "", state: bank1WriteState },
    { label: "SPI", name: "", state: spiWriteState },
  ];
  const transferStateClass = (state) =>
    state.tone === "success"
      ? "text-emerald-300"
      : state.tone === "error"
        ? "text-red-300"
        : state.tone === "active" || state.tone === "erase" || state.tone === "verify"
          ? mode.accentText
          : "text-zinc-500";
  const bank1WriteDisabled = (isDeviceTransferActive && flashOperation !== "bank1") || !mcuFirmwareFile || !isDeviceWritable;
  const bank2WriteDisabled = (isDeviceTransferActive && flashOperation !== "bank2") || !bank2FirmwareFile || !isDeviceWritable;
  const spiWriteDisabled = (isDeviceTransferActive && flashOperation !== "spi") || !spiFirmwareFile || !isDeviceWritable;
  const writeButtonClass = (disabled, state) =>
    cx(
      "relative overflow-hidden rounded-2xl border px-5 py-4 text-left transition-all duration-300",
      disabled
        ? "cursor-not-allowed border-zinc-900 bg-black/30 text-zinc-600"
        : state.tone === "success"
          ? "border-emerald-500 bg-emerald-950/20 text-emerald-300 hover:bg-zinc-900"
          : state.tone === "error"
            ? "border-red-500 bg-red-950/25 text-red-300 hover:bg-zinc-900"
            : state.tone === "erase"
              ? "cursor-wait border-sky-500 bg-sky-950/25 text-sky-300"
              : state.tone === "verify"
                ? "cursor-wait border-amber-500 bg-amber-950/25 text-amber-300"
              : state.tone === "active"
              ? cx("cursor-wait", mode.accentBorder, mode.accentBgSoft, mode.accentText)
              : cx(mode.accentBorder, mode.accentBgSoft, mode.accentText, "hover:bg-zinc-900"),
    );
  return (
    <Panel className="p-5 min-h-[260px]">
      <div className={cx("mb-4 text-sm font-black uppercase tracking-wide", mode.accentText)}>
        {t.flashDevice}
      </div>
      <div className="mb-4 space-y-3">
        <div className="grid grid-cols-3 gap-3">
          <button
            type="button"
            onClick={onAutoFlash}
            disabled={!canAutoFlash}
            className={cx(
              "relative overflow-hidden rounded-2xl border px-5 py-4 text-left transition-all duration-300",
              isAutoFlashRunning
                ? cx("cursor-wait", mode.accentBorder, mode.accentBgSoft, mode.accentText)
                : isAutoFlashDone
                  ? "border-emerald-500 bg-emerald-950/25 text-emerald-300 shadow-[0_0_28px_rgba(16,185,129,0.18)]"
                : canAutoFlash
                ? cx(mode.accentBorder, mode.accentBgSoft, mode.accentText, "hover:bg-zinc-900")
                : "cursor-not-allowed border-zinc-900 bg-black/30 text-zinc-600",
            )}
          >
            {isAutoFlashRunning && (
              <span className="absolute inset-0 overflow-hidden rounded-2xl">
                <span className="gw-copy-marquee" />
              </span>
            )}
            <div className="relative z-10 text-lg font-black">
              {isAutoFlashRunning ? flashPhaseLabel || t.autoFlashRunning : isAutoFlashDone ? t.autoFlashDone : t.autoFlash}
            </div>
            <div className={cx("relative z-10 mt-1 text-xs", isAutoFlashDone ? "text-emerald-200/80" : "text-zinc-500")}>
              {isAutoFlashRunning
                ? t.writingBankSpi
                : isAutoFlashDone
                  ? t.autoFlashDoneHint
                : isDeviceWritable
                ? !hasCurrentFirmwareBuild
                  ? t.buildFirmwareFirst
                  : spiFirmwareFile
                  ? bank2FirmwareFile && mcuFirmwareFile
                    ? t.flashStockForkSpi
                    : bank2FirmwareFile
                      ? t.bank1Missing
                      : t.bank2Missing
                  : t.buildFirmwareFirst
                : t.protectedDevice}
            </div>
            {(isAutoFlashRunning || isAutoFlashDone) && (
              <div className="relative z-10 mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-900">
                <div
                  className={cx("h-full rounded-full transition-all duration-300", isAutoFlashDone ? "bg-emerald-400" : mode.accentBg)}
                  style={{ width: `${isAutoFlashDone ? 100 : Math.max(4, Math.min(100, Math.round(flashProgress ?? 0)))}%` }}
                />
              </div>
            )}
            {(isAutoFlashRunning || isAutoFlashDone) && (
              <div className="relative z-10 mt-3 space-y-1">
                {autoRows.map((row) => (
                  <div key={row.label} className="flex items-center justify-between gap-2 text-[11px]">
                    <span className="min-w-0 truncate text-zinc-500">
                      {row.label}: {row.name || "missing"}
                    </span>
                    <span className={cx("shrink-0 font-black", transferStateClass(row.state))}>
                      {row.state.tone === "success"
                        ? "OK"
                        : row.state.tone === "error"
                          ? "ERR"
                          : isAutoFlashDone
                            ? "OK"
                          : row.state.tone !== "idle"
                            ? "RUN"
                            : row.name
                              ? "READY"
                              : "MISS"}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </button>
          <button
            type="button"
            onClick={onRestoreDevice}
            disabled={isRestoreRunning || !canRestoreDevice}
            className={cx(
              "relative overflow-hidden rounded-2xl border px-5 py-4 text-left transition-all duration-300",
              isRestoreRunning
                ? cx("cursor-wait", mode.accentBorder, mode.accentBgSoft, mode.accentText)
                : canRestoreDevice
                ? "border-sky-700 bg-sky-950/20 text-sky-300 hover:bg-zinc-900"
                : "cursor-not-allowed border-zinc-900 bg-black/30 text-zinc-600",
            )}
          >
            {isRestoreRunning && (
              <span className="absolute inset-0 overflow-hidden rounded-2xl">
                <span className="gw-copy-marquee" />
              </span>
            )}
            <div className="relative z-10 text-lg font-black">
              {isRestoreRunning ? flashPhaseLabel || t.unbrickRunning : recoveryMode ? t.unbrickDevice : t.restoreDevice}
            </div>
            <div className="relative z-10 mt-1 text-xs text-zinc-500">
              {isRestoreRunning
                ? recoveryMode
                  ? t.installRecoveryFiles
                  : t.restoreSavedBackup
                : isDeviceWritable
                ? recoveryMode
                  ? recoveryBank1Ready && recoverySpiReady
                    ? t.installSelectedRecovery
                    : t.selectRecoveryFirst
                  : mcuBackupReady && spiBackupReady
                    ? t.restoreMcuSpiBackup
                    : t.readOrSelectMcuSpi
                : t.protectedDevice}
            </div>
            {isRestoreRunning && (
              <div className="relative z-10 mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-900">
                <div
                  className={cx("h-full rounded-full transition-all duration-300", mode.accentBg)}
                  style={{ width: `${Math.max(4, Math.min(100, Math.round(flashProgress ?? 0)))}%` }}
                />
              </div>
            )}
            {isRestoreRunning && (
              <div className="relative z-10 mt-3 space-y-1">
                {restoreRows.map((row) => (
                  <div key={row.label} className="flex items-center justify-between gap-2 text-[11px]">
                    <span className="min-w-0 truncate text-zinc-500">
                      {row.label}: {row.name || "missing"}
                    </span>
                    <span className={cx("shrink-0 font-black", transferStateClass(row.state))}>
                      {row.state.tone === "success"
                        ? "OK"
                        : row.state.tone === "error"
                          ? "ERR"
                          : row.state.tone !== "idle"
                            ? "RUN"
                            : row.name
                              ? "READY"
                              : "MISS"}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </button>
          <button
            type="button"
            onClick={onRestoreOriginalFirmware}
            disabled={isStockRestoreRunning || !canRestoreOriginalFirmware}
            className={cx(
              "relative overflow-hidden rounded-2xl border px-5 py-4 text-left transition-all duration-300",
              isStockRestoreRunning
                ? "cursor-wait border-amber-600 bg-amber-950/25 text-amber-300"
                : canRestoreOriginalFirmware
                  ? "border-amber-700 bg-amber-950/20 text-amber-300 hover:bg-zinc-900"
                : "cursor-not-allowed border-zinc-900 bg-black/30 text-zinc-600",
            )}
          >
            {isStockRestoreRunning && (
              <span className="absolute inset-0 overflow-hidden rounded-2xl">
                <span className="gw-copy-marquee" />
              </span>
            )}
            <div className="relative z-10 text-lg font-black">
              {isStockRestoreRunning ? flashPhaseLabel || t.restoreOriginalRunning : t.restoreOriginalFirmware}
            </div>
            <div className="relative z-10 mt-1 text-xs text-zinc-500">
              {isDeviceWritable ? t.chooseHardwareThenFlash : t.protectedDevice}
            </div>
            {isStockRestoreRunning && (
              <div className="relative z-10 mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-900">
                <div
                  className="h-full rounded-full bg-amber-400 transition-all duration-300"
                  style={{ width: `${Math.max(4, Math.min(100, Math.round(flashProgress ?? 0)))}%` }}
                />
              </div>
            )}
            {isStockRestoreRunning && (
              <div className="relative z-10 mt-3 space-y-1">
                {stockRestoreRows.map((row) => (
                  <div key={row.label} className="flex items-center justify-between gap-2 text-[11px]">
                    <span className="min-w-0 truncate text-zinc-500">
                      {row.label}: {row.name || "missing"}
                    </span>
                    <span className={cx("shrink-0 font-black", transferStateClass(row.state))}>
                      {row.state.tone === "success"
                        ? "OK"
                        : row.state.tone === "error"
                          ? "ERR"
                          : row.state.tone !== "idle"
                            ? "RUN"
                            : row.name
                              ? "READY"
                              : "MISS"}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </button>
        </div>

        {showAdvancedFlash && (
          <div className="rounded-2xl border border-zinc-900 bg-black/20 p-3">
            <div className="mb-3 text-[11px] font-black uppercase tracking-wide text-zinc-500">
              {t.advancedFlash}
            </div>
        <div className="grid grid-cols-[minmax(0,1fr)_220px] gap-3">
          <button
            type="button"
            onClick={onSelectMcuFirmware}
            className="rounded-2xl border border-zinc-800 bg-black/35 px-4 py-4 text-left transition hover:border-zinc-600 hover:bg-zinc-900"
          >
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <div className="text-lg font-black text-zinc-100">{t.bank1Firmware}</div>
                <div className="mt-1 truncate text-xs text-zinc-500">{mcuBackupName || t.selectBank1Firmware}</div>
                {mcuBackupName && <div className="mt-1 truncate text-[11px] text-zinc-600">{mcuFirmwarePath || firmwareDirectoryHint}</div>}
              </div>
              <div className="text-3xl text-zinc-400">📁</div>
            </div>
          </button>
          <button
            type="button"
            onClick={onWriteMcuBackup}
            disabled={isDeviceTransferActive || !mcuFirmwareFile || !isDeviceWritable}
            className={writeButtonClass(bank1WriteDisabled, bank1WriteState)}
          >
            {(flashOperation === "bank1" || activeFlashPhase === "bank1") && (
              <span className="absolute inset-0 overflow-hidden rounded-2xl">
                <span className="gw-copy-marquee" />
              </span>
            )}
            <div className="flex items-center justify-between gap-4">
              <div className="relative z-10 min-w-0">
                <div className="text-lg font-black">{bank1WriteState.title}</div>
                <div className="mt-1 truncate text-xs text-zinc-500">{bank1WriteState.text}</div>
              </div>
              <div className="relative z-10 text-3xl">{flashOperation === "bank1" || activeFlashPhase === "bank1" ? "…" : "⬆"}</div>
            </div>
          </button>
        </div>
        <div className="grid grid-cols-[minmax(0,1fr)_220px] gap-3">
          <button
            type="button"
            onClick={onSelectBank2Firmware}
            className="rounded-2xl border border-zinc-800 bg-black/35 px-4 py-4 text-left transition hover:border-zinc-600 hover:bg-zinc-900"
          >
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <div className="text-lg font-black text-zinc-100">{t.bank2Firmware}</div>
                <div className="mt-1 truncate text-xs text-zinc-500">{bank2Name || t.resolvedBank2Payload}</div>
                {bank2Name && <div className="mt-1 truncate text-[11px] text-zinc-600">{bank2FirmwarePath || firmwareDirectoryHint}</div>}
              </div>
              <div className="text-3xl text-zinc-400">📁</div>
            </div>
          </button>
          <button
            type="button"
            onClick={onWriteBank2Backup}
            disabled={isDeviceTransferActive || !bank2FirmwareFile || !isDeviceWritable}
            className={writeButtonClass(bank2WriteDisabled, bank2WriteState)}
          >
            {(flashOperation === "bank2" || activeFlashPhase === "bank2") && (
              <span className="absolute inset-0 overflow-hidden rounded-2xl">
                <span className="gw-copy-marquee" />
              </span>
            )}
            <div className="flex items-center justify-between gap-4">
              <div className="relative z-10 min-w-0">
                <div className="text-lg font-black">{bank2WriteState.title}</div>
                <div className="mt-1 truncate text-xs text-zinc-500">{bank2WriteState.text}</div>
              </div>
              <div className="relative z-10 text-3xl">{flashOperation === "bank2" || activeFlashPhase === "bank2" ? "…" : "⬆"}</div>
            </div>
          </button>
        </div>
        <div className="grid grid-cols-[minmax(0,1fr)_220px] gap-3">
          <button
            type="button"
            onClick={onSelectSpiFirmware}
            className="rounded-2xl border border-zinc-800 bg-black/35 px-4 py-4 text-left transition hover:border-zinc-600 hover:bg-zinc-900"
          >
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <div className="text-lg font-black text-zinc-100">{t.spiFirmware}</div>
                <div className="mt-1 truncate text-xs text-zinc-500">{spiBackupName || t.selectSpiFirmware}</div>
                {spiBackupName && <div className="mt-1 truncate text-[11px] text-zinc-600">{spiFirmwarePath || firmwareDirectoryHint}</div>}
              </div>
              <div className="text-3xl text-zinc-400">📁</div>
            </div>
          </button>
          <button
            type="button"
            onClick={onWriteSpiBackup}
            disabled={isDeviceTransferActive || !spiFirmwareFile || !isDeviceWritable}
            className={writeButtonClass(spiWriteDisabled, spiWriteState)}
          >
            {(flashOperation === "spi" || activeFlashPhase === "spi") && (
              <span className="absolute inset-0 overflow-hidden rounded-2xl">
                <span className="gw-copy-marquee" />
              </span>
            )}
            <div className="flex items-center justify-between gap-4">
              <div className="relative z-10 min-w-0">
                <div className="text-lg font-black">{spiWriteState.title}</div>
                <div className="mt-1 truncate text-xs text-zinc-500">{spiWriteState.text}</div>
              </div>
              <div className="relative z-10 text-3xl">{flashOperation === "spi" || activeFlashPhase === "spi" ? "…" : "⬆"}</div>
            </div>
          </button>
        </div>
          </div>
        )}
      </div>
    </Panel>
  );
}

function BackupPanel({
  mode,
  t,
  advancedBackupEnabled,
  fullBackupDone,
  onAutoBackup,
  onReadMcuBackup,
  onReadBank2Backup,
  onReadSpiBackup,
  onSelectMcuBackup,
  onSelectBank2Backup,
  onSelectSpiBackup,
  onRevealMcuBackup,
  onRevealBank2Backup,
  onRevealSpiBackup,
  mcuBackupFile,
  mcuBackupPath,
  bank2BackupFile,
  bank2BackupPath,
  spiBackupFile,
  spiBackupPath,
  isAutoBackupRunning,
  isManualBackupRunning,
  isDeviceTransferActive,
  readingBackupKind,
  backupProgressState,
  canAutoBackup,
  canSelectBackupFiles,
  backupReadReason,
  backupDirectoryHint,
}) {
  const mcuActive = isManualBackupRunning && readingBackupKind === "mcu";
  const bank2Active = isManualBackupRunning && readingBackupKind === "bank2";
  const spiActive = isManualBackupRunning && readingBackupKind === "spi";
  const mcuFileName = mcuBackupFile ? String(mcuBackupFile).split(/[\\/]/).pop() : "";
  const bank2FileName = bank2BackupFile ? String(bank2BackupFile).split(/[\\/]/).pop() : "";
  const spiFileName = spiBackupFile ? String(spiBackupFile).split(/[\\/]/).pop() : "";
  const speedText =
    backupProgressState?.speedBps > 0
      ? `${(backupProgressState.speedBps / 1024 / 1024).toFixed(2)} MB/s`
      : "—";
  const frequencyText = backupProgressState?.frequency ? `${Math.round(backupProgressState.frequency / 1000)} kHz` : "—";
  const activityMessage = backupProgressState?.message || t.readingFromDevice;
  const spiProgressValue = Math.round(backupProgressState?.spiProgress ?? 0);
  const spiHasRealProgress = spiProgressValue > 0;
  const spiProgressText = `${spiProgressValue}%`;
  const spiStatusText = `${speedText} • ${frequencyText}`;
  const autoBackupPhaseTitle =
    readingBackupKind === "mcu"
      ? "Bank1"
      : readingBackupKind === "bank2"
        ? "Bank2"
        : readingBackupKind === "spi"
          ? spiHasRealProgress
            ? `SPI ${spiProgressText}`
            : "SPI"
          : t.backup;
  const autoBackupStatusText =
    readingBackupKind === "spi" && spiHasRealProgress
      ? spiStatusText
      : activityMessage;
  const backupRows = [
    {
      key: "mcu",
      title: t.bank1Backup,
      fileName: mcuFileName,
      path: mcuBackupPath,
      emptyText: t.backupAppearsAfterSave,
      onSelect: onSelectMcuBackup,
      onReveal: onRevealMcuBackup,
      canSelect: canSelectBackupFiles,
      onRead: onReadMcuBackup,
      active: mcuActive,
      canRead: canAutoBackup && !backupReadReason.bankLocked,
      readTitle: mcuActive ? t.readingBank1 : t.readBank1,
      readText: backupReadReason.bankLocked ? t.lockedBankReadUnavailable : canAutoBackup ? t.readBank1Backup : backupReadReason.general,
    },
    {
      key: "bank2",
      title: t.bank2Backup,
      fileName: bank2FileName,
      path: bank2BackupPath,
      emptyText: t.advancedBackupAppearsAfterSave,
      onSelect: onSelectBank2Backup,
      onReveal: onRevealBank2Backup,
      canSelect: canSelectBackupFiles,
      onRead: onReadBank2Backup,
      active: bank2Active,
      canRead: canAutoBackup && !backupReadReason.bankLocked,
      readTitle: bank2Active ? t.readingBank2 : t.readBank2,
      readText: backupReadReason.bankLocked ? t.lockedBankReadUnavailable : canAutoBackup ? t.readBank2Backup : backupReadReason.general,
      advancedOnly: true,
    },
    {
      key: "spi",
      title: t.spiBackup,
      fileName: spiFileName,
      path: spiBackupPath,
      emptyText: t.backupAppearsAfterSave,
      onSelect: onSelectSpiBackup,
      onReveal: onRevealSpiBackup,
      canSelect: canSelectBackupFiles,
      onRead: onReadSpiBackup,
      active: spiActive,
      canRead: canAutoBackup,
      readTitle: spiActive ? (spiHasRealProgress ? `${t.readingSpi} ${spiProgressText}` : t.readingSpi) : t.readSpiFlash,
      readText: spiActive ? spiStatusText : canAutoBackup ? t.readSpiBackup : backupReadReason.general,
    },
  ];

  function renderFileTile(row) {
    return (
      <button
        key={`${row.key}-file`}
        type="button"
        onClick={row.onSelect}
        disabled={!row.canSelect}
        className={cx(
          "min-w-0 rounded-2xl border px-5 py-4 text-left transition-all duration-300",
          row.canSelect
            ? "border-zinc-800 bg-black/35 hover:border-zinc-600 hover:bg-zinc-900"
            : "cursor-not-allowed border-zinc-900 bg-black/25 text-zinc-600 grayscale",
        )}
      >
        <div className="flex min-w-0 items-center justify-between gap-4">
          <div className="min-w-0">
            <div className="text-lg font-black text-zinc-100">{row.title}</div>
            <div className="mt-1 truncate text-xs text-zinc-500">{row.fileName || row.emptyText}</div>
            <div className="mt-1 truncate text-[11px] text-zinc-600">{row.path || backupDirectoryHint}</div>
          </div>
          <span
            role="button"
            tabIndex={0}
            onClick={(event) => {
              event.stopPropagation();
              row.onReveal?.();
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                event.stopPropagation();
                row.onReveal?.();
              }
            }}
            className="shrink-0 text-3xl text-zinc-400 transition hover:text-zinc-100"
            title={t.showInExplorer}
          >
            📁
          </span>
        </div>
      </button>
    );
  }

  function renderReadButton(row) {
    return (
      <button
        key={`${row.key}-read`}
        type="button"
        onClick={row.onRead}
        disabled={isManualBackupRunning || !row.canRead}
        className={cx(
          "relative min-h-[86px] overflow-hidden rounded-2xl border px-5 py-4 text-left transition-all duration-300",
          row.canRead
            ? cx(
                "hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-100",
                mode.accentBorder,
                mode.accentBgSoft,
                mode.accentText,
              )
            : "cursor-not-allowed border-zinc-900 bg-black/30 text-zinc-500 grayscale",
        )}
      >
        {isManualBackupRunning && row.active && (
          <span className="absolute inset-0 overflow-hidden rounded-2xl">
            <span className="gw-copy-marquee" />
          </span>
        )}
        <div className="relative z-10 flex h-full items-center justify-between gap-4">
          <div className="min-w-0">
            <div className="text-lg font-black">{row.readTitle}</div>
            <div className="mt-1 truncate text-xs text-zinc-500">{row.readText}</div>
          </div>
          <div className="shrink-0 text-3xl">{row.active ? "…" : "⬇"}</div>
        </div>
      </button>
    );
  }

  return (
    <Panel className="p-5 min-h-[260px]">
      <div className={cx("mb-4 text-sm font-black uppercase tracking-wide", mode.accentText)}>
        {t.backupManager}
      </div>
      <div className="space-y-4">
        {!advancedBackupEnabled && (
          <>
            <button
              type="button"
              onClick={onAutoBackup}
              disabled={isAutoBackupRunning || !canAutoBackup}
              className={cx(
                "relative w-full overflow-hidden rounded-2xl border px-5 py-5 text-left transition-all duration-300",
                canAutoBackup
                  ? cx(
                      "hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-100",
                      mode.accentBorder,
                      mode.accentBgSoft,
                      mode.accentText,
                    )
                  : "cursor-not-allowed border-zinc-900 bg-black/30 text-zinc-500 grayscale",
              )}
            >
              {isAutoBackupRunning && (
                <span className="absolute inset-0 overflow-hidden rounded-2xl">
                  <span className="gw-copy-marquee" />
                </span>
              )}
              <div className="relative z-10 flex items-center justify-between gap-4">
                <div className="min-w-0">
                  <div className="flex items-center gap-3 text-lg font-black">
                    <span>{isAutoBackupRunning ? `${t.runningAutoBackup} ${autoBackupPhaseTitle}` : t.autoBackup}</span>
                    {canAutoBackup && !fullBackupDone && !isAutoBackupRunning && <span className="text-xl text-amber-300">!</span>}
                  </div>
                  <div className="mt-1 truncate text-xs text-zinc-500">
                    {isAutoBackupRunning
                      ? `${readingBackupKind === "spi" ? "SPI" : autoBackupPhaseTitle} • ${autoBackupStatusText}`
                      : canAutoBackup
                        ? backupReadReason.bankLocked
                          ? t.readSpiLocked
                          : t.readFullBackup
                        : backupReadReason.general}
                  </div>
                </div>
                <div className="shrink-0 text-3xl">{isAutoBackupRunning ? "…" : "⬇"}</div>
              </div>
            </button>

          </>
        )}

        {advancedBackupEnabled && (
          <div className="space-y-3 rounded-2xl border border-zinc-900 bg-black/20 p-3">
            <div className="text-[11px] font-black uppercase tracking-wide text-zinc-500">{t.advancedBackup}</div>
            {backupRows.map((row) => (
              <div key={row.key} className="grid grid-cols-[minmax(0,1fr)_260px] items-stretch gap-4">
                {renderFileTile(row)}
                {renderReadButton(row)}
              </div>
            ))}
          </div>
        )}
      </div>
    </Panel>
  );
}

function LiveLog({ mode, t, lines, onClear }) {
  return (
    <Panel className="gw-live-log p-5 h-[180px] flex flex-col">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-lg font-black">{t.liveLog}</h3>
        <button
          type="button"
          onClick={onClear}
          className={cx("rounded-xl border px-4 py-2 text-xs transition hover:bg-zinc-900", mode.accentBorder, mode.accentSoftText)}
        >
          {t.clear}
        </button>
      </div>
      <div className={cx("flex-1 overflow-auto rounded-2xl border border-zinc-900 bg-[#050505] p-4 font-mono text-xs leading-5 shadow-[inset_0_0_25px_rgba(255,255,255,0.02)]", mode.accentSoftText)}>
        {lines.map((line, index) => (
          <div key={`${index}-${line}`}>{line}</div>
        ))}
      </div>
    </Panel>
  );
}

export default function App() {
  const lastSpiLogRef = useRef({ percent: -1, speedText: "", message: "" });
  const handleGlobalDropRef = useRef(null);
  const lastNativeDropRef = useRef({ signature: "", at: 0 });
  const emulatorRomInputRef = useRef(null);
  const imagePackInputRef = useRef(null);
  const imageFolderInputRef = useRef(null);
  const manualImageInputRef = useRef(null);
  const flashProgressLogRef = useRef({ stage: "", bucket: -1 });
  const [modeName, setModeName] = useState("M");
  const [firmwareProfile, setFirmwareProfile] = useState("M");
  const [backupDone, setBackupDone] = useState(false);
  const [activeAction, setActiveAction] = useState("backup");
  const [isReadingInfo, setIsReadingInfo] = useState(false);
  const [readInfoProgress, setReadInfoProgress] = useState(0);
  const [selectedEmulator, setSelectedEmulator] = useState(null);
  const [selectedGameId, setSelectedGameId] = useState(null);
  const [pendingRomEmulator, setPendingRomEmulator] = useState(null);
  const [builderSettingsOpen, setBuilderSettingsOpen] = useState(false);
  const [isAutoBackupRunning, setIsAutoBackupRunning] = useState(false);
  const [isManualBackupRunning, setIsManualBackupRunning] = useState(false);
  const [readingBackupKind, setReadingBackupKind] = useState(null);
  const [backupProgressState, setBackupProgressState] = useState({
    totalProgress: 0,
    mcuProgress: 0,
    spiProgress: 0,
    speedBps: 0,
    frequency: 0,
    phase: "idle",
    backend: "",
    message: "",
  });
  const [flashProgress, setFlashProgress] = useState(0);
  const [isFlashing, setIsFlashing] = useState(false);
  const [flashOperation, setFlashOperation] = useState(null);
  const [restoreDisplayActive, setRestoreDisplayActive] = useState(null);
  const [activeFlashPhase, setActiveFlashPhase] = useState(null);
  const [flashPhaseLabel, setFlashPhaseLabel] = useState("");
  const [flashStage, setFlashStage] = useState("");
  const [flashDisplayRows, setFlashDisplayRows] = useState([]);
  const [flashCompletionStatus, setFlashCompletionStatus] = useState(null);
  const [flashResult, setFlashResult] = useState({
    bank1: null,
    bank2: null,
    spi: null,
  });
  const [mcuFirmwareFile, setMcuFirmwareFile] = useState(null);
  const [bank2FirmwareFile, setBank2FirmwareFile] = useState(null);
  const [spiFirmwareFile, setSpiFirmwareFile] = useState(null);
  const [mcuFirmwarePath, setMcuFirmwarePath] = useState(null);
  const [bank2FirmwarePath, setBank2FirmwarePath] = useState(null);
  const [spiFirmwarePath, setSpiFirmwarePath] = useState(null);
  const [mcuBackupFile, setMcuBackupFile] = useState(null);
  const [mcuBackupPath, setMcuBackupPath] = useState(null);
  const [bank2BackupFile, setBank2BackupFile] = useState(null);
  const [bank2BackupPath, setBank2BackupPath] = useState(null);
  const [spiBackupFile, setSpiBackupFile] = useState(null);
  const [spiBackupPath, setSpiBackupPath] = useState(null);
  const [stockMcuBackupFile, setStockMcuBackupFile] = useState(null);
  const [stockMcuBackupPath, setStockMcuBackupPath] = useState(null);
  const [stockSpiBackupFile, setStockSpiBackupFile] = useState(null);
  const [stockSpiBackupPath, setStockSpiBackupPath] = useState(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [originalFirmwarePickerOpen, setOriginalFirmwarePickerOpen] = useState(false);
  const [confirmAction, setConfirmAction] = useState(null);
  const [msxBiosPrompt, setMsxBiosPrompt] = useState(null);
  const [colecoBiosPrompt, setColecoBiosPrompt] = useState(null);
  const [stockFirmwarePrompt, setStockFirmwarePrompt] = useState(false);
  const [pendingStockRestoreProfile, setPendingStockRestoreProfile] = useState(null);
  const [retryBuildAfterMsxBios, setRetryBuildAfterMsxBios] = useState(false);
  const [retryBuildAfterColecoBios, setRetryBuildAfterColecoBios] = useState(false);
  const [retryBuildAfterStockFirmware, setRetryBuildAfterStockFirmware] = useState(false);
  const [advancedFlasherEnabled, setAdvancedFlasherEnabled] = useState(false);
  const [recoveryPromptAnswered, setRecoveryPromptAnswered] = useState(false);
  const [recoveryMode, setRecoveryMode] = useState(false);
  const [nativeDragDropAvailable, setNativeDragDropAvailable] = useState(false);
  const isDeviceTransferActive = isAutoBackupRunning || isManualBackupRunning || isFlashing;
  const isStmBusy = isReadingInfo || isDeviceTransferActive;
  const [language, setLanguage] = useState(() => detectInitialLanguage());
  const [probeFrequencyKhz, setProbeFrequencyKhz] = useState(8000);
  const [readInfoIssue, setReadInfoIssue] = useState("");
  const [deviceInfo, setDeviceInfo] = useState({
    programmer: "UNKNOWN",
    probe_vendor: "UNKNOWN",
    probe_id: "UNKNOWN",
    device_uid: "UNKNOWN",
    cpu_id: "UNKNOWN",
    target_voltage: "UNKNOWN",
    mcu_profile: "UNKNOWN",
    detected_firmware: "UNKNOWN",
    external_flash: "UNKNOWN",
    protection: "UNKNOWN",
    filesystem: "UNKNOWN",
    summary: "",
  });
  const [workspaceRoot, setWorkspaceRoot] = useState("GameWatchBuilderData");
  const [thumbnailsRoot, setThumbnailsRoot] = useState("content\\image");
  const [screenFrames, setScreenFrames] = useState(() => ({
    M: { ...MODES.M.screenFrame },
    Z: { ...MODES.Z.screenFrame },
  }));
  const [logs, setLogs] = useState([
    "[bootstrap] GW Studio React/Tauri shell loaded",
    "[info] Rust commands will replace PySide handlers here",
  ]);
  const [startupLoading, setStartupLoading] = useState(true);
  const [appSha256, setAppSha256] = useState("");
  const [startupProgress, setStartupProgress] = useState(0);
  const [startupMessage, setStartupMessage] = useState("Preparing runtime");
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [updateInfo, setUpdateInfo] = useState(null);
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);
  const startupUpdateCheckedRef = useRef(false);
  const [coverflowImagesEnabled, setCoverflowImagesEnabled] = useState(false);
  const [isDownloadingImages, setIsDownloadingImages] = useState(false);
  const [imageDownloadProgress, setImageDownloadProgress] = useState(0);
  const [romLibrary, setRomLibrary] = useState({});
  const [isDragActive, setIsDragActive] = useState(false);
  const [gameSortMode, setGameSortMode] = useState("title-asc");
  const [pendingManualImageGameId, setPendingManualImageGameId] = useState(null);
  const [buildMetrics, setBuildMetrics] = useState({
    romBytes: 0,
    imageBytes: 0,
    romCount: 0,
    imageCount: 0,
  });
  const [isBuildingFirmware, setIsBuildingFirmware] = useState(false);
  const [buildFirmwareProgress, setBuildFirmwareProgress] = useState(0);
  const [buildFirmwareMessage, setBuildFirmwareMessage] = useState("");
  const [buildFirmwareError, setBuildFirmwareError] = useState("");
  const [builtExtflashBytes, setBuiltExtflashBytes] = useState(0);
  const [builtExtflashSignature, setBuiltExtflashSignature] = useState("");

  const t = TRANSLATIONS[language] || TRANSLATIONS.ru;
  const sdEnabled = false;
  const detectedFirmwareLabel = formatFirmwareLabel(deviceInfo.detected_firmware);
  const normalizedDetectedFirmware = normalizeFirmwareAlias(deviceInfo.detected_firmware);
  const detectedExternalFlashValue = Number.parseFloat(String(deviceInfo.external_flash ?? "").replace(/[^\d.]/g, ""));
  const firmwareBaseMb = firmwareProfile === "Z" ? 4 : 1;
  const firmwareBaseBytes = Math.round(firmwareBaseMb * 1024 * 1024);
  const spiTotalMb = Number.isFinite(detectedExternalFlashValue) && detectedExternalFlashValue > 0
    ? detectedExternalFlashValue
    : 0;

  const hasReadDeviceInfo = !isUnknownValue(deviceInfo.detected_firmware) || !isUnknownValue(deviceInfo.external_flash);
  const hasVisualFirmwareProfile = Boolean(normalizedDetectedFirmware);
  const visualModeName = hasVisualFirmwareProfile ? normalizedDetectedFirmware : null;
  const mode = visualModeName ? MODES[visualModeName] : NEUTRAL_MODE;
  const marqueeRgb = visualModeName === "Z" ? "52, 255, 176" : visualModeName === "M" ? "255, 120, 120" : "148, 163, 184";
  const isZMode = visualModeName === "Z";
  const isZFirmware = firmwareProfile === "Z";
  const firmwareMode = MODES[firmwareProfile];
  const currentSceneTuning = mode.sceneTuning;
  const currentScreenFrame = visualModeName ? (screenFrames[visualModeName] ?? mode.screenFrame) : mode.screenFrame;
  const probeFrequencyHz = probeFrequencyKhz * 1000;
  const hasKnownDeviceUid = isValidDeviceUid(deviceInfo.device_uid);
  const deviceUidIssue = deviceUidErrorText(deviceInfo.device_uid);
  const hasKnownFirmwareProfile = Boolean(normalizedDetectedFirmware);
  const deviceIdentifyIssue = !hasReadDeviceInfo
    ? "Read Device Info first"
    : !hasKnownDeviceUid
      ? deviceUidIssue
    : !hasKnownFirmwareProfile
      ? "Detected firmware is UNKNOWN"
      : "";
  const isDeviceIdentified = hasKnownDeviceUid && hasKnownFirmwareProfile;
  const isMcuDetected = !isUnknownValue(deviceInfo.mcu_profile);
  const recoveryCandidate = hasKnownDeviceUid && isMcuDetected && !hasKnownFirmwareProfile;
  const showRecoveryPrompt = recoveryCandidate && !recoveryPromptAnswered && !recoveryMode;
  const isDeviceLocked = deviceInfo.protection === "LOCKED";
  const stockBackupFirmware = normalizedDetectedFirmware ?? firmwareProfile;
  const stockMcuReady = Boolean(stockMcuBackupFile && stockMcuBackupPath);
  const stockSpiReady = Boolean(stockSpiBackupFile && stockSpiBackupPath);
  const stockBackupReady = stockMcuReady && stockSpiReady;
  const stockBackupGate = hasReadDeviceInfo && hasKnownFirmwareProfile
    ? {
        ready: stockBackupReady,
        canImport: hasKnownFirmwareProfile,
        firmware: stockBackupFirmware,
        mcuReady: stockMcuReady,
        spiReady: stockSpiReady,
        mcuName: stockMcuBackupFile,
        spiName: stockSpiBackupFile,
        message: t.stockFirmwareRequiredMessage.replace("{firmware}", stockBackupFirmware),
      }
    : null;
  const stockFirmwarePromptProfile = pendingStockRestoreProfile ?? stockBackupFirmware;
  const stockFirmwarePromptGate = stockFirmwarePrompt
    ? {
        ready: Boolean(stockMcuBackupFile && stockMcuBackupPath && stockSpiBackupFile && stockSpiBackupPath),
        canImport: Boolean(stockFirmwarePromptProfile),
        firmware: stockFirmwarePromptProfile,
        mcuReady: Boolean(stockMcuBackupFile && stockMcuBackupPath),
        spiReady: Boolean(stockSpiBackupFile && stockSpiBackupPath),
        mcuName: stockMcuBackupFile,
        spiName: stockSpiBackupFile,
        message: t.stockFirmwareRequiredMessage.replace("{firmware}", stockFirmwarePromptProfile ?? "?"),
      }
    : stockBackupGate;
  const backupDirectoryHint = hasKnownDeviceUid
    ? `${workspaceRoot}\\backups\\${deviceInfo.device_uid}`
    : `${workspaceRoot}\\backups\\<read device info first>`;
  const isDeviceWritable = hasReadDeviceInfo && deviceInfo.protection === "UNLOCKED";
  const isRecoveryWritable = recoveryMode && hasKnownDeviceUid && isDeviceWritable;
  const canAutoBackup = hasReadDeviceInfo && hasKnownDeviceUid && hasKnownFirmwareProfile;
  const backupReadReasonText = !hasReadDeviceInfo
    ? "Read Device Info first"
    : !hasKnownDeviceUid
      ? deviceUidIssue
      : !hasKnownFirmwareProfile
        ? "Detected firmware is UNKNOWN"
      : "Ready";
  const backupReadReason = {
    general: backupReadReasonText,
    bankLocked: isDeviceLocked,
  };
  const canSelectBackupFiles = hasKnownDeviceUid;
  const fullBackupDone = isDeviceLocked
    ? Boolean(spiBackupFile)
    : Boolean(mcuBackupFile && bank2BackupFile && spiBackupFile);

  const loadedGames = useMemo(() => dedupeGames(Object.values(romLibrary).flat()), [romLibrary]);
  const selectedEmulatorGames = useMemo(
    () => (selectedEmulator ? dedupeGames(romLibrary[selectedEmulator] ?? []) : []),
    [romLibrary, selectedEmulator],
  );
  const imageCapableGames = useMemo(
    () => loadedGames.filter((game) => game.emulatorId && THUMBNAIL_SOURCES[game.emulatorId]),
    [loadedGames],
  );
  const canDownloadImages = imageCapableGames.length > 0;
  const sortedLoadedGames = useMemo(() => {
    const next = [...loadedGames];
    if (gameSortMode === "title-asc") {
      next.sort((a, b) => String(a.title ?? "").localeCompare(String(b.title ?? ""), undefined, { sensitivity: "base" }));
    } else if (gameSortMode === "title-desc") {
      next.sort((a, b) => String(b.title ?? "").localeCompare(String(a.title ?? ""), undefined, { sensitivity: "base" }));
    } else if (gameSortMode === "size-asc") {
      next.sort((a, b) => Number(a.sizeBytes ?? 0) - Number(b.sizeBytes ?? 0));
    } else if (gameSortMode === "size-desc") {
      next.sort((a, b) => Number(b.sizeBytes ?? 0) - Number(a.sizeBytes ?? 0));
    }
    return next;
  }, [loadedGames, gameSortMode]);
  const romBuildSignature = useMemo(
    () => [
      `coverflow=${coverflowImagesEnabled ? "1" : "0"}`,
      ...loadedGames
        .map((game) => `${game.emulatorId ?? "?"}|${game.path ?? game.title ?? game.id}`)
        .sort(),
    ].join("||"),
    [loadedGames, coverflowImagesEnabled],
  );
  const romUsedMb = bytesToMb(loadedGames.reduce((sum, game) => sum + Number(game.sizeBytes ?? 0), 0));
  const imageUsedMb = bytesToMb(buildMetrics.imageBytes);
  const estimatedEmulatorUsedMb = romUsedMb + imageUsedMb;
  const builtExtflashMb = bytesToMb(builtExtflashBytes);
  const hasCurrentBuiltExtflash = builtExtflashBytes > 0 && builtExtflashSignature === romBuildSignature;
  const emulatorUsedMb = hasCurrentBuiltExtflash ? builtExtflashMb : estimatedEmulatorUsedMb;
  const spiUsedMb = firmwareBaseMb + emulatorUsedMb;
  const sdTotalMb = 8192;
  const sdUsedMb = selectedEmulator ? 1240 : 0;

  const selectedGame = sortedLoadedGames.find((game) => game.id === selectedGameId);

  async function openExternalUrl(url) {
    try {
      await safeInvoke("open_external_url", { request: { url } });
    } catch (error) {
      setLogs((items) => [...items, `[app] Open URL failed: ${String(error?.message ?? error)}`]);
    }
  }

  async function installAppUpdate(info) {
    if (!info?.downloadUrl || isInstallingUpdate) {
      return;
    }
    setIsInstallingUpdate(true);
    setLogs((items) => [...items, `[update] Installing GW Studio ${info.version}`]);
    try {
      await safeInvoke("install_app_update", {
        request: {
          download_url: info.downloadUrl,
          expected_sha256: info.sha256 || null,
          version: info.version,
        },
      });
    } catch (error) {
      setIsInstallingUpdate(false);
      setLogs((items) => [...items, `[update] ERROR: ${String(error?.message ?? error)}`]);
    }
  }

  function promptInstallAppUpdate(info) {
    if (!info?.downloadUrl) {
      return;
    }
    setConfirmAction({
      title: t.updateConfirmTitle.replace("{version}", info.version),
      message: t.updateConfirmMessage.replace("{current}", APP_VERSION).replace("{version}", info.version),
      confirmText: t.updateNow,
      cancelText: t.no,
      tone: "emerald",
      onConfirm: () => {
        setConfirmAction(null);
        installAppUpdate(info);
      },
      onCancel: () => setConfirmAction(null),
    });
  }

  async function checkForAppUpdate({ interactive = false } = {}) {
    if (isCheckingUpdate || isInstallingUpdate) {
      return;
    }
    setIsCheckingUpdate(true);
    try {
      const response = await fetch(GITHUB_LATEST_RELEASE_API, {
        headers: {
          Accept: "application/vnd.github+json",
        },
      });
      if (!response.ok) {
        throw new Error(`GitHub API ${response.status}`);
      }
      const release = await response.json();
      const latestVersion = String(release.tag_name || release.name || "").replace(/^v/i, "");
      const assets = Array.isArray(release.assets) ? release.assets : [];
      const exeAsset = assets.find((asset) => String(asset.name ?? "").toLowerCase() === "gw studio.exe")
        ?? assets.find((asset) => String(asset.name ?? "").toLowerCase().endsWith(".exe"));
      const shaAsset = assets.find((asset) => String(asset.name ?? "").toLowerCase().endsWith(".sha256"));

      if (!latestVersion || !exeAsset?.browser_download_url) {
        throw new Error("latest release does not contain GW Studio exe asset");
      }

      let expectedSha = "";
      if (shaAsset?.browser_download_url) {
        try {
          const shaResponse = await fetch(shaAsset.browser_download_url);
          if (shaResponse.ok) {
            expectedSha = parseSha256Text(await shaResponse.text());
          }
        } catch {
          expectedSha = "";
        }
      }

      const latestInfo = {
        version: latestVersion,
        downloadUrl: exeAsset.browser_download_url,
        sha256: expectedSha,
        releaseUrl: release.html_url || GITHUB_REPOSITORY_URL,
      };
      const isNewer = compareVersions(latestVersion, APP_VERSION) > 0;
      setUpdateAvailable(isNewer);
      setUpdateInfo(isNewer ? latestInfo : null);
      setLogs((items) => [
        ...items,
        isNewer
          ? `[update] Available: ${APP_VERSION} -> ${latestVersion}`
          : `[update] Current version is up to date (${APP_VERSION})`,
      ]);

      if (isNewer) {
        promptInstallAppUpdate(latestInfo);
      } else if (interactive) {
        setConfirmAction({
          title: t.programUpdate,
          message: t.upToDate,
          confirmText: t.ok,
          cancelText: t.close,
          tone: "emerald",
          onConfirm: () => setConfirmAction(null),
          onCancel: () => setConfirmAction(null),
        });
      }
    } catch (error) {
      setLogs((items) => [...items, `[update] Check failed: ${String(error?.message ?? error)}`]);
      if (interactive) {
        setConfirmAction({
          title: t.programUpdate,
          message: t.updateCheckFailed,
          confirmText: t.ok,
          cancelText: t.close,
          tone: "amber",
          onConfirm: () => setConfirmAction(null),
          onCancel: () => setConfirmAction(null),
        });
      }
    } finally {
      setIsCheckingUpdate(false);
    }
  }

  useEffect(() => {
    let cancelled = false;

    async function refreshBuildMetrics() {
      if (loadedGames.length === 0) {
        if (!cancelled) {
          setBuildMetrics({
            romBytes: 0,
            imageBytes: 0,
            romCount: 0,
            imageCount: 0,
          });
        }
        return;
      }

      try {
        const groupedByEmulator = loadedGames.reduce((groups, game) => {
          const key = game.emulatorId ?? "unknown";
          if (!groups[key]) {
            groups[key] = [];
          }
          groups[key].push(game);
          return groups;
        }, {});
        const metricChunks = await Promise.all(
          Object.entries(groupedByEmulator).map(async ([emulatorId, games]) =>
            safeInvoke("compute_build_metrics", {
              request: {
                emulator: emulatorId,
                titles: games.map((game) => game.title),
                rom_paths: games.map((game) => game.path).filter(Boolean),
              },
            }),
          ),
        );
        if (cancelled) {
          return;
        }
        setBuildMetrics({
          romBytes: loadedGames.reduce((sum, game) => sum + Number(game.sizeBytes ?? 0), 0),
          imageBytes: metricChunks.reduce((sum, metrics) => sum + Number(metrics?.image_bytes ?? 0), 0),
          romCount: loadedGames.length,
          imageCount: metricChunks.reduce((sum, metrics) => sum + Number(metrics?.image_count ?? 0), 0),
        });
      } catch (error) {
        if (cancelled) {
          return;
        }
        setBuildMetrics({
          romBytes: loadedGames.reduce((sum, game) => sum + Number(game.sizeBytes ?? 0), 0),
          imageBytes: 0,
          romCount: loadedGames.length,
          imageCount: 0,
        });
        setLogs((items) => [...items, `[build] Metrics error: ${String(error)}`]);
      }
    }

    refreshBuildMetrics();
    return () => {
      cancelled = true;
    };
  }, [loadedGames]);

  useEffect(() => {
    function onWindowKeyDown(event) {
      if (sortedLoadedGames.length === 0) {
        return;
      }
      const target = event.target;
      const tagName = target?.tagName?.toUpperCase?.() ?? "";
      const isTypingTarget =
        tagName === "INPUT" ||
        tagName === "TEXTAREA" ||
        tagName === "SELECT" ||
        target?.isContentEditable;
      if (isTypingTarget) {
        return;
      }

      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") {
        if (event.key !== "Delete") {
          return;
        }
        if (!selectedGameId) {
          return;
        }
        event.preventDefault();
        handleRemoveGame(selectedGameId);
        return;
      }

      event.preventDefault();
      const currentIndex = sortedLoadedGames.findIndex((game) => game.id === selectedGameId);
      const safeIndex = currentIndex >= 0 ? currentIndex : 0;
      const nextIndex =
        event.key === "ArrowDown"
          ? Math.min(sortedLoadedGames.length - 1, safeIndex + 1)
          : Math.max(0, safeIndex - 1);
      const nextGame = sortedLoadedGames[nextIndex];
      if (nextGame && nextGame.id !== selectedGameId) {
        setSelectedGameId(nextGame.id);
      }
    }

    window.addEventListener("keydown", onWindowKeyDown);
    return () => window.removeEventListener("keydown", onWindowKeyDown);
  }, [sortedLoadedGames, selectedGameId]);

  useEffect(() => {
    let cancelled = false;
    let unlistenRuntimeReady = null;
    let unlistenRuntimeProgress = null;
    let runtimeReadyHandled = false;

    async function finishStartupStatus(runtimeMessage = "") {
      const startedAt = Date.now();
      const [status, sha] = await Promise.all([
        safeInvoke("runtime_status", {}, () => ({
          workspace_root: "GameWatchBuilderData",
          logs_dir: "GameWatchBuilderData/logs",
          tools_dir: "external",
          host_root: ".",
          gnwmanager_source: "python-module",
          rust_backend: "fallback",
        })),
        safeInvoke("app_sha256", {}, () => ""),
      ]);

        if (cancelled) {
          return;
        }
        setAppSha256(sha || "");
        setLogs((items) => [
          ...items,
          `[app] version=${APP_VERSION}`,
          ...(sha ? [`[app] sha256=${sha}`] : []),
          ...(runtimeMessage ? [`[runtime] ${runtimeMessage}`] : []),
          `[runtime] workspace=${status.workspace_root}`,
          `[runtime] backend=${status.rust_backend}, gnwmanager=${status.gnwmanager_source}`,
        ]);
        setWorkspaceRoot(status.workspace_root ?? "GameWatchBuilderData");
        setThumbnailsRoot(status.thumbnails_dir ?? "content\\image");
        const remainingMs = Math.max(0, 900 - (Date.now() - startedAt));
        window.setTimeout(() => {
          if (!cancelled) {
            setStartupLoading(false);
          }
        }, remainingMs);
    }

    async function attachRuntimeListener() {
      unlistenRuntimeProgress = await listen("portable-runtime-progress", (event) => {
        const payload = event.payload ?? {};
        setStartupProgress(Number(payload.progress ?? 0));
        setStartupMessage(payload.message ?? payload.asset ?? "Preparing runtime");
      });
      unlistenRuntimeReady = await listen("portable-runtime-ready", (event) => {
        runtimeReadyHandled = true;
        const payload = event.payload ?? {};
        if (!payload.ok) {
          setLogs((items) => [...items, `[runtime] ${payload.message ?? "portable runtime failed"}`]);
          setStartupLoading(false);
          return;
        }
        setStartupProgress(100);
        setStartupMessage(payload.message ?? "portable runtime ready");
        finishStartupStatus(payload.message ?? "").catch((error) => {
          if (!cancelled) {
            setLogs((items) => [...items, `[runtime] ${String(error)}`]);
            setStartupLoading(false);
          }
        });
      });
    }

    safeInvoke("app_sha256", {}, () => "")
      .then((sha) => {
        if (!cancelled && sha) {
          setAppSha256(sha);
        }
      })
      .catch(() => {});

    attachRuntimeListener().catch((error) => {
        if (!cancelled) {
          setLogs((items) => [...items, `[runtime] ${String(error)}`]);
          setStartupLoading(false);
        }
      });
    const fallbackTimer = window.setTimeout(() => {
      if (!cancelled && !runtimeReadyHandled) {
        finishStartupStatus("portable runtime status fallback").catch((error) => {
          if (!cancelled) {
            setLogs((items) => [...items, `[runtime] ${String(error)}`]);
            setStartupLoading(false);
          }
        });
      }
    }, 60000);

    return () => {
      cancelled = true;
      window.clearTimeout(fallbackTimer);
      if (typeof unlistenRuntimeReady === "function") {
        unlistenRuntimeReady();
      }
      if (typeof unlistenRuntimeProgress === "function") {
        unlistenRuntimeProgress();
      }
    };
  }, []);

  useEffect(() => {
    if (startupLoading || startupUpdateCheckedRef.current) {
      return;
    }
    startupUpdateCheckedRef.current = true;
    checkForAppUpdate({ interactive: false });
  }, [startupLoading]);

  useEffect(() => {
    if (normalizedDetectedFirmware) {
      setModeName(normalizedDetectedFirmware);
      setFirmwareProfile(normalizedDetectedFirmware);
    }
  }, [normalizedDetectedFirmware]);

  useEffect(() => {
    if (!hasKnownFirmwareProfile) {
      setStockMcuBackupFile(null);
      setStockMcuBackupPath(null);
      setStockSpiBackupFile(null);
      setStockSpiBackupPath(null);
      return undefined;
    }

    let cancelled = false;
    safeInvoke("lookup_stock_backups", {
      request: { device_uid: deviceInfo.device_uid, firmware_profile: stockBackupFirmware },
    })
      .then((stockLookup) => {
        if (cancelled) {
          return;
        }
        setStockMcuBackupFile(stockLookup.mcu_name || null);
        setStockMcuBackupPath(stockLookup.mcu_path || null);
        setStockSpiBackupFile(stockLookup.spi_name || null);
        setStockSpiBackupPath(stockLookup.spi_path || null);
        setLogs((items) => [
          ...items,
          `[stock] Auto lookup ${stockBackupFirmware}: BANK1=${stockLookup.mcu_name || "missing"}, SPI=${stockLookup.spi_name || "missing"}`,
        ]);
      })
      .catch((error) => {
        if (!cancelled) {
          setLogs((items) => [...items, `[stock] Auto lookup failed: ${String(error)}`]);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [deviceInfo.device_uid, hasKnownFirmwareProfile, stockBackupFirmware]);

  useEffect(() => {
    if (isDeviceIdentified || !recoveryCandidate) {
      setRecoveryMode(false);
      setRecoveryPromptAnswered(false);
    }
  }, [isDeviceIdentified, recoveryCandidate]);

  useEffect(() => {
    if (!isReadingInfo) {
      setReadInfoProgress(0);
      return undefined;
    }

    setReadInfoProgress(8);
    const timer = window.setInterval(() => {
      setReadInfoProgress((value) => {
        if (value >= 92) {
          return value;
        }
        const step = value < 40 ? 9 : value < 70 ? 6 : 3;
        return Math.min(92, value + step);
      });
    }, 180);

    return () => window.clearInterval(timer);
  }, [isReadingInfo]);

  useEffect(() => {
    let unlisten;
    listen("backup-progress", (event) => {
      const payload = event.payload ?? {};
      const phase = payload.phase ?? "idle";
      const phaseProgress = Math.round(payload.phase_progress ?? 0);
      const speedText =
        payload.speed_bps > 0
          ? `${(payload.speed_bps / 1024 / 1024).toFixed(2)} MB/s`
          : "—";
      const frequencyText = payload.frequency ? `${Math.round(payload.frequency / 1000)} kHz` : "—";
      const message = payload.message ?? "";
      setBackupProgressState((prev) => ({
        totalProgress: payload.total_progress ?? prev.totalProgress ?? 0,
        mcuProgress: phase === "mcu" ? payload.phase_progress ?? 0 : prev.mcuProgress ?? 0,
        spiProgress: phase === "spi" ? payload.phase_progress ?? 0 : prev.spiProgress ?? 0,
        speedBps: payload.speed_bps ?? 0,
        frequency: payload.frequency ?? 0,
        phase: phase ?? prev.phase ?? "idle",
        backend: payload.backend ?? prev.backend ?? "",
        message: message ?? prev.message ?? "",
      }));
      if (phase === "spi") {
        const previous = lastSpiLogRef.current;
        const hasRealSpeed = payload.speed_bps > 0;
        const shouldLogPercent = phaseProgress > 0 && hasRealSpeed && phaseProgress !== previous.percent;
        const shouldLogStage =
          phaseProgress === 0 &&
          message &&
          message !== previous.message &&
          (message.includes("probe") || message.includes("Trying") || message.includes("finished"));

        if (shouldLogPercent || shouldLogStage) {
          lastSpiLogRef.current = { percent: phaseProgress, speedText, message };
          setLogs((items) => [
            ...items,
            shouldLogPercent
              ? `[backup] SPI ${phaseProgress}% • ${speedText} • ${frequencyText}`
              : `[backup] SPI ${message} • ${frequencyText}`,
          ]);
        }
      }
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [t]);

  useEffect(() => {
    let unlisten;
    listen("backup-debug", (event) => {
      const payload = event.payload ?? {};
      if (payload.phase !== "spi" || !payload.line) {
        return;
      }
      setLogs((items) => [...items, `[spi-raw:${payload.source ?? "stream"}] ${payload.line}`]);
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    let unlisten;
    listen("firmware-write-progress", (event) => {
      const payload = event.payload ?? {};
      const phase = payload.phase ?? "spi";
      const stage = payload.stage ?? "write";
      const progress = Math.round(Number(payload.progress ?? 0));
      if (phase === "bank1" || phase === "bank2" || phase === "spi") {
        setActiveFlashPhase(phase);
        setFlashOperation((current) =>
          current === "auto" || current === "restore" || current === "stock-restore" ? current : phase,
        );
      }
      const label =
        phase === "spi"
          ? stage === "prepare"
            ? t.prepareSpiFlash
            : stage === "erase"
            ? t.eraseSpiFlash
            : stage === "verify"
            ? t.verifySpiFlash
            : t.writeSpiFlashProgress
          : phase === "bank2"
            ? t.writeBank2Progress
            : t.writeBank1Progress;
      setFlashStage(stage);
      setFlashProgress(Number.isFinite(progress) ? Math.max(0, Math.min(100, progress)) : 0);
      setFlashPhaseLabel(progress > 0 ? `${label} ${progress}%` : stage === "erase" ? `${label}...` : label);
      if (phase === "spi" && stage === "verify") {
        const bucket = progress >= 100 ? 100 : Math.floor(progress / 25) * 25;
        const last = flashProgressLogRef.current;
        if (last.stage !== stage || last.bucket !== bucket) {
          flashProgressLogRef.current = { stage, bucket };
          setLogs((items) => [
            ...items,
            bucket >= 100 ? "[flash] Verify SPI Flash completed" : `[flash] Verify SPI Flash ${Math.max(0, bucket)}%`,
          ]);
        }
      } else if (flashProgressLogRef.current.stage !== stage) {
        flashProgressLogRef.current = { stage, bucket: -1 };
      }
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    let unlisten;
    listen("build-progress", (event) => {
      const payload = event.payload ?? {};
      const progress = Number(payload.progress ?? 0);
      const message = String(payload.message ?? "");
      setBuildFirmwareProgress(Number.isFinite(progress) ? Math.max(0, Math.min(100, progress)) : 0);
      setBuildFirmwareMessage(message);
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    setBackupDone(fullBackupDone);
  }, [fullBackupDone]);

  async function handleReadInfo() {
    setIsReadingInfo(true);
    setReadInfoProgress(8);
    setActiveAction("readInfo");
    setReadInfoIssue("");
    setLogs((items) => [...items, "[device] Read Info button pressed", "[device] Reading probe and firmware info..."]);
    try {
      const result = await safeInvoke("read_device_info", { backend: "pyocd", frequency: probeFrequencyHz });
      setReadInfoProgress(100);
      setDeviceInfo(result);
      const detectedMainboard = normalizeFirmwareAlias(result.detected_firmware) ?? normalizeFirmwareAlias(result.mcu_profile);
      const validResultDeviceUid = isValidDeviceUid(result.device_uid);
      if (isDeviceReadEmpty(result)) {
        setReadInfoIssue("Устройство не определилось. Проверьте подключение ST-LINK и питание консоли, нажмите кнопку включения на консоли и повторите Read Device Info.");
      } else if (!validResultDeviceUid) {
        setReadInfoIssue(deviceUidErrorText(result.device_uid));
      }
      if (validResultDeviceUid) {
        try {
          const backupLookup = await safeInvoke("lookup_device_backups", {
            request: { device_uid: result.device_uid },
          });
          if (backupLookup.mcu_name && backupLookup.mcu_path) {
            setMcuBackupFile(backupLookup.mcu_name);
            setMcuBackupPath(backupLookup.mcu_path);
          } else {
            setMcuBackupFile(null);
            setMcuBackupPath(null);
          }
          if (backupLookup.bank2_name && backupLookup.bank2_path) {
            setBank2BackupFile(backupLookup.bank2_name);
            setBank2BackupPath(backupLookup.bank2_path);
          } else {
            setBank2BackupFile(null);
            setBank2BackupPath(null);
          }
          if (backupLookup.spi_name && backupLookup.spi_path) {
            setSpiBackupFile(backupLookup.spi_name);
            setSpiBackupPath(backupLookup.spi_path);
          } else {
            setSpiBackupFile(null);
            setSpiBackupPath(null);
          }
          if (backupLookup.mcu_name || backupLookup.bank2_name || backupLookup.spi_name) {
            setBackupDone(result.protection === "LOCKED"
              ? Boolean(backupLookup.spi_name)
              : Boolean(backupLookup.mcu_name && backupLookup.bank2_name && backupLookup.spi_name));
            setLogs((items) => [
              ...items,
              `[backup] Auto-loaded backups for UID ${result.device_uid}`,
              ...(backupLookup.mcu_name ? [`[backup] BANK1=${backupLookup.mcu_name}`] : []),
              ...(backupLookup.bank2_name ? [`[backup] BANK2=${backupLookup.bank2_name}`] : []),
              ...(backupLookup.spi_name ? [`[backup] SPI=${backupLookup.spi_name}`] : []),
            ]);
          }
          const stockLookup = await safeInvoke("lookup_stock_backups", {
            request: { device_uid: result.device_uid, firmware_profile: detectedMainboard ?? firmwareProfile },
          });
          if (stockLookup.mcu_name && stockLookup.mcu_path) {
            setStockMcuBackupFile(stockLookup.mcu_name);
            setStockMcuBackupPath(stockLookup.mcu_path);
          } else {
            setStockMcuBackupFile(null);
            setStockMcuBackupPath(null);
          }
          if (stockLookup.spi_name && stockLookup.spi_path) {
            setStockSpiBackupFile(stockLookup.spi_name);
            setStockSpiBackupPath(stockLookup.spi_path);
          } else {
            setStockSpiBackupFile(null);
            setStockSpiBackupPath(null);
          }
        } catch (lookupError) {
          setLogs((items) => [...items, `[backup] Auto-load lookup failed: ${String(lookupError)}`]);
        }
      } else {
        setMcuBackupFile(null);
        setMcuBackupPath(null);
        setBank2BackupFile(null);
        setBank2BackupPath(null);
          setSpiBackupFile(null);
          setSpiBackupPath(null);
          setStockMcuBackupFile(null);
          setStockMcuBackupPath(null);
          setStockSpiBackupFile(null);
          setStockSpiBackupPath(null);
      }
      setLogs((items) => [
        ...items,
        `[device] ${result.summary ?? "Device info updated"}`,
        `[device] cpu=${result.cpu_id ?? "UNKNOWN"}, voltage=${result.target_voltage ?? "UNKNOWN"}`,
        `[device] firmware=${result.detected_firmware ?? "UNKNOWN"}, extflash=${result.external_flash ?? "UNKNOWN"}, protection=${result.protection ?? "UNKNOWN"}`,
        ...(detectedMainboard ? [`[device] mainboard=${detectedMainboard} (derived from firmware)`] : []),
        `[device] uid=${result.device_uid ?? "UNKNOWN"}`,
      ]);
    } catch (error) {
      const message = String(error?.message ?? error);
      setReadInfoProgress(100);
      setReadInfoIssue("Не удалось прочитать устройство. Проверьте подключение ST-LINK и питание консоли, нажмите кнопку включения на консоли и повторите Read Device Info.");
      setLogs((items) => [...items, `[device] ERROR: ${message}`]);
    } finally {
      window.setTimeout(() => {
        setIsReadingInfo(false);
      }, 180);
    }
  }

  async function handleStartRecovery() {
    setRecoveryPromptAnswered(true);
    setRecoveryMode(true);
    setBuilderSettingsOpen(false);
    setSelectedEmulator(null);
    setSelectedGameId(null);
    setActiveAction("flash");
    setAdvancedFlashEnabled(true);
    setLogs((items) => [
      ...items,
      "[recovery] User confirmed firmware recovery mode",
      `[recovery] UID=${deviceInfo.device_uid}`,
      "[recovery] Looking for saved restore backup...",
    ]);

    try {
      const restoreLookup = await safeInvoke("lookup_restore_backups", {
        request: { device_uid: deviceInfo.device_uid },
      });
      const hasBank1 = Boolean(restoreLookup.mcu_name && restoreLookup.mcu_path);
      const hasBank2 = Boolean(restoreLookup.bank2_name && restoreLookup.bank2_path);
      const hasSpi = Boolean(restoreLookup.spi_name && restoreLookup.spi_path);

      if (hasBank1) {
        setMcuBackupFile(restoreLookup.mcu_name);
        setMcuBackupPath(restoreLookup.mcu_path);
        setMcuFirmwareFile(restoreLookup.mcu_name);
        setMcuFirmwarePath(restoreLookup.mcu_path);
      }
      if (hasBank2) {
        setBank2BackupFile(restoreLookup.bank2_name);
        setBank2BackupPath(restoreLookup.bank2_path);
        setBank2FirmwareFile(restoreLookup.bank2_name);
        setBank2FirmwarePath(restoreLookup.bank2_path);
      }
      if (hasSpi) {
        setSpiBackupFile(restoreLookup.spi_name);
        setSpiBackupPath(restoreLookup.spi_path);
        setSpiFirmwareFile(restoreLookup.spi_name);
        setSpiFirmwarePath(restoreLookup.spi_path);
      }

      if (hasBank1 && hasSpi) {
        setLogs((items) => [
          ...items,
          "[recovery] Saved backup found and loaded into Flash Device",
          `[recovery] Bank1=${restoreLookup.mcu_path}`,
          ...(hasBank2 ? [`[recovery] Bank2=${restoreLookup.bank2_path}`] : []),
          `[recovery] SPI=${restoreLookup.spi_path}`,
          "[recovery] Press Restore Device to write the backup",
        ]);
      } else {
        setLogs((items) => [
          ...items,
          "[recovery] Saved backup is missing or incomplete",
          ...(hasBank1 ? [] : ["[recovery] Select Bank1 firmware manually"]),
          ...(hasSpi ? [] : ["[recovery] Select SPI flash manually"]),
        ]);
      }
    } catch (error) {
      setLogs((items) => [
        ...items,
        `[recovery] Backup lookup failed: ${String(error?.message ?? error)}`,
        "[recovery] Select Bank1 and SPI files manually in Flash Device",
      ]);
    }
  }

  function handleDeclineRecovery() {
    setRecoveryPromptAnswered(true);
    setRecoveryMode(false);
    setLogs((items) => [...items, "[recovery] User declined firmware recovery mode"]);
  }

  async function performBackupRead(kind, { announce = true } = {}) {
    const kindLabel = kind === "mcu" ? "BANK1" : kind === "bank2" ? "BANK2" : "SPI";
    if (!normalizedDetectedFirmware) {
      setLogs((items) => [
        ...items,
        `[backup] ${kindLabel} blocked: detected firmware is UNKNOWN, backup model name is unavailable`,
      ]);
      return false;
    }
    const backupModel = normalizedDetectedFirmware;
    const command =
      kind === "mcu"
        ? "read_mcu_backup"
        : kind === "bank2"
          ? "read_bank2_backup"
          : "read_spi_backup";

    if (announce) {
      setLogs((items) => [
        ...items,
        `[backup] Read ${kindLabel} button pressed`,
        `[backup] Reading ${kindLabel} backup from device...`,
      ]);
    }

    try {
      const result = await safeInvoke(command, {
        backend: "pyocd",
        frequency: probeFrequencyHz,
        model: backupModel,
        protection: deviceInfo.protection ?? "UNKNOWN",
        externalFlashMb: Number.parseFloat(String(deviceInfo.external_flash ?? "64")) || 64,
        });
      setBackupProgressState((prev) => ({
        ...prev,
        totalProgress: 100,
        mcuProgress: kind === "mcu" ? 100 : prev.mcuProgress,
        spiProgress: kind === "spi" ? 100 : prev.spiProgress,
      }));
      if (kind === "mcu") {
        setMcuBackupFile(result.name);
        setMcuBackupPath(result.path);
      } else if (kind === "bank2") {
        setBank2BackupFile(result.name);
        setBank2BackupPath(result.path);
      } else {
        setSpiBackupFile(result.name);
        setSpiBackupPath(result.path);
      }
      if ((kind === "mcu" || kind === "spi") && hasKnownDeviceUid) {
        try {
          const stockLookup = await safeInvoke("lookup_stock_backups", {
            request: { device_uid: deviceInfo.device_uid, firmware_profile: stockBackupFirmware },
          });
          if (stockLookup.mcu_name && stockLookup.mcu_path) {
            setStockMcuBackupFile(stockLookup.mcu_name);
            setStockMcuBackupPath(stockLookup.mcu_path);
          }
          if (stockLookup.spi_name && stockLookup.spi_path) {
            setStockSpiBackupFile(stockLookup.spi_name);
            setStockSpiBackupPath(stockLookup.spi_path);
          }
        } catch (lookupError) {
          setLogs((items) => [...items, `[stock] Refresh failed: ${String(lookupError)}`]);
        }
      }
      setLogs((items) => [
        ...items,
        `[backup] ${result.summary ?? `${kindLabel} backup completed`}`,
        `[backup] ${kindLabel}=${result.name}`,
      ]);
      return true;
    } catch (error) {
      const message = String(error?.message ?? error);
      setBackupProgressState((prev) => ({
        ...prev,
        totalProgress: 100,
        message,
      }));
      setLogs((items) => [...items, `[backup] ${kindLabel} ERROR: ${message}`]);
      return false;
    }
  }

  async function runBackupRead(kind) {
    setIsManualBackupRunning(true);
    setReadingBackupKind(kind);
    setBackupProgressState((prev) => ({
      totalProgress: 0,
      mcuProgress: kind === "mcu" ? 0 : prev.mcuProgress ?? 0,
      spiProgress: kind === "spi" ? 0 : prev.spiProgress ?? 0,
      speedBps: 0,
      frequency: probeFrequencyHz,
      phase: kind,
      backend: "",
      message: `Starting ${kind} backup`,
    }));
    setActiveAction("backup");

    try {
      await performBackupRead(kind, { announce: true });
    } finally {
      window.setTimeout(() => {
        setIsManualBackupRunning(false);
        setReadingBackupKind(null);
      }, 180);
    }
  }

  async function handleReadMcuBackup() {
    await runBackupRead("mcu");
  }

  async function handleReadBank2Backup() {
    await runBackupRead("bank2");
  }

  async function handleReadSpiBackup() {
    await runBackupRead("spi");
  }

  async function handleAutoBackup() {
    if (!canAutoBackup) {
      setLogs((items) => [...items, `[backup] Auto Backup blocked: ${backupReadReason.general}`]);
      return;
    }
    setIsAutoBackupRunning(true);
    setActiveAction("backup");
    setLogs((items) => [
      ...items,
      "[backup] Auto Backup started",
      isDeviceLocked ? "[backup] Order: SPI only (device locked)" : "[backup] Order: Bank1 -> Bank2 -> SPI",
    ]);

    try {
      if (!isDeviceLocked) {
        setReadingBackupKind("mcu");
        setBackupProgressState({
          totalProgress: 0,
          mcuProgress: 0,
          spiProgress: 0,
          speedBps: 0,
          frequency: probeFrequencyHz,
          phase: "mcu",
          backend: "",
          message: "Starting auto backup",
        });
        const bank1Ok = await performBackupRead("mcu", { announce: false });
        if (!bank1Ok) return;

        setReadingBackupKind("bank2");
        setBackupProgressState((prev) => ({
          ...prev,
          totalProgress: 34,
          speedBps: 0,
          phase: "bank2",
          message: t.readingBank2,
        }));
        const bank2Ok = await performBackupRead("bank2", { announce: false });
        if (!bank2Ok) return;
      }

      setReadingBackupKind("spi");
      setBackupProgressState((prev) => ({
        ...prev,
        totalProgress: isDeviceLocked ? 0 : 67,
        mcuProgress: isDeviceLocked ? prev.mcuProgress ?? 0 : 100,
        speedBps: 0,
        phase: "spi",
        message: t.readingSpi,
      }));
      const spiOk = await performBackupRead("spi", { announce: false });
      if (!spiOk) return;

      setBackupProgressState((prev) => ({
        ...prev,
        totalProgress: 100,
        mcuProgress: isDeviceLocked ? prev.mcuProgress ?? 0 : 100,
        spiProgress: 100,
        message: t.autoBackupFinished,
      }));
      setLogs((items) => [...items, "[backup] Auto Backup finished successfully"]);
    } finally {
      window.setTimeout(() => {
        setIsAutoBackupRunning(false);
        setReadingBackupKind(null);
      }, 180);
    }
  }

  async function handleSelectBackupFile(kind) {
    const label = kind === "mcu" ? "Bank1 backup" : kind === "bank2" ? "Bank2 backup" : "SPI backup";
    if (!canSelectBackupFiles) {
      setLogs((items) => [...items, `[backup] ${label} selection blocked: ${deviceUidIssue}`]);
      return;
    }
    const defaultPath =
      kind === "mcu"
        ? mcuBackupPath || backupDirectoryHint
        : kind === "bank2"
          ? bank2BackupPath || backupDirectoryHint
          : spiBackupPath || backupDirectoryHint;
    try {
      const picked = await safeInvoke("select_bin_file", {
        request: { title: `Select ${label}`, default_path: defaultPath },
      });
      if (!picked?.path) {
        return;
      }
      if (kind === "mcu") {
        setMcuBackupFile(picked.name);
        setMcuBackupPath(picked.path);
      } else if (kind === "bank2") {
        setBank2BackupFile(picked.name);
        setBank2BackupPath(picked.path);
      } else {
        setSpiBackupFile(picked.name);
        setSpiBackupPath(picked.path);
      }
      setActiveAction("backup");
      setLogs((items) => [...items, `[backup] ${label} selected: ${picked.path}`]);
    } catch (error) {
      setLogs((items) => [...items, `[backup] File picker failed: ${String(error)}`]);
    }
  }

  function applyStockBackupLookup(stockLookup) {
    setStockMcuBackupFile(stockLookup?.mcu_name || null);
    setStockMcuBackupPath(stockLookup?.mcu_path || null);
    setStockSpiBackupFile(stockLookup?.spi_name || null);
    setStockSpiBackupPath(stockLookup?.spi_path || null);
  }

  async function refreshStockBackupLookup(firmware = stockFirmwarePromptProfile ?? stockBackupFirmware) {
    if (!firmware || !hasKnownDeviceUid) {
      return {
        mcu_name: stockMcuBackupFile,
        mcu_path: stockMcuBackupPath,
        spi_name: stockSpiBackupFile,
        spi_path: stockSpiBackupPath,
      };
    }
    const stockLookup = await safeInvoke("lookup_stock_backups", {
      request: { device_uid: deviceInfo.device_uid, firmware_profile: firmware },
    });
    applyStockBackupLookup(stockLookup);
    return stockLookup;
  }

  async function importStockBackupPath(kind, path, firmware = stockFirmwarePromptProfile ?? stockBackupFirmware) {
    const isMcu = kind === "mcu";
    const label = isMcu ? "MCU Bank1 stock backup" : "SPI Flash stock backup";
    if (!firmware) {
      setLogs((items) => [...items, `[stock] ${label} import blocked: firmware profile is unknown`]);
      return false;
    }

    const result = await safeInvoke("import_stock_backup", {
      request: {
        firmware_profile: firmware,
        kind: isMcu ? "bank1" : "spi",
        path,
      },
    });
    setLogs((items) => [
      ...items,
      `[stock] Imported ${label}: ${result.path}`,
      `[stock] Size: ${formatMbValue(bytesToMb(Number(result.size_bytes ?? 0)))} MB`,
    ]);
    return true;
  }

  function continueBuildAfterStockIfReady(stockLookup) {
    const ready = Boolean(stockLookup?.mcu_path && stockLookup?.spi_path);
    if (ready && pendingStockRestoreProfile) {
      const profile = pendingStockRestoreProfile;
      setStockFirmwarePrompt(false);
      setPendingStockRestoreProfile(null);
      window.setTimeout(() => {
        handleRestoreOriginalFirmware(profile);
      }, 50);
      return true;
    }
    if (ready && retryBuildAfterStockFirmware) {
      setStockFirmwarePrompt(false);
      setRetryBuildAfterStockFirmware(false);
      window.setTimeout(() => {
        handleBuildFirmware();
      }, 50);
    }
    return ready;
  }

  async function handleImportStockBackup(kind) {
    const isMcu = kind === "mcu";
    const label = isMcu ? "MCU Bank1 stock backup" : "SPI Flash stock backup";
    const importFirmware = stockFirmwarePromptProfile ?? stockBackupFirmware;
    if (!importFirmware) {
      setLogs((items) => [...items, `[stock] ${label} import blocked: firmware profile is unknown`]);
      return;
    }

    try {
      const picked = await safeInvoke("select_bin_file", {
        request: { title: `Select ${label}`, default_path: backupDirectoryHint },
      });
      if (!picked?.path) {
        return;
      }

      await importStockBackupPath(kind, picked.path, importFirmware);
      const stockLookup = await refreshStockBackupLookup(importFirmware);
      setActiveAction("build");
      continueBuildAfterStockIfReady(stockLookup);
    } catch (error) {
      setLogs((items) => [...items, `[stock] Import failed: ${String(error?.message ?? error)}`]);
    }
  }

  async function handleRevealBackup(kind) {
    const filePath = kind === "mcu" ? mcuBackupPath : kind === "bank2" ? bank2BackupPath : spiBackupPath;
    if (!filePath) {
      setLogs((items) => [...items, `[backup] No ${kind.toUpperCase()} backup path to reveal`]);
      return;
    }
    try {
      await safeInvoke("reveal_path_in_explorer", { request: { path: filePath } });
      setLogs((items) => [...items, `[backup] Opened in Explorer: ${filePath}`]);
    } catch (error) {
      setLogs((items) => [...items, `[backup] Explorer open failed: ${String(error)}`]);
    }
  }

  async function handleSelectOrRevealFirmware(kind) {
    const label = kind === "mcu" ? "Bank1 firmware" : kind === "bank2" ? "Bank2 firmware" : "SPI firmware";
    const defaultPath =
      kind === "mcu"
        ? mcuFirmwarePath || ""
        : kind === "bank2"
          ? bank2FirmwarePath || ""
          : spiFirmwarePath || "";
    try {
      const picked = await safeInvoke("select_bin_file", {
        request: { title: `Select ${label}`, default_path: defaultPath },
      });
      if (!picked?.path) {
        return;
      }
      const resultKind = kind === "mcu" ? "bank1" : kind;
      if (kind === "mcu") {
        setMcuFirmwareFile(picked.name);
        setMcuFirmwarePath(picked.path);
      } else if (kind === "bank2") {
        setBank2FirmwareFile(picked.name);
        setBank2FirmwarePath(picked.path);
      } else {
        setSpiFirmwareFile(picked.name);
        setSpiFirmwarePath(picked.path);
      }
      setFlashResult((prev) => ({ ...prev, [resultKind]: null }));
      setActiveAction("flash");
      setLogs((items) => [...items, `[flash] ${label} selected: ${picked.path}`]);
    } catch (error) {
      setLogs((items) => [...items, `[flash] File picker failed: ${String(error)}`]);
    }
  }

  function handleAddGames(emulatorId) {
    setSelectedEmulator(emulatorId);
    setSelectedGameId(null);
    setPendingRomEmulator(emulatorId);
    emulatorRomInputRef.current?.click();
  }

  async function hydrateCachedImages(emulatorId, games) {
    for (const game of games) {
      try {
        const cachedImageUrl = await safeInvoke("load_thumbnail_cache", {
          request: { emulator: emulatorId, title: game.title },
        });
        if (!cachedImageUrl) {
          continue;
        }
        setRomLibrary((prev) => ({
          ...prev,
          [emulatorId]: (prev[emulatorId] ?? []).map((entry) =>
            entry.id === game.id
              ? {
                  ...entry,
                  imageLoaded: true,
                  imageStatus: "loaded",
                  imageProgress: 100,
                  imageUrl: cachedImageUrl,
                }
              : entry,
          ),
        }));
      } catch (error) {
        setLogs((items) => [...items, `[images] Cache read error for ${game.title}: ${String(error)}`]);
      }
    }
  }

  useEffect(() => {
    let cancelled = false;

    async function syncCachedImages() {
      const pendingGames = loadedGames.filter(
        (game) => game?.emulatorId && !game.imageLoaded && !game.imageUrl && game.imageStatus !== "downloading",
      );

      for (const game of pendingGames) {
        if (cancelled) {
          return;
        }
        try {
          const cachedImageUrl = await safeInvoke("load_thumbnail_cache", {
            request: { emulator: game.emulatorId, title: game.title },
          });
          if (!cachedImageUrl || cancelled) {
            continue;
          }
          setRomLibrary((prev) => ({
            ...prev,
            [game.emulatorId]: (prev[game.emulatorId] ?? []).map((entry) =>
              entry.id === game.id
                ? {
                    ...entry,
                    imageLoaded: true,
                    imageStatus: "loaded",
                    imageProgress: 100,
                    imageUrl: cachedImageUrl,
                  }
                : entry,
            ),
          }));
        } catch {
          // Ignore cache misses and continue scanning.
        }
      }
    }

    if (loadedGames.length > 0) {
      syncCachedImages();
    }

    return () => {
      cancelled = true;
    };
  }, [loadedGames]);

  async function handleRomFilesPicked(event) {
    const emulatorId = pendingRomEmulator;
    const files = Array.from(event.target.files ?? []);
    if (!emulatorId || files.length === 0) {
      event.target.value = "";
      return;
    }

    const importPayload = await Promise.all(
      files.map(async (file) => ({
        name: file.name,
        bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
      })),
    );

    let importResult;
    try {
      importResult = await safeInvoke("import_rom_files", {
        request: {
          emulator: emulatorId,
          files: importPayload,
        },
      });
    } catch (error) {
      setLogs((items) => [...items, `[rom] Import error for ${emulatorId.toUpperCase()}: ${String(error)}`]);
      setPendingRomEmulator(null);
      event.target.value = "";
      return;
    }

    const importedEntries = importResult?.entries ?? [];
    const existingGames = romLibrary[emulatorId] ?? [];
    const candidateGames = importedEntries.map((entry, index) =>
      toImportedGame(emulatorId, entry, existingGames.length + index),
    );
    const { merged, uniqueImported } = mergeUniqueGames(existingGames, candidateGames);

    setRomLibrary((prev) => ({
      ...prev,
      [emulatorId]: merged,
    }));
    setSelectedEmulator(emulatorId);
    setSelectedGameId(uniqueImported[0]?.id ?? existingGames[0]?.id ?? null);
    setBuilderSettingsOpen(true);
    setActiveAction("build");
    setLogs((items) => [
      ...items,
      `[rom] Added ${uniqueImported.length} ROM file(s) for ${emulatorId.toUpperCase()}`,
      ...(candidateGames.length !== uniqueImported.length
        ? [`[rom] Skipped duplicates: ${candidateGames.length - uniqueImported.length}`]
        : []),
      ...((importResult?.warnings ?? []).map((warning) => `[rom] ${warning}`)),
    ]);
    setPendingRomEmulator(null);
    event.target.value = "";
    await hydrateCachedImages(emulatorId, uniqueImported);
  }

  async function handleDownloadImages() {
    const gamesNeedingImages = imageCapableGames.filter((game) => !game.imageLoaded && !game.imageUrl);
    if (imageCapableGames.length === 0) {
      setLogs((items) => [...items, "[images] No image-capable ROMs loaded"]);
      return;
    }
    if (gamesNeedingImages.length === 0) {
      setCoverflowImagesEnabled(true);
      setLogs((items) => [...items, "[images] All loaded ROMs already have cached images"]);
      return;
    }

    setIsDownloadingImages(true);
    setImageDownloadProgress(0);
    const groupedGames = gamesNeedingImages.reduce((groups, game) => {
      const key = game.emulatorId;
      if (!groups[key]) {
        groups[key] = [];
      }
      groups[key].push(game);
      return groups;
    }, {});

    setRomLibrary((prev) => {
      const next = { ...prev };
      for (const emulatorId of Object.keys(groupedGames)) {
        const ids = new Set(groupedGames[emulatorId].map((game) => game.id));
        next[emulatorId] = (prev[emulatorId] ?? []).map((game) =>
          ids.has(game.id)
            ? {
                ...game,
                imageStatus: "idle",
                imageProgress: 0,
              }
            : game,
        );
      }
      return next;
    });
    setLogs((items) => [...items, `[images] Starting real thumbnail fetch for ${gamesNeedingImages.length} ROM(s)`]);

    let loadedCount = 0;
    let missingCount = 0;
    const totalCount = gamesNeedingImages.length;
    let processedCount = 0;

    try {
      for (const [emulatorId, currentGames] of Object.entries(groupedGames)) {
        const source = THUMBNAIL_SOURCES[emulatorId];
        if (!source) {
          continue;
        }

        setLogs((items) => [
          ...items,
          `[images] Source ${source.label}: ${currentGames.length} ROM(s)`,
        ]);

        for (let index = 0; index < currentGames.length; index += 1) {
          const game = currentGames[index];
          const thumbnailCandidates = buildThumbnailCandidates(game.title);

          setRomLibrary((prev) => ({
            ...prev,
            [emulatorId]: (prev[emulatorId] ?? []).map((entry) =>
              entry.id === game.id
                ? {
                    ...entry,
                    imageStatus: "downloading",
                    imageProgress: 15,
                  }
                : entry,
            ),
          }));

          try {
            let resolvedResponse = null;
            let resolvedName = "";

            for (const candidate of thumbnailCandidates) {
              for (const snapsBase of source.snapsBases ?? [source.snapsBase]) {
                const candidateUrl = `${snapsBase}/${encodeURIComponent(candidate)}.png`;
                try {
                  const response = await fetch(candidateUrl, { cache: "no-store" });
                  if (response.ok) {
                    resolvedResponse = response;
                    resolvedName = candidate;
                    break;
                  }
                } catch {
                  // Try the next source; some hosts can fail CORS or rate-limit transiently.
                }
              }
              if (resolvedResponse) {
                break;
              }
            }

            if (!resolvedResponse) {
              const indexedMatch = await resolveIndexedThumbnail(source, game.title);
              if (indexedMatch) {
                for (const snapsBase of source.snapsBases ?? [source.snapsBase]) {
                  const candidateUrl = `${snapsBase}/${encodeURIComponent(indexedMatch.fileName)}`;
                  try {
                    const response = await fetch(candidateUrl, { cache: "no-store" });
                    if (response.ok) {
                      resolvedResponse = response;
                      resolvedName = indexedMatch.fileName.replace(/\.png$/i, "");
                      break;
                    }
                  } catch {
                    // Keep fallback order deterministic.
                  }
                }
              }
            }

            if (!resolvedResponse) {
              throw new Error("No thumbnail match");
            }
            const blob = await resolvedResponse.blob();
            const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
            const cachedPath = await safeInvoke("save_thumbnail_cache", {
              request: { emulator: emulatorId, title: game.title, bytes },
            });
            const objectUrl = URL.createObjectURL(blob);
            loadedCount += 1;

            setRomLibrary((prev) => ({
              ...prev,
              [emulatorId]: (prev[emulatorId] ?? []).map((entry) =>
                entry.id === game.id
                  ? {
                      ...entry,
                      imageLoaded: true,
                      imageStatus: "loaded",
                      imageProgress: 100,
                      imageUrl: objectUrl,
                    }
                  : entry,
              ),
            }));
            setLogs((items) => [
              ...items,
              `[images] ${source.label} ${index + 1}/${currentGames.length} loaded: ${game.title} -> ${resolvedName}`,
              `[images] cached: ${cachedPath}`,
            ]);
          } catch (error) {
            missingCount += 1;
            setRomLibrary((prev) => ({
              ...prev,
              [emulatorId]: (prev[emulatorId] ?? []).map((entry) =>
                entry.id === game.id
                  ? {
                      ...entry,
                      imageLoaded: false,
                      imageStatus: "missing",
                      imageProgress: 0,
                      imageUrl: null,
                    }
                  : entry,
              ),
            }));
            setLogs((items) => [
              ...items,
              `[images] ${source.label} ${index + 1}/${currentGames.length} missing: ${game.title} (${String(error)})`,
            ]);
          } finally {
            processedCount += 1;
            setImageDownloadProgress(totalCount > 0 ? (processedCount / totalCount) * 100 : 0);
          }
        }
      }
      setLogs((items) => [
        ...items,
        `[images] Finished: loaded=${loadedCount}, missing=${missingCount}`,
      ]);
      setCoverflowImagesEnabled(true);
    } finally {
      setIsDownloadingImages(false);
      setImageDownloadProgress(100);
    }
  }

  async function persistGameImage(emulatorId, game, bytes, sourceLabel, { forceReplace = false } = {}) {
    if (!game || !emulatorId) {
      return false;
    }
    if (game.imageLoaded && !forceReplace) {
      return false;
    }

    setRomLibrary((prev) => ({
      ...prev,
      [emulatorId]: (prev[emulatorId] ?? []).map((entry) =>
        entry.id === game.id
          ? {
              ...entry,
              imageStatus: "downloading",
              imageProgress: 30,
            }
          : entry,
      ),
    }));

    try {
      const cachedPath = await safeInvoke("save_thumbnail_cache", {
        request: { emulator: emulatorId, title: game.title, bytes },
      });
      const cachedImageUrl = await safeInvoke("load_thumbnail_cache", {
        request: { emulator: emulatorId, title: game.title },
      });
      setRomLibrary((prev) => ({
        ...prev,
        [emulatorId]: (prev[emulatorId] ?? []).map((entry) =>
          entry.id === game.id
            ? {
                ...entry,
                imageLoaded: true,
                imageStatus: "loaded",
                imageProgress: 100,
                imageUrl: cachedImageUrl ?? entry.imageUrl,
              }
            : entry,
        ),
      }));
      setLogs((items) => [...items, `[images] Applied: ${game.title} (${sourceLabel})`, `[images] cached: ${cachedPath}`]);
      setCoverflowImagesEnabled(true);
      return true;
    } catch (error) {
      setRomLibrary((prev) => ({
        ...prev,
        [emulatorId]: (prev[emulatorId] ?? []).map((entry) =>
          entry.id === game.id
            ? {
                ...entry,
                imageLoaded: false,
                imageStatus: "missing",
                imageProgress: 0,
                imageUrl: null,
              }
            : entry,
        ),
      }));
      setLogs((items) => [...items, `[images] Import error for ${game.title}: ${String(error)}`]);
      return false;
    }
  }

  async function importImageFiles(files, options = {}) {
    const supportedFiles = Array.from(files ?? []).filter((file) => isSupportedImagePath(file.name));
    if (supportedFiles.length === 0) {
      setLogs((items) => [...items, "[images] No supported image files found"]);
      return;
    }

    const availableGames = loadedGames.filter((game) => options.forceReplace || !game.imageLoaded);
    if (availableGames.length === 0) {
      setLogs((items) => [...items, "[images] No eligible ROMs available for image import"]);
      return;
    }

    const unmatchedNames = [];
    const matchedGameIds = new Set();
    let appliedCount = 0;

    for (const file of supportedFiles) {
      const fileKey = normalizeImageMatchName(file.name);
      const matchedGame = availableGames.find((game) => {
        if (matchedGameIds.has(game.id)) {
          return false;
        }
        return buildGameImageMatchKeys(game.title).includes(fileKey);
      });

      if (!matchedGame) {
        unmatchedNames.push(file.name);
        continue;
      }

      matchedGameIds.add(matchedGame.id);
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const applied = await persistGameImage(
        matchedGame.emulatorId,
        matchedGame,
        bytes,
        file.name,
        { forceReplace: Boolean(options.forceReplace) },
      );
      if (applied) {
        appliedCount += 1;
      }
    }

    setLogs((items) => [
      ...items,
      `[images] Import summary: applied=${appliedCount}, skipped=${supportedFiles.length - appliedCount}`,
      ...unmatchedNames.slice(0, 20).map((name) => `[images] No ROM match: ${name}`),
      ...(unmatchedNames.length > 20 ? [`[images] Additional unmatched files: ${unmatchedNames.length - 20}`] : []),
    ]);
  }

  async function importImagePaths(paths, options = {}) {
    const supportedPaths = Array.from(paths ?? []).filter((path) => isSupportedImagePath(path));
    if (supportedPaths.length === 0) {
      setLogs((items) => [...items, "[images] No supported image paths found"]);
      return;
    }

    const availableGames = loadedGames.filter((game) => options.forceReplace || !game.imageLoaded);
    if (availableGames.length === 0) {
      setLogs((items) => [...items, "[images] No eligible ROMs available for image import"]);
      return;
    }

    const matchedGameIds = new Set();
    const unmatchedPaths = [];
    let appliedCount = 0;

    for (const path of supportedPaths) {
      const fileName = String(path).split(/[\\/]/).pop() ?? String(path);
      const fileKey = normalizeImageMatchName(fileName);
      const matchedGame = availableGames.find((game) => {
        if (matchedGameIds.has(game.id)) {
          return false;
        }
        return buildGameImageMatchKeys(game.title).includes(fileKey);
      });

      if (!matchedGame) {
        unmatchedPaths.push(fileName);
        continue;
      }

      matchedGameIds.add(matchedGame.id);
      try {
        const bytes = await safeInvoke("read_binary_file", { request: { path } });
        const applied = await persistGameImage(
          matchedGame.emulatorId,
          matchedGame,
          bytes,
          fileName,
          { forceReplace: Boolean(options.forceReplace) },
        );
        if (applied) {
          appliedCount += 1;
        }
      } catch (error) {
        setLogs((items) => [...items, `[images] Read error for ${fileName}: ${String(error)}`]);
      }
    }

    setLogs((items) => [
      ...items,
      `[images] Path import summary: applied=${appliedCount}, skipped=${supportedPaths.length - appliedCount}`,
      ...unmatchedPaths.slice(0, 20).map((name) => `[images] No ROM match: ${name}`),
      ...(unmatchedPaths.length > 20 ? [`[images] Additional unmatched files: ${unmatchedPaths.length - 20}`] : []),
    ]);
  }

  async function handleBuildFirmware() {
    if (loadedGames.length === 0) {
      return;
    }
    if (isDownloadingImages) {
      setLogs((items) => [...items, "[build] BLOCKED: wait for image download to finish"]);
      return;
    }
    if (!isDeviceIdentified) {
      setLogs((items) => [...items, `[build] BLOCKED: ${deviceIdentifyIssue}`]);
      return;
    }
    let resolvedStockMcuPath = stockMcuBackupPath;
    let resolvedStockSpiPath = stockSpiBackupPath;
    let resolvedStockMcuReady = stockMcuReady;
    let resolvedStockSpiReady = stockSpiReady;
    if (!stockBackupReady) {
      try {
        const stockLookup = await refreshStockBackupLookup();
        resolvedStockMcuPath = stockLookup?.mcu_path || null;
        resolvedStockSpiPath = stockLookup?.spi_path || null;
        resolvedStockMcuReady = Boolean(resolvedStockMcuPath);
        resolvedStockSpiReady = Boolean(resolvedStockSpiPath);
      } catch (error) {
        setLogs((items) => [...items, `[stock] Lookup failed: ${String(error?.message ?? error)}`]);
      }
      if (!resolvedStockMcuReady || !resolvedStockSpiReady) {
        setBuilderSettingsOpen(true);
        setActiveAction("build");
        setStockFirmwarePrompt(true);
        setRetryBuildAfterStockFirmware(true);
        setLogs((items) => [
          ...items,
          "[build] Stock firmware required: drop original files to continue build",
          ...(resolvedStockMcuReady ? [] : ["[build] Missing MCU Bank1 stock firmware"]),
          ...(resolvedStockSpiReady ? [] : ["[build] Missing SPI Flash stock firmware"]),
        ]);
        return;
      }
    }
    if (loadedGames.some((game) => game.emulatorId === "msx")) {
      const biosStatus = await safeInvoke("check_msx_bios");
      if (!biosStatus.ready) {
        setMsxBiosPrompt(biosStatus);
        setRetryBuildAfterMsxBios(true);
        setLogs((items) => [
          ...items,
          `[build] MSX BIOS required: ${biosStatus.missing.join(", ")}`,
          `[build] Drop MSX BIOS files into the app to continue build`,
        ]);
        return;
      }
    }
    if (loadedGames.some((game) => game.emulatorId === "col")) {
      const biosStatus = await safeInvoke("check_coleco_bios");
      if (!biosStatus.ready) {
        setColecoBiosPrompt(biosStatus);
        setRetryBuildAfterColecoBios(true);
        setLogs((items) => [
          ...items,
          `[build] ColecoVision BIOS required: ${biosStatus.missing.join(", ")}`,
          "[build] Drop coleco.rom into the app to continue build",
        ]);
        return;
      }
    }

    const romMb = formatMbValue(romUsedMb);
    const imageMb = formatMbValue(imageUsedMb);
    const totalMb = formatMbValue(emulatorUsedMb);
    const requiredMb = formatMbValue(spiUsedMb);
    const availableMb = formatMbValue(spiTotalMb);

    setBuilderSettingsOpen(true);
    setActiveAction("build");
    setIsBuildingFirmware(true);
    setBuildFirmwareProgress(0);
    setBuildFirmwareMessage("Preparing build workspace");
    setBuildFirmwareError("");
    setMcuFirmwareFile(null);
    setMcuFirmwarePath(null);
    setBank2FirmwareFile(null);
    setBank2FirmwarePath(null);
    setSpiFirmwareFile(null);
    setSpiFirmwarePath(null);
    setBuiltExtflashBytes(0);
    setBuiltExtflashSignature("");
    setFlashCompletionStatus(null);
    setFlashResult({ bank1: null, bank2: null, spi: null });
    setLogs((items) => [
      ...items,
      `[build] Preparing firmware build for ${Object.keys(romLibrary).length} emulator(s)`,
      `[build] ROMs: ${buildMetrics.romCount} file(s) • ${romMb} MB`,
      `[build] Images: ${buildMetrics.imageCount} file(s) • ${imageMb} MB`,
      `[build] Emulator payload: ${totalMb} MB`,
      `[build] SPI required: ${requiredMb} MB / ${availableMb} MB available`,
      `[build] Retro-Go menu: ${coverflowImagesEnabled ? "cover cards" : "plain list"}`,
    ]);

    try {
      const result = await safeInvoke("build_firmware_bundle", {
        request: {
          firmware_profile: firmwareProfile,
          installed_spi_mb: spiTotalMb,
          firmware_reserved_mb: firmwareBaseMb,
          stock_bank1_path: resolvedStockMcuPath,
          stock_spi_path: resolvedStockSpiPath,
          coverflow_enabled: coverflowImagesEnabled,
          entries: loadedGames.map((game) => ({
            emulator: game.emulatorId,
            title: game.title,
            rom_path: game.path,
          })),
        },
      });
      if (result.bank1_candidate_path) {
        setMcuFirmwareFile(result.bank1_candidate_path);
        setMcuFirmwarePath(result.bank1_candidate_path);
      }
      if (result.bank2_candidate_path) {
        setBank2FirmwareFile(result.bank2_candidate_path);
        setBank2FirmwarePath(result.bank2_candidate_path);
      }
      if (result.extflash_build_path) {
        setSpiFirmwareFile(result.extflash_build_path);
        setSpiFirmwarePath(result.extflash_build_path);
      }
      if (result.bank1_candidate_path || result.bank2_candidate_path || result.extflash_build_path) {
        setFlashResult({ bank1: null, bank2: null, spi: null });
      }
      setActiveAction("flash");
      setBuiltExtflashBytes(Number(result.extflash_build_size_bytes ?? 0));
      setBuiltExtflashSignature(romBuildSignature);
      setBuildFirmwareProgress(100);
      setBuildFirmwareMessage(t.firmwareBuiltReady);
      setLogs((items) => [
        ...items,
        `[build] Bundle ready: ${result.bundle_dir}`,
        `[build] Retro-Go fork workspace: ${result.retro_go_workspace_dir}`,
        `[build] Build log: ${result.build_log_path}`,
        `[build] Output: ${result.rom_count} ROM(s), ${result.image_count} image(s)`,
        `[build] Coverflow embedded: ${Number(result.coverflow_count ?? result.romart_count ?? 0)} image(s)`,
        `[build] Extflash built: ${result.extflash_built ? "yes" : "no"}`,
        `[build] Built extflash size: ${formatMbValue(bytesToMb(Number(result.extflash_build_size_bytes ?? 0)))} MB`,
      ]);
    } catch (error) {
      const message = String(error?.message ?? error);
      setBuildFirmwareError(message);
      setLogs((items) => [...items, `[build] ERROR: ${message}`]);
    } finally {
      setIsBuildingFirmware(false);
    }
  }

  async function primeLatestFirmwareBundle() {
    const isBundleFirmwarePath = (path) => {
      const text = String(path ?? "").replaceAll("\\", "/").toLowerCase();
      return text.includes("/bundles/") && text.includes("/firmware/");
    };
    const hasAnyFirmwarePath = Boolean(mcuFirmwarePath || bank2FirmwarePath || spiFirmwarePath);
    const hasOnlyBundleFirmwarePaths = [mcuFirmwarePath, bank2FirmwarePath, spiFirmwarePath]
      .filter(Boolean)
      .every(isBundleFirmwarePath);
    if (hasAnyFirmwarePath && !hasOnlyBundleFirmwarePaths) {
      setMcuFirmwareFile(null);
      setMcuFirmwarePath(null);
      setBank2FirmwareFile(null);
      setBank2FirmwarePath(null);
      setSpiFirmwareFile(null);
      setSpiFirmwarePath(null);
      setFlashResult({ bank1: null, bank2: null, spi: null });
      setLogs((items) => [...items, "[flash] Cleared stock/backup paths from firmware flash slots"]);
    } else if (hasAnyFirmwarePath) {
      return true;
    }
    try {
      const result = await safeInvoke("latest_firmware_bundle");
      if (!result?.found) {
        setLogs((items) => [
          ...items,
          `[flash] No dualboot firmware bundle found${result?.message ? `: ${result.message}` : ""}`,
        ]);
        return false;
      }
      const bundleText = `${result.bundle_dir ?? ""} ${result.manifest_path ?? ""} ${result.bank2_candidate_path ?? ""}`.toLowerCase();
      const resultProfile = bundleText.includes("bundle_z_") || bundleText.includes("bank2z.bin")
        ? "Z"
        : bundleText.includes("bundle_m_") || bundleText.includes("bank2m.bin")
          ? "M"
          : null;
      if (resultProfile && resultProfile !== firmwareProfile) {
        setLogs((items) => [
          ...items,
          `[flash] Latest bundle is ${resultProfile}, but connected firmware is ${firmwareProfile}. Rebuild firmware for this device.`,
        ]);
        return false;
      }
      if (result.bank1_candidate_path) {
        setMcuFirmwareFile(result.bank1_candidate_path);
        setMcuFirmwarePath(result.bank1_candidate_path);
      }
      if (result.bank2_candidate_path) {
        setBank2FirmwareFile(result.bank2_candidate_path);
        setBank2FirmwarePath(result.bank2_candidate_path);
      }
      if (result.extflash_build_path) {
        setSpiFirmwareFile(result.extflash_build_path);
        setSpiFirmwarePath(result.extflash_build_path);
      }
      setBuiltExtflashBytes(Number(result.extflash_build_size_bytes ?? 0));
      setFlashResult({ bank1: null, bank2: null, spi: null });
      return Boolean(result.extflash_build_path && (result.bank2_candidate_path || result.bank1_candidate_path));
    } catch (error) {
      setLogs((items) => [...items, `[flash] Failed to load latest bundle: ${String(error)}`]);
      return false;
    }
  }

  useEffect(() => {
    if (activeAction !== "flash") {
      return;
    }
    primeLatestFirmwareBundle();
  }, [activeAction, mcuFirmwarePath, bank2FirmwarePath, spiFirmwarePath]);

  async function handleImagePackPicked(event) {
    const files = Array.from(event.target.files ?? []);
    await importImageFiles(files);
    event.target.value = "";
  }

  async function handleImageFolderPicked(event) {
    const files = Array.from(event.target.files ?? []);
    await importImageFiles(files);
    event.target.value = "";
  }

  async function handleManualImagePicked(event) {
    const file = event.target.files?.[0];
    const gameId = pendingManualImageGameId;
    if (!file || !gameId) {
      event.target.value = "";
      setPendingManualImageGameId(null);
      return;
    }

    const game = loadedGames.find((entry) => entry.id === gameId);
    if (!game) {
      event.target.value = "";
      setPendingManualImageGameId(null);
      return;
    }

    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    await persistGameImage(game.emulatorId, game, bytes, file.name, { forceReplace: true });
    setPendingManualImageGameId(null);
    event.target.value = "";
  }

  async function importDroppedStockFirmwarePaths(paths) {
    if (!Array.isArray(paths) || paths.length === 0) {
      return;
    }
    const importFirmware = stockFirmwarePromptProfile ?? stockBackupFirmware;
    if (!importFirmware) {
      setLogs((items) => [...items, "[stock] Import blocked: firmware profile is unknown"]);
      return;
    }

    let currentMcuReady = stockMcuReady;
    let currentSpiReady = stockSpiReady;
    let importedCount = 0;

    for (const path of paths) {
      let imported = false;
      if (!currentMcuReady) {
        try {
          imported = await importStockBackupPath("mcu", path, importFirmware);
          currentMcuReady = true;
        } catch (_error) {
          setLogs((items) => [...items, `[stock] Not MCU Bank1 stock: ${String(path).split(/[\\/]/).pop()}`]);
        }
      }
      if (!imported && !currentSpiReady) {
        try {
          imported = await importStockBackupPath("spi", path, importFirmware);
          currentSpiReady = true;
        } catch (_error) {
          setLogs((items) => [...items, `[stock] Not SPI Flash stock: ${String(path).split(/[\\/]/).pop()}`]);
        }
      }
      if (imported) {
        importedCount += 1;
      }
      if (currentMcuReady && currentSpiReady) {
        break;
      }
    }

    const stockLookup = await refreshStockBackupLookup(importFirmware);
    const ready = continueBuildAfterStockIfReady(stockLookup);
    setLogs((items) => [
      ...items,
      ready
        ? "[stock] Stock firmware ready, build will continue"
        : `[stock] Stock import incomplete: imported=${importedCount}`,
    ]);
  }

  async function handleGlobalDrop(event) {
    event?.preventDefault?.();
    event?.stopPropagation?.();
    setIsDragActive(false);

    if (stockFirmwarePrompt) {
      try {
        if (Array.isArray(event?.payload?.paths) && event.payload.paths.length > 0) {
          await importDroppedStockFirmwarePaths(event.payload.paths);
        } else {
          setLogs((items) => [
            ...items,
            "[stock] Browser drop cannot import stock firmware paths; use the select buttons in the stock firmware window",
          ]);
        }
      } catch (error) {
        setLogs((items) => [...items, `[stock] Import error: ${String(error?.message ?? error)}`]);
      }
      return;
    }

    if (msxBiosPrompt) {
      try {
        let status;
        if (Array.isArray(event?.payload?.paths) && event.payload.paths.length > 0) {
          status = await safeInvoke("save_msx_bios_paths", { request: { paths: event.payload.paths } });
        } else {
          const files = Array.from(event?.dataTransfer?.files ?? []);
          const payload = [];
          for (const file of files) {
            payload.push({
              name: file.name,
              bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
            });
          }
          status = await safeInvoke("save_msx_bios_files", { request: { files: payload } });
        }
        setMsxBiosPrompt(status);
        setLogs((items) => [
          ...items,
          status.ready
            ? `[build] MSX BIOS saved: ${status.dir}`
            : `[build] MSX BIOS still missing: ${status.missing.join(", ")}`,
        ]);
        if (status.ready && retryBuildAfterMsxBios) {
          setMsxBiosPrompt(null);
          setRetryBuildAfterMsxBios(false);
          window.setTimeout(() => {
            handleBuildFirmware();
          }, 50);
        }
      } catch (error) {
        setLogs((items) => [...items, `[build] MSX BIOS import error: ${String(error?.message ?? error)}`]);
      }
      return;
    }
    if (colecoBiosPrompt) {
      try {
        let status;
        if (Array.isArray(event?.payload?.paths) && event.payload.paths.length > 0) {
          status = await safeInvoke("save_coleco_bios_paths", { request: { paths: event.payload.paths } });
        } else {
          const files = Array.from(event?.dataTransfer?.files ?? []);
          const payload = [];
          for (const file of files) {
            payload.push({
              name: file.name,
              bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
            });
          }
          status = await safeInvoke("save_coleco_bios_files", { request: { files: payload } });
        }
        setColecoBiosPrompt(status);
        setLogs((items) => [
          ...items,
          status.ready
            ? `[build] ColecoVision BIOS saved: ${status.dir}`
            : `[build] ColecoVision BIOS still missing: ${status.missing.join(", ")}`,
        ]);
        if (status.ready && retryBuildAfterColecoBios) {
          setColecoBiosPrompt(null);
          setRetryBuildAfterColecoBios(false);
          window.setTimeout(() => {
            handleBuildFirmware();
          }, 50);
        }
      } catch (error) {
        setLogs((items) => [...items, `[build] ColecoVision BIOS import error: ${String(error?.message ?? error)}`]);
      }
      return;
    }

    let importResult;
    try {
      if (Array.isArray(event?.payload?.paths) && event.payload.paths.length > 0) {
        const signature = event.payload.paths.join("|");
        const now = Date.now();
        if (
          lastNativeDropRef.current.signature === signature &&
          now - lastNativeDropRef.current.at < 1500
        ) {
          return;
        }
        lastNativeDropRef.current = { signature, at: now };
        const imagePaths = event.payload.paths.filter((path) => isSupportedImagePath(path));
        if (imagePaths.length > 0) {
          await importImagePaths(imagePaths);
          return;
        }
        const supportedPaths = event.payload.paths.filter((path) => isSupportedImportPath(path));
        const skippedPaths = event.payload.paths.filter((path) => !isSupportedImportPath(path));
        if (skippedPaths.length > 0) {
          setLogs((items) => [
            ...items,
            ...skippedPaths.map((path) => `[rom] Dropped file ignored: ${String(path).split(/[\\/]/).pop()}`),
          ]);
        }
        if (supportedPaths.length === 0) {
          return;
        }
        importResult = await safeInvoke("import_rom_paths_auto", { request: { paths: supportedPaths } });
      } else {
        const files = Array.from(event?.dataTransfer?.files ?? []);
        if (files.length === 0) {
          return;
        }
        setLogs((items) => [
          ...items,
          `[drop] browser files received: ${files.length}`,
        ]);
        const imageFiles = files.filter((file) => isSupportedImagePath(file.name));
        if (imageFiles.length > 0) {
          await importImageFiles(imageFiles);
          return;
        }
        const supportedFiles = files.filter((file) => isSupportedImportPath(file.name));
        const skippedFiles = files.filter((file) => !isSupportedImportPath(file.name));
        if (skippedFiles.length > 0) {
          setLogs((items) => [
            ...items,
            ...skippedFiles.map((file) => `[rom] Dropped file ignored: ${file.name}`),
          ]);
        }
        if (supportedFiles.length === 0) {
          return;
        }
        setLogs((items) => [...items, `[drop] browser accepted files: ${supportedFiles.length}`]);
        const maxBrowserDropBytes = 64 * 1024 * 1024;
        const totalBrowserDropBytes = supportedFiles.reduce((sum, file) => sum + Number(file.size ?? 0), 0);
        if (totalBrowserDropBytes > maxBrowserDropBytes) {
          setLogs((items) => [
            ...items,
            `[drop] browser import blocked: ${formatMbValue(bytesToMb(totalBrowserDropBytes))} MB selected`,
            "[drop] Use native drag/drop paths or the Add Games button",
          ]);
          return;
        }
        const importPayload = [];
        for (const file of supportedFiles) {
          importPayload.push({
            name: file.name,
            bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
          });
        }
        importResult = await safeInvoke("import_rom_files_auto", { files: importPayload });
      }
    } catch (error) {
      setLogs((items) => [...items, `[rom] Drop import error: ${String(error)}`]);
      return;
    }

    if ((importResult?.warnings ?? []).length > 0) {
      setLogs((items) => [...items, `[drop] backend warnings: ${importResult.warnings.length}`]);
    }

    const groupedEntries = new Map();
    for (const entry of importResult?.entries ?? []) {
      const emulatorId = entry.emulator;
      const group = groupedEntries.get(emulatorId) ?? [];
      group.push(entry);
      groupedEntries.set(emulatorId, group);
    }

    let firstEmulatorId = null;
    const importedPerEmulator = new Map();
    const nextRomLibrary = { ...romLibrary };
    for (const [emulatorId, entries] of groupedEntries.entries()) {
      const existing = nextRomLibrary[emulatorId] ?? [];
      const offset = existing.length;
      const candidateGames = entries.map((entry, index) => toImportedGame(emulatorId, entry, offset + index));
      const { merged, uniqueImported } = mergeUniqueGames(existing, candidateGames);
      nextRomLibrary[emulatorId] = merged;
      importedPerEmulator.set(emulatorId, { candidateGames, uniqueImported });
      if (!firstEmulatorId && uniqueImported.length > 0) {
        firstEmulatorId = emulatorId;
      }
    }
    setRomLibrary(nextRomLibrary);

    if (firstEmulatorId) {
      setSelectedEmulator(firstEmulatorId);
      const firstEntry = groupedEntries.get(firstEmulatorId)?.[0];
      if (firstEntry) {
        setSelectedGameId(toImportedGame(firstEmulatorId, firstEntry, 0).id);
      }
      setBuilderSettingsOpen(true);
      setActiveAction("build");
    }

    const groupedLogs = [];
    for (const [emulatorId, entries] of groupedEntries.entries()) {
      const stats = importedPerEmulator.get(emulatorId);
      const addedCount = stats?.uniqueImported?.length ?? 0;
      const skippedCount = Math.max(0, (stats?.candidateGames?.length ?? 0) - addedCount);
      groupedLogs.push(`[rom] Dropped ${addedCount} ROM file(s) into ${emulatorId.toUpperCase()}`);
      if (skippedCount > 0) {
        groupedLogs.push(`[rom] Dropped duplicates skipped: ${skippedCount} for ${emulatorId.toUpperCase()}`);
      }
    }
    setLogs((items) => [
      ...items,
      ...groupedLogs,
      ...((importResult?.warnings ?? []).map((warning) => `[rom] ${warning}`)),
    ]);

    for (const [emulatorId, entries] of groupedEntries.entries()) {
      const importedGames = importedPerEmulator.get(emulatorId)?.uniqueImported ?? [];
      await hydrateCachedImages(emulatorId, importedGames);
    }
  }

  handleGlobalDropRef.current = handleGlobalDrop;

  useEffect(() => {
    let unlistenPromise;
    try {
      const windowHandle = getCurrentWindow();
      setNativeDragDropAvailable(true);
      const maybePromise = windowHandle.onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setIsDragActive(true);
          return;
        }
        if (event.payload.type === "leave") {
          setIsDragActive(false);
          return;
        }
        if (event.payload.type === "drop") {
          handleGlobalDropRef.current?.(event);
          return;
        }
        if (event.payload.type === "cancel") {
          setLogs((items) => [...items, `[drop] event: cancel`]);
          setIsDragActive(false);
        }
      });
      Promise.resolve(maybePromise)
        .then((unlisten) => {
          unlistenPromise = unlisten;
        })
        .catch((error) => {
          setLogs((items) => [...items, `[rom] Drag-drop listener error: ${String(error)}`]);
        });
    } catch (error) {
      setNativeDragDropAvailable(false);
      setLogs((items) => [...items, `[rom] Drag-drop listener error: ${String(error)}`]);
    }

    return () => {
      if (typeof unlistenPromise === "function") {
        unlistenPromise();
      }
    };
  }, []);

  function handleRemoveGame(gameId) {
    const removedGame = loadedGames.find((game) => game.id === gameId);
    if (!removedGame?.emulatorId) {
      return;
    }
    const currentGames = romLibrary[removedGame.emulatorId] ?? [];
    const removedKey = gameIdentityKey(removedGame);
    const nextGames = currentGames.filter((game) => gameIdentityKey(game) !== removedKey);

    setRomLibrary((prev) => ({
      ...prev,
      [removedGame.emulatorId]: nextGames,
    }));

    if (selectedGameId === gameId) {
      setSelectedGameId(nextGames[0]?.id ?? null);
    }

    if (removedGame) {
      setLogs((items) => [...items, `[rom] Removed from list: ${removedGame.title}`]);
    }
  }

  function handleClearGames() {
    const totalGames = loadedGames.length;
    if (totalGames === 0) {
      return;
    }
    setRomLibrary({});
    setSelectedGameId(null);
    setLogs((items) => [...items, `[rom] Cleared session ROM list: ${totalGames} item(s) removed`]);
  }

  async function handleFlash() {
    if (!hasCurrentBuiltExtflash) {
      setLogs((items) => [...items, "[flash] Auto Flash blocked: build firmware first"]);
      return;
    }

    let bank1Path = mcuFirmwarePath ?? mcuFirmwareFile;
    let bank2Path = bank2FirmwarePath ?? bank2FirmwareFile;
    let spiPath = spiFirmwarePath ?? spiFirmwareFile;
    const isBundleFirmwarePath = (path) => {
      const text = String(path ?? "").replaceAll("\\", "/").toLowerCase();
      return text.includes("/bundles/") && text.includes("/firmware/");
    };
    if (![bank1Path, bank2Path, spiPath].filter(Boolean).every(isBundleFirmwarePath)) {
      bank1Path = null;
      bank2Path = null;
      spiPath = null;
    }

    if (!spiPath || !bank1Path || !bank2Path) {
      setLogs((items) => [...items, "[flash] Auto Flash blocked: current build outputs are incomplete"]);
      return;
    }

    const flashPlan = [
      ["bank1", bank1Path],
      ["bank2", bank2Path],
      ["spi", spiPath],
    ];
    const displayRows = flashPlan.map(([kind, path]) => ({
      kind,
      label: kind === "bank1" ? "Bank1" : kind === "bank2" ? "Bank2" : "SPI",
      name: String(path).split(/[\\/]/).pop(),
    }));

    setIsFlashing(true);
    setFlashOperation("auto");
    setRestoreDisplayActive(null);
    setFlashDisplayRows(displayRows);
    setFlashPhaseLabel(t.autoFlashRunning);
    setFlashStage("");
    setFlashCompletionStatus(null);
    setFlashResult({ bank1: null, bank2: null, spi: null });
    setFlashProgress(8);
    setActiveAction("flash");
    setLogs((items) => [
      ...items,
      "[flash] Starting auto flash...",
      `[flash] Order: ${flashPlan.map(([kind]) => (kind === "spi" ? "SPI extflash" : kind === "bank1" ? "MCU bank1" : "MCU bank2")).join(" -> ")}`,
    ]);

    let completed = false;
    try {
      for (const [index, [kind, path]] of flashPlan.entries()) {
        if (!path) {
          throw new Error(`${kind.toUpperCase()} firmware path is missing`);
        }
        setActiveFlashPhase(kind);
        setFlashProgress(Math.max(12, Math.round((index / Math.max(1, flashPlan.length)) * 85) + 10));
        await writeFirmwarePhase(kind, path, { ...(kind === "spi" ? { externalFlashOffsetBytes: 0 } : {}), preserveOperation: true });
      }
      setFlashProgress(100);
      setFlashPhaseLabel(t.autoFlashDone);
      setFlashCompletionStatus("auto-success");
      completed = true;
      setLogs((items) => [...items, "[flash] Auto flash completed"]);
    } catch (error) {
      const message = String(error?.message ?? error);
      setFlashCompletionStatus(null);
      setLogs((items) => [...items, `[flash] AUTO ERROR: ${message}`]);
    } finally {
      window.setTimeout(() => {
        setIsFlashing(false);
        setFlashOperation(null);
        if (!completed) {
          setFlashDisplayRows([]);
          setFlashPhaseLabel("");
          setFlashProgress(0);
        }
        setFlashStage("");
        setActiveFlashPhase(null);
      }, 220);
    }
  }

  function requestAutoFlash() {
    setConfirmAction({
      title: t.flashBuildTitle,
      message: t.flashBuildConfirm,
      confirmText: t.flashBuildButton,
      tone: "emerald",
      onConfirm: () => {
        setConfirmAction(null);
        handleFlash();
      },
    });
  }

  async function writeFirmwarePhase(kind, path, options = {}) {
    if (!path) {
      throw new Error(`${kind.toUpperCase()} firmware path is missing`);
    }

    const command =
      kind === "bank1"
        ? "write_bank1_firmware"
        : kind === "bank2"
          ? "write_bank2_firmware"
          : "write_spi_firmware";
    const label = kind === "bank1" ? "Bank1" : kind === "bank2" ? "Bank2" : "SPI";
    const phaseLabel = kind === "bank1" ? t.writeBank1 : kind === "bank2" ? t.writeBank2 : t.writeSpiFlash;

    setActiveFlashPhase(kind);
    if (!options.preserveOperation) {
      setFlashOperation(kind);
      setFlashCompletionStatus(null);
    }
    setFlashPhaseLabel(phaseLabel);
    setFlashStage(kind === "spi" ? "write" : "");
    setFlashResult((prev) => ({ ...prev, [kind]: null }));
    setLogs((items) => [...items, `[flash] Writing ${label}: ${path}`]);
    const result = await safeInvoke(command, {
      backend: "pyocd",
      frequency: probeFrequencyHz,
      protection: deviceInfo.protection,
      path,
      externalFlashMb: detectedExternalFlashValue || 64,
      externalFlashOffsetBytes: options.externalFlashOffsetBytes ?? firmwareBaseBytes,
    });
    setLogs((items) => [
      ...items,
      `[flash] ${result.summary}`,
    ]);
    setFlashResult((prev) => ({
      ...prev,
      [kind]: { status: "success", message: result.summary ?? `${label} flash completed successfully` },
    }));
    return result;
  }

  async function handleRestoreDevice() {
    if (!hasKnownDeviceUid) {
      setLogs((items) => [...items, `[restore] ERROR: ${deviceUidIssue}`]);
      return;
    }

    if (recoveryMode) {
      const bank1Path = mcuFirmwarePath ?? mcuFirmwareFile;
      const bank2Path = bank2FirmwarePath ?? bank2FirmwareFile;
      const spiPath = spiFirmwarePath ?? spiFirmwareFile;
      if (!bank1Path || !spiPath) {
        setLogs((items) => [
          ...items,
          "[restore] ERROR: recovery restore needs Bank1 and SPI files",
          ...(bank1Path ? [] : ["[restore] Missing selected Bank1 firmware"]),
          ...(spiPath ? [] : ["[restore] Missing selected SPI flash"]),
        ]);
        return;
      }

      const restoreSteps = [
        ["bank1", bank1Path],
        ...(bank2Path ? [["bank2", bank2Path]] : []),
        ["spi", spiPath],
      ];
      const displayRows = restoreSteps.map(([kind, path]) => ({
        kind,
        label: kind === "bank1" ? "Bank1" : kind === "bank2" ? "Bank2" : "SPI",
        name: String(path).split(/[\\/]/).pop(),
      }));

      setIsFlashing(true);
      setFlashOperation("restore");
      setRestoreDisplayActive("restore");
      setFlashDisplayRows(displayRows);
      setActiveFlashPhase(null);
      setFlashPhaseLabel(t.recoveryRestoreMode);
      setFlashStage("");
      setFlashResult({ bank1: null, bank2: null, spi: null });
      setFlashProgress(0);
      setActiveAction("flash");
      setLogs((items) => [
        ...items,
        "[restore] Starting recovery restore from selected files...",
        `[restore] Source UID: ${deviceInfo.device_uid}`,
        `[restore] Bank1=${bank1Path}`,
        ...(bank2Path ? [`[restore] Bank2=${bank2Path}`] : []),
        `[restore] SPI=${spiPath}`,
        `[restore] Order: ${restoreSteps.map(([kind]) => (kind === "bank1" ? "Bank1" : kind === "bank2" ? "Bank2" : "SPI")).join(" -> ")}`,
      ]);

      try {
        for (const [index, [kind, path]] of restoreSteps.entries()) {
          setFlashProgress(Math.round((index / restoreSteps.length) * 90) + 5);
          await writeFirmwarePhase(kind, path, { preserveOperation: true });
        }
        setFlashProgress(100);
        setLogs((items) => [...items, "[restore] Recovery restore completed"]);
      } catch (error) {
        setLogs((items) => [...items, `[restore] ERROR: ${String(error?.message ?? error)}`]);
      } finally {
        window.setTimeout(() => {
          setIsFlashing(false);
          setFlashOperation(null);
          setRestoreDisplayActive(null);
          setFlashDisplayRows([]);
          setFlashPhaseLabel("");
          setFlashStage("");
          setFlashProgress(0);
          setActiveFlashPhase(null);
        }, 220);
      }
      return;
    }

    let restoreLookup;
    try {
      restoreLookup = await safeInvoke("lookup_restore_backups", {
        request: { device_uid: deviceInfo.device_uid },
      });
    } catch (error) {
      setLogs((items) => [...items, `[restore] ERROR: backup lookup failed: ${String(error?.message ?? error)}`]);
      return;
    }

    const restoreSteps = [
      ["bank1", restoreLookup.mcu_path],
      ...(restoreLookup.bank2_path ? [["bank2", restoreLookup.bank2_path]] : []),
      ["spi", restoreLookup.spi_path],
    ].filter(([, path]) => Boolean(path));
    const displayRows = restoreSteps.map(([kind, path]) => ({
      kind,
      label: kind === "bank1" ? "Bank1" : kind === "bank2" ? "Bank2" : "SPI",
      name: String(path).split(/[\\/]/).pop(),
    }));

    if (!restoreLookup.mcu_path || !restoreLookup.spi_path) {
      setLogs((items) => [
        ...items,
        "[restore] ERROR: saved device backup is incomplete",
        ...(restoreLookup.mcu_path ? [] : ["[restore] Missing saved Bank1 backup"]),
        ...(restoreLookup.spi_path ? [] : ["[restore] Missing saved SPI backup"]),
      ]);
      return;
    }

    setIsFlashing(true);
    setFlashOperation("restore");
    setRestoreDisplayActive("restore");
    setFlashDisplayRows(displayRows);
    setActiveFlashPhase(null);
    setFlashPhaseLabel(t.restoreDeviceRunning);
    setFlashStage("");
    setFlashResult({ bank1: null, bank2: null, spi: null });
    setFlashProgress(0);
    setActiveAction("flash");
    setLogs((items) => [
      ...items,
      "[restore] Starting device restore...",
      `[restore] Source UID: ${deviceInfo.device_uid}`,
      `[restore] Bank1=${restoreLookup.mcu_path}`,
      ...(restoreLookup.bank2_path ? [`[restore] Bank2=${restoreLookup.bank2_path}`] : []),
      `[restore] SPI=${restoreLookup.spi_path}`,
      `[restore] Order: ${restoreSteps.map(([kind]) => (kind === "bank1" ? "Bank1" : kind === "bank2" ? "Bank2" : "SPI")).join(" -> ")}`,
    ]);

    try {
      for (const [index, [kind, path]] of restoreSteps.entries()) {
        setFlashProgress(Math.round((index / restoreSteps.length) * 90) + 5);
        await writeFirmwarePhase(kind, path, { preserveOperation: true });
      }
      setFlashProgress(100);
      setLogs((items) => [...items, "[restore] Device restore completed"]);
    } catch (error) {
      setLogs((items) => [...items, `[restore] ERROR: ${String(error?.message ?? error)}`]);
    } finally {
      window.setTimeout(() => {
        setIsFlashing(false);
        setFlashOperation(null);
        setRestoreDisplayActive(null);
        setFlashDisplayRows([]);
        setFlashPhaseLabel("");
        setFlashStage("");
        setFlashProgress(0);
        setActiveFlashPhase(null);
      }, 220);
    }
  }

  function requestRestoreDevice() {
    setConfirmAction({
      title: t.restoreDevice,
      message: t.restoreDeviceConfirm.replace("{uid}", deviceInfo.device_uid),
      tone: "amber",
      onConfirm: () => {
        setConfirmAction(null);
        handleRestoreDevice();
      },
    });
  }

  async function handleRestoreOriginalFirmware(profile) {
    const hardwareLabel = profile === "Z" ? "Zelda" : "Mario";
    setOriginalFirmwarePickerOpen(false);
    if (!isDeviceWritable) {
      setLogs((items) => [...items, "[stock-restore] ERROR: device is not writable"]);
      return;
    }

    let stockLookup;
    try {
      stockLookup = await safeInvoke("lookup_stock_backups", {
        request: { device_uid: deviceInfo.device_uid, firmware_profile: profile },
      });
    } catch (error) {
      setLogs((items) => [...items, `[stock-restore] ERROR: stock lookup failed: ${String(error?.message ?? error)}`]);
      return;
    }

    if (!stockLookup.mcu_path || !stockLookup.spi_path) {
      setStockMcuBackupFile(stockLookup.mcu_name || null);
      setStockMcuBackupPath(stockLookup.mcu_path || null);
      setStockSpiBackupFile(stockLookup.spi_name || null);
      setStockSpiBackupPath(stockLookup.spi_path || null);
      setPendingStockRestoreProfile(profile);
      setStockFirmwarePrompt(true);
      setActiveAction("flash");
      setLogs((items) => [
        ...items,
        `[stock-restore] ${hardwareLabel} stock firmware is incomplete in StockFirmware`,
        "[stock-restore] Drop original Bank1 and SPI stock files to continue restore",
        ...(stockLookup.mcu_path ? [] : [`[stock-restore] Missing ${profile} stock Bank1`]),
        ...(stockLookup.spi_path ? [] : [`[stock-restore] Missing ${profile} stock SPI`]),
      ]);
      return;
    }

    const restoreSteps = [
      ["bank1", stockLookup.mcu_path],
      ["spi", stockLookup.spi_path],
    ];
    const displayRows = restoreSteps.map(([kind, path]) => ({
      kind,
      label: kind === "bank1" ? "Bank1" : "SPI",
      name: String(path).split(/[\\/]/).pop(),
    }));

    setIsFlashing(true);
    setFlashOperation("stock-restore");
    setRestoreDisplayActive("stock-restore");
    setFlashDisplayRows(displayRows);
    setActiveFlashPhase(null);
    setFlashPhaseLabel(t.restoreOriginalFirmware);
    setFlashStage("");
    setFlashResult({ bank1: null, bank2: null, spi: null });
    setFlashProgress(0);
    setActiveAction("flash");
    setLogs((items) => [
      ...items,
      `[stock-restore] Starting original ${hardwareLabel} firmware restore`,
      "[stock-restore] This does not convert hardware model",
      `[stock-restore] Bank1=${stockLookup.mcu_path}`,
      `[stock-restore] SPI=${stockLookup.spi_path}`,
      "[stock-restore] SPI offset=0",
    ]);

    try {
      for (const [index, [kind, path]] of restoreSteps.entries()) {
        setFlashProgress(Math.round((index / restoreSteps.length) * 90) + 5);
        await writeFirmwarePhase(kind, path, { preserveOperation: true, ...(kind === "spi" ? { externalFlashOffsetBytes: 0 } : {}) });
      }
      setFlashProgress(100);
      setLogs((items) => [...items, `[stock-restore] Original ${hardwareLabel} firmware restore completed`]);
    } catch (error) {
      setLogs((items) => [...items, `[stock-restore] ERROR: ${String(error?.message ?? error)}`]);
    } finally {
      window.setTimeout(() => {
        setIsFlashing(false);
        setFlashOperation(null);
        setRestoreDisplayActive(null);
        setFlashDisplayRows([]);
        setFlashPhaseLabel("");
        setFlashStage("");
        setFlashProgress(0);
        setActiveFlashPhase(null);
      }, 220);
    }
  }

  async function runFirmwareWrite(kind, path) {
    setIsFlashing(true);
    setFlashOperation(kind);
    setRestoreDisplayActive(null);
    setFlashDisplayRows([]);
    setFlashStage(kind === "spi" ? "write" : "");
    setFlashProgress(8);
    setActiveAction("flash");
    setFlashResult((prev) => ({ ...prev, [kind]: null }));

    try {
      setFlashProgress(22);
      await writeFirmwarePhase(kind, path);
      setFlashProgress(100);
    } catch (error) {
      const message = String(error?.message ?? error);
      setFlashResult((prev) => ({
        ...prev,
        [kind]: { status: "error", message },
      }));
      setLogs((items) => [...items, `[flash] ${kind.toUpperCase()} ERROR: ${message}`]);
    } finally {
      window.setTimeout(() => {
        setIsFlashing(false);
      setFlashOperation(null);
      setFlashPhaseLabel("");
      setFlashStage("");
      setFlashProgress(0);
      setActiveFlashPhase(null);
      }, 220);
    }
  }

  return (
    <div
      className="h-screen bg-black text-white font-sans overflow-hidden"
      style={{ "--gw-marquee-rgb": marqueeRgb }}
      onDragOver={(event) => {
        if (!nativeDragDropAvailable) {
          event.preventDefault();
        }
      }}
      onDrop={nativeDragDropAvailable ? undefined : handleGlobalDrop}
    >
      {startupLoading && <StartupOverlay sha256={appSha256} progress={startupProgress} message={startupMessage} />}
      <main className="h-screen min-w-0 flex flex-col overflow-hidden">
        {isDragActive && (
          <div className="pointer-events-none fixed inset-0 z-[100] flex items-center justify-center bg-black/55 backdrop-blur-[2px]">
            <div className={cx("rounded-[28px] border px-8 py-6 text-center shadow-[0_0_40px_rgba(0,0,0,0.45)]", mode.accentBorder, mode.accentBgSoft)}>
              <div className="text-4xl">📥</div>
              <div className="mt-3 text-xl font-black text-white">Drop ROM or image files</div>
              <div className="mt-2 text-sm text-zinc-300">ROMs fill emulator lists, matching images are applied to loaded games</div>
            </div>
          </div>
        )}
        {originalFirmwarePickerOpen && (
          <OriginalFirmwarePicker
            mode={mode}
            t={t}
            onSelect={handleRestoreOriginalFirmware}
            onCancel={() => setOriginalFirmwarePickerOpen(false)}
          />
        )}
        {confirmAction && (
          <ConfirmDialog
            mode={mode}
            t={t}
            title={confirmAction.title}
            message={confirmAction.message}
            confirmText={confirmAction.confirmText}
            cancelText={confirmAction.cancelText}
            tone={confirmAction.tone}
            onConfirm={confirmAction.onConfirm}
            onCancel={() => setConfirmAction(null)}
          />
        )}
        {stockFirmwarePrompt && (
          <StockFirmwareDropDialog
            mode={mode}
            t={t}
            gate={stockFirmwarePromptGate}
            onSelectMcu={() => handleImportStockBackup("mcu")}
            onSelectSpi={() => handleImportStockBackup("spi")}
            onCancel={() => {
              setStockFirmwarePrompt(false);
              setRetryBuildAfterStockFirmware(false);
              setPendingStockRestoreProfile(null);
            }}
          />
        )}
        {msxBiosPrompt && (
          <BiosDropDialog
            mode={mode}
            t={t}
            status={msxBiosPrompt}
            title={t.msxBiosRequired}
            hint={t.msxBiosHint}
            onCancel={() => {
              setMsxBiosPrompt(null);
              setRetryBuildAfterMsxBios(false);
            }}
          />
        )}
        {colecoBiosPrompt && (
          <BiosDropDialog
            mode={mode}
            t={t}
            status={colecoBiosPrompt}
            title={t.colecoBiosRequired}
            hint={t.colecoBiosHint}
            onCancel={() => {
              setColecoBiosPrompt(null);
              setRetryBuildAfterColecoBios(false);
            }}
          />
        )}
        <header className="gw-header h-[76px] shrink-0 border-b border-zinc-900 bg-[#050505]/90 backdrop-blur flex items-center justify-between px-6 shadow-[0_1px_0_rgba(255,255,255,0.03)]">
          <div className="flex items-center gap-5">
            <div className={cx("flex h-12 w-12 items-center justify-center rounded-2xl text-base font-black shadow-lg", mode.accentBg)}>
              GW
            </div>
            <div className="text-3xl font-black leading-8">GW Studio</div>
            <div className="rounded-full border border-zinc-800 px-3 py-1 font-mono text-xs text-zinc-500">v{APP_VERSION}</div>
          </div>

          <div className="flex items-center gap-3 rounded-3xl border border-zinc-800 bg-black/30 px-3 py-2 shadow-[0_0_25px_rgba(0,0,0,0.35)]">
            <button
              type="button"
              onClick={() => setSettingsOpen(true)}
              className="relative flex h-11 w-11 items-center justify-center rounded-2xl border border-zinc-800 bg-black/40 text-lg text-zinc-300 transition-all duration-300 hover:border-zinc-600 hover:bg-zinc-900"
            >
              ⚙
              {updateAvailable && (
                <span className="absolute -right-1 -top-1 flex h-5 w-5 animate-pulse items-center justify-center rounded-full bg-red-600 text-[11px] font-black text-white shadow-[0_0_18px_rgba(239,68,68,0.8)]">
                  !
                </span>
              )}
            </button>
          </div>
        </header>

        <div className="overflow-y-auto overflow-x-hidden px-5 pb-6" style={{ background: `radial-gradient(circle at top, ${mode.glow}, transparent 44%)` }}>
          <div className="grid auto-rows-max gap-4 py-4">
            <div className="grid grid-cols-[minmax(280px,360px)_minmax(520px,1fr)] items-stretch gap-4">
              <DeviceStatus
                mode={mode}
                t={t}
                onReadInfo={handleReadInfo}
                isReadingInfo={isReadingInfo}
                readInfoProgress={readInfoProgress}
                deviceInfo={deviceInfo}
                showUidWarning={hasReadDeviceInfo}
              />
              <DeviceMockup
                mode={mode}
                modeName={visualModeName}
                builderSettingsOpen={builderSettingsOpen}
                selectedEmulator={selectedEmulator}
                selectedGame={selectedGame}
                sceneTuning={currentSceneTuning}
                screenFrame={currentScreenFrame}
                showConsole={Boolean(visualModeName)}
                readInfoIssue={readInfoIssue}
              />
            </div>

            <div className="grid grid-cols-3 items-stretch gap-4">
              <ActionCard
                title={t.backupShort}
                desc={!isDeviceIdentified ? deviceIdentifyIssue : fullBackupDone ? t.fullBackupReady : t.readOrSelectBackup}
                icon="▣"
                accent
                mode={mode}
                tone="blue"
                disabled={!isDeviceIdentified}
                active={activeAction === "backup"}
                onClick={() => {
                  if (!isDeviceIdentified) {
                    setLogs((items) => [...items, `[device] Backup blocked: ${deviceIdentifyIssue}`]);
                    return;
                  }
                  setBuilderSettingsOpen(false);
                  setSelectedEmulator(null);
                  setSelectedGameId(null);
                  setActiveAction("backup");
                }}
              />
              <ActionCard
                title={t.buildEmulator}
                desc={builderSettingsOpen ? t.configureEmulatorCores : t.openEmulatorConfig}
                icon="🎮"
                accent
                mode={mode}
                tone="purple"
                disabled={!isDeviceIdentified}
                active={builderSettingsOpen}
                onClick={() => {
                  if (!isDeviceIdentified) {
                    setLogs((items) => [...items, `[device] Build Emulator blocked: ${deviceIdentifyIssue}`]);
                    return;
                  }
                  setBuilderSettingsOpen(!builderSettingsOpen);
                  setActiveAction(builderSettingsOpen ? "backup" : "build");
                  if (builderSettingsOpen) {
                    setSelectedEmulator(null);
                    setSelectedGameId(null);
                  }
                }}
              />
              <ActionCard
                title={t.flashDevice}
                desc={recoveryMode ? t.recoveryRestoreMode : isDeviceWritable ? t.writeFirmwareToDevice : t.protectedDevice}
                icon={mode.quickIcon}
                accent
                mode={mode}
                disabled={isStmBusy || !isDeviceWritable}
                active={activeAction === "flash"}
                onClick={async () => {
                  if (!isDeviceWritable) {
                    setLogs((items) => [...items, "[flash] Flash Device blocked: device is not writable"]);
                    return;
                  }
                  setBuilderSettingsOpen(false);
                  setActiveAction("flash");
                  await primeLatestFirmwareBundle();
                }}
              />
            </div>

            <div
              className={clsx(
                "gw-work-grid grid items-stretch gap-4",
                builderSettingsOpen ? "grid-cols-[280px_minmax(0,1fr)_360px]" : "grid-cols-[minmax(0,1fr)_360px]",
              )}
            >
              {builderSettingsOpen && (
                <EmulatorPicker
                  mode={mode}
                  t={t}
                  selectedEmulator={selectedEmulator}
                  onAddGames={handleAddGames}
                  onSelect={(id) => {
                    setSelectedEmulator(id);
                    setSelectedGameId(null);
                  }}
                />
              )}

              <div className="min-w-0 min-h-0 h-full">
                {showRecoveryPrompt ? (
                  <RecoveryPrompt
                    mode={mode}
                    t={t}
                    deviceUid={deviceInfo.device_uid}
                    onConfirm={handleStartRecovery}
                    onDecline={handleDeclineRecovery}
                  />
                ) : activeAction === "flash" ? (
                  <FlashPanel
                    t={t}
                    mode={mode}
                    isDeviceTransferActive={isStmBusy}
                    isDeviceWritable={isDeviceWritable}
                    flashOperation={flashOperation}
                    restoreDisplayActive={restoreDisplayActive}
                    activeFlashPhase={activeFlashPhase}
                    flashPhaseLabel={flashPhaseLabel}
                    flashStage={flashStage}
                    flashProgress={flashProgress}
                    flashResult={flashResult}
                    flashCompletionStatus={flashCompletionStatus}
                    flashDisplayRows={flashDisplayRows}
                    advancedFlashEnabled={advancedFlasherEnabled}
                    recoveryMode={recoveryMode}
                    hasCurrentFirmwareBuild={hasCurrentBuiltExtflash}
                    onAutoFlash={requestAutoFlash}
                    onRestoreDevice={requestRestoreDevice}
                    onRestoreOriginalFirmware={() => setOriginalFirmwarePickerOpen(true)}
                    onSelectMcuFirmware={() => {
                      handleSelectOrRevealFirmware("mcu");
                      setActiveAction("flash");
                    }}
                    onSelectBank2Firmware={() => {
                      handleSelectOrRevealFirmware("bank2");
                      setActiveAction("flash");
                    }}
                    onSelectSpiFirmware={() => {
                      handleSelectOrRevealFirmware("spi");
                      setActiveAction("flash");
                    }}
                    onWriteMcuBackup={() => runFirmwareWrite("bank1", mcuFirmwarePath ?? mcuFirmwareFile)}
                    onWriteBank2Backup={() => runFirmwareWrite("bank2", bank2FirmwarePath ?? bank2FirmwareFile)}
                    onWriteSpiBackup={() => runFirmwareWrite("spi", spiFirmwarePath ?? spiFirmwareFile)}
                    mcuBackupFile={mcuBackupFile}
                    bank2BackupFile={bank2BackupFile}
                    spiBackupFile={spiBackupFile}
                    mcuFirmwareFile={mcuFirmwareFile}
                    mcuFirmwarePath={mcuFirmwarePath}
                    bank2FirmwareFile={bank2FirmwareFile}
                    bank2FirmwarePath={bank2FirmwarePath}
                    spiFirmwareFile={spiFirmwareFile}
                    spiFirmwarePath={spiFirmwarePath}
                    firmwareDirectoryHint="Build firmware to prepare flashable outputs"
                  />
                ) : activeAction === "backup" ? (
                    <BackupPanel
                      mode={mode}
                      t={t}
                      advancedBackupEnabled={advancedFlasherEnabled}
                      fullBackupDone={fullBackupDone}
                      onAutoBackup={handleAutoBackup}
                      onReadMcuBackup={handleReadMcuBackup}
                      onReadBank2Backup={handleReadBank2Backup}
                      onReadSpiBackup={handleReadSpiBackup}
                      onSelectMcuBackup={() => {
                        handleSelectBackupFile("mcu");
                        setActiveAction("backup");
                      }}
                      onSelectBank2Backup={() => {
                        handleSelectBackupFile("bank2");
                        setActiveAction("backup");
                      }}
                      onSelectSpiBackup={() => {
                        handleSelectBackupFile("spi");
                        setActiveAction("backup");
                      }}
                      onRevealMcuBackup={() => handleRevealBackup("mcu")}
                      onRevealBank2Backup={() => handleRevealBackup("bank2")}
                      onRevealSpiBackup={() => handleRevealBackup("spi")}
                      mcuBackupFile={mcuBackupFile}
                      mcuBackupPath={mcuBackupPath}
                      bank2BackupFile={bank2BackupFile}
                      bank2BackupPath={bank2BackupPath}
                      spiBackupFile={spiBackupFile}
                      spiBackupPath={spiBackupPath}
                      isAutoBackupRunning={isAutoBackupRunning}
                      isManualBackupRunning={isManualBackupRunning}
                      isDeviceTransferActive={isStmBusy}
                      readingBackupKind={readingBackupKind}
                      backupProgressState={backupProgressState}
                      canAutoBackup={canAutoBackup}
                      canSelectBackupFiles={canSelectBackupFiles}
                      backupReadReason={backupReadReason}
                      backupDirectoryHint={backupDirectoryHint}
                    />
                ) : builderSettingsOpen ? (
                  <GameListPanel
                    mode={mode}
                    t={t}
                    selectedEmulator={selectedEmulator}
                    games={sortedLoadedGames}
                    selectedGameId={selectedGameId}
                    onSelectGame={setSelectedGameId}
                    onRemoveGame={handleRemoveGame}
                    onClearGames={handleClearGames}
                    onImportImagesPack={() => imagePackInputRef.current?.click()}
                    onScanImagesFolder={() => imageFolderInputRef.current?.click()}
                    onSetGameImage={(gameId) => {
                      setPendingManualImageGameId(gameId);
                      manualImageInputRef.current?.click();
                    }}
                    sortMode={gameSortMode}
                    onSortModeChange={setGameSortMode}
                  />
                ) : null}
              </div>

              <div className="sticky top-4 self-start min-h-0 h-full">
                <BuildActionsPanel
                  t={t}
                  mode={mode}
                  selectedEmulator={selectedEmulator}
                  hasLoadedGames={loadedGames.length > 0}
                  isBuildingFirmware={isBuildingFirmware}
                  buildFirmwareProgress={buildFirmwareProgress}
                  buildFirmwareMessage={buildFirmwareMessage}
                  buildFirmwareError={buildFirmwareError}
                  isDownloadingImages={isDownloadingImages}
                  imageDownloadProgress={imageDownloadProgress}
                  canDownloadImages={canDownloadImages}
                  isDeviceWritable={isDeviceWritable}
                  isDeviceIdentified={isDeviceIdentified}
                  detectedFirmware={detectedFirmwareLabel}
                  hasReadDeviceInfo={hasReadDeviceInfo}
                  sdEnabled={sdEnabled}
                  spiUsedMb={spiUsedMb}
                  spiTotalMb={spiTotalMb}
                  sdUsedMb={sdUsedMb}
                  sdTotalMb={sdTotalMb}
                  firmwareBaseMb={firmwareBaseMb}
                  emulatorUsedMb={emulatorUsedMb}
                  estimatedEmulatorUsedMb={estimatedEmulatorUsedMb}
                  builtExtflashMb={hasCurrentBuiltExtflash ? builtExtflashMb : 0}
                  romUsedMb={romUsedMb}
                  imageUsedMb={imageUsedMb}
                  romCount={buildMetrics.romCount}
                  imageCount={buildMetrics.imageCount}
                  onDownloadImages={handleDownloadImages}
                  onBuildFirmware={handleBuildFirmware}
                />
              </div>
            </div>

            <div className="grid grid-cols-1 gap-4 pb-4">
              <LiveLog mode={mode} t={t} lines={logs} onClear={() => setLogs([])} />
            </div>
          </div>
        </div>

        {settingsOpen && (
          <div className="fixed bottom-4 right-3 top-4 z-50 w-[min(420px,calc(100vw-24px))] sm:right-6 sm:top-6">
            <Panel className="flex h-full max-h-[calc(100vh-2rem)] flex-col overflow-hidden border-zinc-700/90 bg-[rgba(10,10,12,0.96)] p-5 shadow-[0_0_40px_rgba(0,0,0,0.55)] sm:p-6">
              <div className="mb-4 flex shrink-0 items-center justify-between sm:mb-6">
                <div>
                  <div className={cx("text-sm font-black uppercase tracking-wide", mode.accentText)}>
                    {t.settings}
                  </div>
                  <div className="mt-1 text-xs text-zinc-500">{t.languageUpdate}</div>
                </div>
                <button type="button" onClick={() => setSettingsOpen(false)} className="flex h-10 w-10 items-center justify-center rounded-xl border border-zinc-800 bg-black/40 text-zinc-400 transition hover:border-zinc-600 hover:text-white">
                  ×
                </button>
              </div>

                <div className="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1 sm:space-y-5 sm:pr-2">
                  <div>
                    <div className="mb-3 text-xs font-bold uppercase tracking-wide text-zinc-500">{t.language}</div>
                  <div className="grid grid-cols-3 gap-3">
                    {[
                      ["ru", "Рус"],
                      ["en", "Eng"],
                      ["uk", "Укр"],
                    ].map(([id, label]) => (
                      <button
                        key={id}
                        type="button"
                        onClick={() => setLanguage(id)}
                        className={cx(
                          "rounded-2xl border px-4 py-4 text-sm font-black transition",
                          language === id ? cx(mode.accentBorder, mode.accentBgSoft, mode.accentText) : "border-zinc-800 bg-black/35 text-zinc-500 hover:border-zinc-600 hover:text-zinc-200",
                        )}
                      >
                        {label}
                      </button>
                    ))}
                    </div>
                  </div>

                  <div className="rounded-2xl border border-zinc-900 bg-black/35 p-4">
                    <div className="mb-3 flex items-center justify-between">
                      <div>
                        <div className="text-sm font-black text-zinc-100">{t.probeFrequency}</div>
                        <div className="mt-1 text-xs text-zinc-500">
                          {t.probeFrequencyHint.replace("{khz}", probeFrequencyKhz)}
                        </div>
                      </div>
                      <div className={cx("rounded-xl border px-3 py-2 text-sm font-black", mode.accentBorder, mode.accentText)}>
                        {probeFrequencyKhz} kHz
                      </div>
                    </div>

                    <input
                      type="range"
                      min={10}
                      max={8000}
                      step={10}
                      value={probeFrequencyKhz}
                      onChange={(event) => setProbeFrequencyKhz(Number(event.target.value))}
                      className="h-2 w-full cursor-pointer appearance-none rounded-lg bg-zinc-200 accent-red-500"
                    />

                    <div className="mt-2 flex justify-between text-[11px] text-zinc-500">
                      <span>10 kHz</span>
                      <span>8000 kHz</span>
                    </div>
                  </div>

                  <div className="rounded-2xl border border-zinc-900 bg-black/35 p-4">
                    <div className="flex items-center justify-between gap-4">
                      <div>
                        <div className="text-sm font-black text-zinc-100">{t.advancedFlasher}</div>
                        <div className="mt-1 text-xs text-zinc-500">
                          {t.advancedFlasherHint}
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setAdvancedFlasherEnabled((value) => !value)}
                        className={cx(
                          "rounded-xl border px-3 py-2 text-xs font-black uppercase tracking-wide transition",
                          advancedFlasherEnabled
                            ? cx(mode.accentBorder, mode.accentBgSoft, mode.accentText)
                            : "border-zinc-800 bg-black/35 text-zinc-500 hover:border-zinc-600 hover:text-zinc-200",
                        )}
                      >
                        {advancedFlasherEnabled ? t.on : t.off}
                      </button>
                    </div>
                  </div>

                  <div className="rounded-2xl border border-zinc-900 bg-black/35 p-4">
                    <div className="mb-3 flex items-center justify-between">
                      <div>
                      <div className="text-sm font-black text-zinc-100">{t.programUpdate}</div>
                      <div className="mt-1 text-xs text-zinc-500">{updateAvailable ? t.updateAvailable : t.upToDate}</div>
                    </div>
                    {updateAvailable && (
                      <div className="rounded-full bg-red-600 px-3 py-1 text-xs font-black text-white">
                        !
                      </div>
                    )}
                  </div>
                  <button
                    type="button"
                    onClick={() => checkForAppUpdate({ interactive: true })}
                    disabled={isCheckingUpdate || isInstallingUpdate}
                    className={cx(
                      "w-full rounded-2xl border px-5 py-4 text-left text-sm font-black transition",
                      isCheckingUpdate || isInstallingUpdate
                        ? "cursor-wait border-zinc-900 bg-black/30 text-zinc-600"
                        : cx(mode.accentBorder, mode.accentBgSoft, mode.accentText, "hover:bg-zinc-900"),
                    )}
                  >
                    {isInstallingUpdate ? t.installingUpdate : isCheckingUpdate ? t.checkingUpdate : t.updateProgram}
                  </button>
                </div>

                  <div className="grid grid-cols-2 gap-3">
                    <button
                      type="button"
                      onClick={() => openExternalUrl(GITHUB_REPOSITORY_URL)}
                      className="rounded-2xl border border-zinc-800 bg-black/35 px-5 py-4 text-left text-sm font-black text-zinc-200 transition hover:border-zinc-600 hover:bg-zinc-900"
                    >
                      {t.githubRepository}
                    </button>
                    <button
                      type="button"
                      onClick={() => openExternalUrl(PAYPAL_THANKS_URL)}
                      className="rounded-2xl border border-zinc-800 bg-black/35 px-5 py-4 text-left text-sm font-black text-zinc-200 transition hover:border-zinc-600 hover:bg-zinc-900"
                    >
                      {t.sayThanks}
                    </button>
                  </div>

              </div>
            </Panel>
          </div>
        )}
      </main>
      <input
        ref={emulatorRomInputRef}
        type="file"
        multiple
        accept={pendingRomEmulator ? EMULATOR_FILE_ACCEPT[pendingRomEmulator] ?? "" : ""}
        className="hidden"
        onChange={handleRomFilesPicked}
      />
      <input
        ref={imagePackInputRef}
        type="file"
        multiple
        accept=".png,.jpg,.jpeg,.webp"
        className="hidden"
        onChange={handleImagePackPicked}
      />
      <input
        ref={imageFolderInputRef}
        type="file"
        multiple
        accept=".png,.jpg,.jpeg,.webp"
        webkitdirectory="true"
        directory=""
        className="hidden"
        onChange={handleImageFolderPicked}
      />
      <input
        ref={manualImageInputRef}
        type="file"
        accept=".png,.jpg,.jpeg,.webp"
        className="hidden"
        onChange={handleManualImagePicked}
      />
    </div>
  );
}
