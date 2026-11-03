/**
 * 主题系统：色板、明暗/跟随系统、偏好持久化、应用到 <html>。
 *
 * 约定：
 * - 这里是 accent（主色）的唯一来源；底色/文字/语义色在 `style.css` 的 token 块里。
 * - 纯函数优先，DOM 只通过 `ThemeRoot` 这个最小接口碰，方便单测。
 * - 偏好只进 localStorage（跟机器走），不写存档、不改 Rust。
 */

export const THEME_MODES = [
  { id: "light", name: "浅色" },
  { id: "dark", name: "深色" },
  { id: "system", name: "跟随系统" },
] as const;

export type ThemeMode = (typeof THEME_MODES)[number]["id"];
export type ResolvedTheme = "light" | "dark";

export const PALETTES = [
  {
    id: "sage",
    name: "鼠尾草绿",
    accent: "#9caf88",
    hover: "#b7c9a4",
    ink: "#5f7050",
    inkDark: "#d6e2c6",
  },
  {
    id: "mint",
    name: "薄荷绿",
    accent: "#a8d8c9",
    hover: "#c2e6db",
    ink: "#3f6e60",
    inkDark: "#cde9e0",
  },
  {
    id: "matcha",
    name: "抹茶绿",
    accent: "#a8b28c",
    hover: "#c0c9a4",
    ink: "#5a6349",
    inkDark: "#d3dac2",
  },
  {
    id: "mist",
    name: "雾灰绿",
    accent: "#a5b8a6",
    hover: "#c0d0c0",
    ink: "#55665a",
    inkDark: "#d0dcd2",
  },
  {
    id: "bean",
    name: "豆绿",
    accent: "#b8c4a5",
    hover: "#d0d9bc",
    ink: "#616b4f",
    inkDark: "#dbe2cb",
  },
] as const;

export type PaletteId = (typeof PALETTES)[number]["id"];
export type Palette = (typeof PALETTES)[number];

export const DEFAULT_MODE: ThemeMode = "system";
export const DEFAULT_PALETTE: PaletteId = "sage";

export const STORAGE_KEYS = {
  mode: "farm.ui.themeMode",
  palette: "farm.ui.palette",
} as const;

export interface ThemePrefs {
  mode: ThemeMode;
  palette: PaletteId;
}

export const DEFAULT_PREFS: ThemePrefs = {
  mode: DEFAULT_MODE,
  palette: DEFAULT_PALETTE,
};

/** localStorage 的最小接口；单测用内存实现即可。 */
export interface PrefsStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** `<html>` 的最小接口；避免单测依赖 jsdom。 */
export interface ThemeRoot {
  dataset: { [key: string]: string | undefined };
  style: { setProperty(name: string, value: string): void };
}

export function isThemeMode(value: unknown): value is ThemeMode {
  return THEME_MODES.some((mode) => mode.id === value);
}

export function isPaletteId(value: unknown): value is PaletteId {
  return PALETTES.some((palette) => palette.id === value);
}

export function paletteById(id: string | null | undefined): Palette {
  return PALETTES.find((palette) => palette.id === id) ?? PALETTES[0];
}

/** 把「浅色/深色/跟随系统」+ 系统当前状态折成真正要画的那一套。 */
export function resolveTheme(mode: ThemeMode, systemDark: boolean): ResolvedTheme {
  if (mode === "dark") return "dark";
  if (mode === "light") return "light";
  return systemDark ? "dark" : "light";
}

/** #rgb / #rrggbb → rgba(...)；只认色板里那几种写法，非法值退回透明。 */
export function withAlpha(hex: string, alpha: number): string {
  const raw = hex.trim().replace(/^#/, "");
  const full =
    raw.length === 3
      ? raw
          .split("")
          .map((ch) => ch + ch)
          .join("")
      : raw;
  if (!/^[0-9a-fA-F]{6}$/.test(full)) return `rgba(0, 0, 0, ${alpha})`;
  const value = Number.parseInt(full, 16);
  const r = (value >> 16) & 0xff;
  const g = (value >> 8) & 0xff;
  const b = value & 0xff;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** 色板 → 内联 CSS 变量（写在 <html> 上，避开 CSS 里再写一套）。 */
export function accentVars(palette: Palette): Record<string, string> {
  return {
    "--accent": palette.accent,
    "--accent-hover": palette.hover,
    "--accent-soft": withAlpha(palette.accent, 0.16),
    "--accent-line": withAlpha(palette.accent, 0.34),
    "--accent-ink": palette.ink,
    "--accent-ink-dark": palette.inkDark,
  };
}

export function readThemePrefs(storage: PrefsStorage | null | undefined): ThemePrefs {
  if (!storage) return { ...DEFAULT_PREFS };
  let mode: string | null = null;
  let palette: string | null = null;
  try {
    mode = storage.getItem(STORAGE_KEYS.mode);
    palette = storage.getItem(STORAGE_KEYS.palette);
  } catch {
    return { ...DEFAULT_PREFS };
  }
  return {
    mode: isThemeMode(mode) ? mode : DEFAULT_PREFS.mode,
    palette: isPaletteId(palette) ? palette : DEFAULT_PREFS.palette,
  };
}

/** 存储写失败（隐私模式/配额）不该打断界面；返回是否写成功。 */
export function writeThemePrefs(
  storage: PrefsStorage | null | undefined,
  prefs: ThemePrefs,
): boolean {
  if (!storage) return false;
  try {
    storage.setItem(STORAGE_KEYS.mode, prefs.mode);
    storage.setItem(STORAGE_KEYS.palette, prefs.palette);
    return true;
  } catch {
    return false;
  }
}

/**
 * 把主题落到 <html>：data-theme / data-palette + accent 内联变量。
 * 返回真正生效的明暗，方便壳层同步窗口底色。
 */
export function applyTheme(
  root: ThemeRoot,
  prefs: ThemePrefs,
  systemDark: boolean,
): ResolvedTheme {
  const resolved = resolveTheme(prefs.mode, systemDark);
  const palette = paletteById(prefs.palette);
  root.dataset.theme = resolved;
  root.dataset.palette = palette.id;
  for (const [name, value] of Object.entries(accentVars(palette))) {
    root.style.setProperty(name, value);
  }
  return resolved;
}
