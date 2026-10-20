/**
 * 自定义字体（#79）：内置系统字体预设 + 上传自己的字体文件。
 *
 * 约定：
 * - 预设只写 font-family（系统字体，不打包文件）；上传文件走 IndexedDB，见 font-store.ts。
 * - 全站文字走 `--font-ui`；代码/路径保留 `--font-mono`。
 * - 落 <html>：data-font + --font-ui；偏好只存 localStorage，不写存档。
 */

export type FontPresetId = "song" | "hei" | "kai" | "fang" | "mono" | "system";
export type FontChoiceId = FontPresetId | "custom";

export interface FontPreset {
  id: FontPresetId;
  name: string;
  family: string;
}

export const FONT_FALLBACK = '"SimSun", "宋体", "Songti SC", serif';
export const CUSTOM_FONT_FAMILY = "FarmCustom";

export const FONT_PRESETS: readonly FontPreset[] = [
  { id: "song", name: "宋体", family: FONT_FALLBACK },
  {
    id: "hei",
    name: "黑体",
    family: '"Microsoft YaHei", "微软雅黑", "PingFang SC", sans-serif',
  },
  { id: "kai", name: "楷体", family: '"KaiTi", "楷体", "STKaiti", serif' },
  { id: "fang", name: "仿宋", family: '"FangSong", "仿宋", "STFangsong", serif' },
  {
    id: "mono",
    name: "等宽",
    family: '"Consolas", "Courier New", "SimSun", monospace',
  },
  {
    id: "system",
    name: "系统",
    family: 'system-ui, "Segoe UI", "Microsoft YaHei", sans-serif',
  },
];

export const DEFAULT_FONT_PRESET: FontPresetId = "song";

export const FONT_KEYS = {
  id: "farm.ui.fontId",
  name: "farm.ui.fontName",
} as const;

export interface FontPrefs {
  id: FontChoiceId;
  /** 自定义字体的显示名（不含扩展名）；预设时为空 */
  name: string;
}

export const DEFAULT_FONT_PREFS: FontPrefs = { id: DEFAULT_FONT_PRESET, name: "" };

/** localStorage 的最小接口；单测用内存实现。 */
export interface FontStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** `<html>` 的最小接口。 */
export interface FontRoot {
  dataset: { [key: string]: string | undefined };
  style: { setProperty(name: string, value: string): void };
}

export function isFontPresetId(value: unknown): value is FontPresetId {
  return FONT_PRESETS.some((preset) => preset.id === value);
}

export function isFontChoiceId(value: unknown): value is FontChoiceId {
  return value === "custom" || isFontPresetId(value);
}

export function sanitizeFontName(value: unknown): string {
  if (typeof value !== "string") return "";
  return value.replace(/\s+/g, " ").trim().slice(0, 40);
}

export function normalizeFontPrefs(
  prefs: Partial<Record<keyof FontPrefs, unknown>> | null | undefined,
): FontPrefs {
  const source = prefs ?? {};
  const rawId = source.id;
  const id = isFontChoiceId(rawId) ? rawId : DEFAULT_FONT_PRESET;
  // 预设也保留自定义字体的显示名：下次点「自定义」还能复用已存文件。
  const name = isFontChoiceId(rawId) ? sanitizeFontName(source.name) : "";
  if (id === "custom" && !name) {
    return { ...DEFAULT_FONT_PREFS };
  }
  return { id, name };
}

export function readFontPrefs(storage: FontStorage | null | undefined): FontPrefs {
  if (!storage) return { ...DEFAULT_FONT_PREFS };
  try {
    return normalizeFontPrefs({
      id: storage.getItem(FONT_KEYS.id),
      name: storage.getItem(FONT_KEYS.name),
    });
  } catch {
    return { ...DEFAULT_FONT_PREFS };
  }
}

export function writeFontPrefs(
  storage: FontStorage | null | undefined,
  prefs: FontPrefs,
): boolean {
  if (!storage) return false;
  const safe = normalizeFontPrefs(prefs);
  try {
    storage.setItem(FONT_KEYS.id, safe.id);
    storage.setItem(FONT_KEYS.name, safe.name);
    return true;
  } catch {
    return false;
  }
}

/** 预设/自定义 → 可直接写进 `--font-ui` 的 font-family 值。 */
export function fontFamilyFor(id: FontChoiceId, customFamily?: string): string {
  if (id === "custom") {
    return customFamily
      ? `"${customFamily.replace(/"/g, "")}", ${FONT_FALLBACK}`
      : FONT_FALLBACK;
  }
  return FONT_PRESETS.find((preset) => preset.id === id)?.family ?? FONT_FALLBACK;
}

export function applyFontPrefs(
  root: FontRoot,
  prefs: FontPrefs,
  customFamily?: string,
): FontPrefs {
  const safe = normalizeFontPrefs(prefs);
  root.dataset.font = safe.id;
  root.style.setProperty("--font-ui", fontFamilyFor(safe.id, customFamily));
  return safe;
}
