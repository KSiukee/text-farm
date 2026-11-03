/**
 * 背景与不透明度：纯色 / 纸纹 / 自定义图片 + 双滑杆。
 *
 * 约定：
 * - 颜色仍归 theme.css；这里只决定背景模式、纸纹、图片铺法与两档透明度。
 * - 图片本体不进 localStorage（5MB 风险），存 IndexedDB，见 background-store.ts。
 * - 落 <html>：data-bg-mode / data-bg-paper / data-bg-fit + --panel-alpha / --bg-image-alpha。
 */

export type BackgroundMode = "solid" | "paper" | "image";
export type PaperPattern = "grain" | "dots" | "grid";
export type BackgroundFit = "cover" | "contain" | "tile";

export interface BackgroundPrefs {
  mode: BackgroundMode;
  paper: PaperPattern;
  fit: BackgroundFit;
  /** 背景图不透明度 0–100 */
  imageAlpha: number;
  /** 面板不透明度 60–100 */
  panelAlpha: number;
}

export const BACKGROUND_MODES: readonly { id: BackgroundMode; name: string }[] = [
  { id: "solid", name: "纯色" },
  { id: "paper", name: "纸纹" },
  { id: "image", name: "图片" },
];

export const PAPER_PATTERNS: readonly { id: PaperPattern; name: string }[] = [
  { id: "grain", name: "细颗粒" },
  { id: "dots", name: "点阵" },
  { id: "grid", name: "方格" },
];

export const BACKGROUND_FITS: readonly { id: BackgroundFit; name: string }[] = [
  { id: "cover", name: "铺满" },
  { id: "contain", name: "完整" },
  { id: "tile", name: "平铺" },
];

export const DEFAULT_BACKGROUND_PREFS: BackgroundPrefs = {
  mode: "solid",
  paper: "grain",
  fit: "cover",
  imageAlpha: 30,
  panelAlpha: 92,
};

export const BACKGROUND_KEYS = {
  mode: "farm.ui.bgMode",
  paper: "farm.ui.bgPaper",
  fit: "farm.ui.bgFit",
  imageAlpha: "farm.ui.bgImageAlpha",
  panelAlpha: "farm.ui.bgPanelAlpha",
} as const;

export interface BackgroundStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** 输入允许是 localStorage 的原始字符串/坏值，统一由 normalize 收口。 */
export type BackgroundPrefsInput = Partial<Record<keyof BackgroundPrefs, unknown>>;

/** `<html>` 的最小接口，单测不依赖 DOM。 */
export interface BackgroundRoot {
  dataset: { [key: string]: string | undefined };
  style: { setProperty(name: string, value: string): void };
}

export function isBackgroundMode(value: unknown): value is BackgroundMode {
  return BACKGROUND_MODES.some((mode) => mode.id === value);
}

export function isPaperPattern(value: unknown): value is PaperPattern {
  return PAPER_PATTERNS.some((pattern) => pattern.id === value);
}

export function isBackgroundFit(value: unknown): value is BackgroundFit {
  return BACKGROUND_FITS.some((fit) => fit.id === value);
}

/** 数字收口：非法值回默认，四舍五入后夹在 [min,max]。 */
export function clampPercent(
  value: unknown,
  min: number,
  max: number,
  fallback: number,
): number {
  if (value === null || value === undefined || value === "") return fallback;
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.min(max, Math.max(min, Math.round(parsed)));
}

export function normalizeBackgroundPrefs(
  prefs: BackgroundPrefsInput | null | undefined,
): BackgroundPrefs {
  const source = prefs ?? {};
  return {
    mode: isBackgroundMode(source.mode) ? source.mode : DEFAULT_BACKGROUND_PREFS.mode,
    paper: isPaperPattern(source.paper) ? source.paper : DEFAULT_BACKGROUND_PREFS.paper,
    fit: isBackgroundFit(source.fit) ? source.fit : DEFAULT_BACKGROUND_PREFS.fit,
    imageAlpha: clampPercent(
      source.imageAlpha,
      0,
      100,
      DEFAULT_BACKGROUND_PREFS.imageAlpha,
    ),
    panelAlpha: clampPercent(
      source.panelAlpha,
      60,
      100,
      DEFAULT_BACKGROUND_PREFS.panelAlpha,
    ),
  };
}

export function readBackgroundPrefs(
  storage: BackgroundStorage | null | undefined,
): BackgroundPrefs {
  if (!storage) return { ...DEFAULT_BACKGROUND_PREFS };
  try {
    return normalizeBackgroundPrefs({
      mode: storage.getItem(BACKGROUND_KEYS.mode),
      paper: storage.getItem(BACKGROUND_KEYS.paper),
      fit: storage.getItem(BACKGROUND_KEYS.fit),
      imageAlpha: storage.getItem(BACKGROUND_KEYS.imageAlpha),
      panelAlpha: storage.getItem(BACKGROUND_KEYS.panelAlpha),
    });
  } catch {
    return { ...DEFAULT_BACKGROUND_PREFS };
  }
}

export function writeBackgroundPrefs(
  storage: BackgroundStorage | null | undefined,
  prefs: BackgroundPrefs,
): boolean {
  if (!storage) return false;
  const safe = normalizeBackgroundPrefs(prefs);
  try {
    storage.setItem(BACKGROUND_KEYS.mode, safe.mode);
    storage.setItem(BACKGROUND_KEYS.paper, safe.paper);
    storage.setItem(BACKGROUND_KEYS.fit, safe.fit);
    storage.setItem(BACKGROUND_KEYS.imageAlpha, String(safe.imageAlpha));
    storage.setItem(BACKGROUND_KEYS.panelAlpha, String(safe.panelAlpha));
    return true;
  } catch {
    return false;
  }
}

/** 给 `.app-bg` 的 class 列表；纸纹只在 paper 模式下加。 */
export function backgroundClasses(prefs: BackgroundPrefs): string[] {
  const safe = normalizeBackgroundPrefs(prefs);
  return [
    "app-bg",
    `app-bg--${safe.mode}`,
    ...(safe.mode === "paper" ? [`app-bg--${safe.paper}`] : []),
  ];
}

/** 图片层的铺法；tile 用原始尺寸平铺，其余用 cover/contain。 */
export function imageFitStyle(fit: BackgroundFit): {
  backgroundSize: string;
  backgroundRepeat: string;
} {
  return {
    backgroundSize: fit === "tile" ? "auto" : fit,
    backgroundRepeat: fit === "tile" ? "repeat" : "no-repeat",
  };
}

export function applyBackgroundPrefs(
  root: BackgroundRoot,
  prefs: BackgroundPrefs,
): BackgroundPrefs {
  const safe = normalizeBackgroundPrefs(prefs);
  root.dataset.bgMode = safe.mode;
  root.dataset.bgPaper = safe.paper;
  root.dataset.bgFit = safe.fit;
  root.style.setProperty("--panel-alpha", String(safe.panelAlpha / 100));
  root.style.setProperty("--bg-image-alpha", String(safe.imageAlpha / 100));
  return safe;
}
