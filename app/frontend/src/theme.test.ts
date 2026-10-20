import { describe, expect, it } from "vitest";
import {
  applyTheme,
  accentVars,
  DEFAULT_PREFS,
  PALETTES,
  paletteById,
  readThemePrefs,
  resolveTheme,
  STORAGE_KEYS,
  withAlpha,
  writeThemePrefs,
  type PrefsStorage,
  type ThemePrefs,
  type ThemeRoot,
} from "./theme";

function memoryStorage(initial: Record<string, string> = {}): PrefsStorage & {
  data: Record<string, string>;
} {
  const data = { ...initial };
  return {
    data,
    getItem: (key) => (key in data ? data[key] : null),
    setItem: (key, value) => {
      data[key] = value;
    },
  };
}

function fakeRoot(): ThemeRoot & { vars: Record<string, string> } {
  const vars: Record<string, string> = {};
  return {
    vars,
    dataset: {},
    style: {
      setProperty: (name, value) => {
        vars[name] = value;
      },
    },
  };
}

function channel(value: number): number {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(hex: string): number {
  const raw = hex.replace(/^#/, "");
  const value = Number.parseInt(raw, 16);
  const r = (value >> 16) & 0xff;
  const g = (value >> 8) & 0xff;
  const b = value & 0xff;
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

// 与 style.css token 块的 --paper / 暗色 --paper 保持一致。
const LIGHT_PAPER = "#f6f3ec";
const DARK_PAPER = "#20251d";

describe("theme palettes", () => {
  it("has five complete, unique palettes", () => {
    expect(PALETTES).toHaveLength(5);
    expect(new Set(PALETTES.map((p) => p.id)).size).toBe(5);
    for (const palette of PALETTES) {
      for (const key of ["accent", "hover", "ink", "inkDark"] as const) {
        expect(palette[key], `${palette.id}.${key}`).toMatch(/^#[0-9a-f]{6}$/);
      }
    }
  });

  it("keeps text readable on both papers", () => {
    for (const palette of PALETTES) {
      expect(contrast(palette.ink, LIGHT_PAPER), `${palette.id}.ink`).toBeGreaterThanOrEqual(4.5);
      expect(
        contrast(palette.inkDark, DARK_PAPER),
        `${palette.id}.inkDark`,
      ).toBeGreaterThanOrEqual(4.5);
      // 选中态按钮：accent 底 + --on-accent（#2b3325）深字。
      expect(contrast("#2b3325", palette.accent), `${palette.id}.onAccent`).toBeGreaterThanOrEqual(
        4.5,
      );
    }
  });

  it("falls back to sage for unknown palette ids", () => {
    expect(paletteById("nope").id).toBe("sage");
    expect(paletteById(null).id).toBe("sage");
  });
});

describe("theme resolution", () => {
  it("resolves light/dark/system", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
});

describe("theme prefs", () => {
  it("round-trips through storage", () => {
    const storage = memoryStorage();
    const prefs: ThemePrefs = { mode: "dark", palette: "mint" };
    expect(writeThemePrefs(storage, prefs)).toBe(true);
    expect(storage.data[STORAGE_KEYS.mode]).toBe("dark");
    expect(storage.data[STORAGE_KEYS.palette]).toBe("mint");
    expect(readThemePrefs(storage)).toEqual(prefs);
  });

  it("falls back to defaults for bad or missing values", () => {
    expect(readThemePrefs(memoryStorage())).toEqual(DEFAULT_PREFS);
    expect(
      readThemePrefs(memoryStorage({ [STORAGE_KEYS.mode]: "neon", [STORAGE_KEYS.palette]: "neon" })),
    ).toEqual(DEFAULT_PREFS);
  });

  it("survives storage failures", () => {
    const broken: PrefsStorage = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("quota");
      },
    };
    expect(readThemePrefs(broken)).toEqual(DEFAULT_PREFS);
    expect(writeThemePrefs(broken, DEFAULT_PREFS)).toBe(false);
    expect(readThemePrefs(null)).toEqual(DEFAULT_PREFS);
    expect(writeThemePrefs(null, DEFAULT_PREFS)).toBe(false);
  });
});

describe("theme application", () => {
  it("sets data attributes and accent variables", () => {
    const root = fakeRoot();
    const resolved = applyTheme(root, { mode: "dark", palette: "bean" }, false);
    expect(resolved).toBe("dark");
    expect(root.dataset.theme).toBe("dark");
    expect(root.dataset.palette).toBe("bean");
    expect(root.vars["--accent"]).toBe("#b8c4a5");
    expect(root.vars["--accent-ink"]).toBe("#616b4f");
    expect(root.vars["--accent-ink-dark"]).toBe("#dbe2cb");
    expect(root.vars["--accent-soft"]).toMatch(/^rgba\(184, 196, 165, 0\.16\)$/);
  });

  it("follows the system when mode is system", () => {
    const root = fakeRoot();
    expect(applyTheme(root, { mode: "system", palette: "sage" }, true)).toBe("dark");
    expect(root.dataset.theme).toBe("dark");
    expect(applyTheme(root, { mode: "system", palette: "sage" }, false)).toBe("light");
    expect(root.dataset.theme).toBe("light");
  });

  it("only declares the six accent variables", () => {
    expect(Object.keys(accentVars(paletteById("sage"))).sort()).toEqual(
      [
        "--accent",
        "--accent-hover",
        "--accent-ink",
        "--accent-ink-dark",
        "--accent-line",
        "--accent-soft",
      ].sort(),
    );
  });

  it("converts hex to rgba", () => {
    expect(withAlpha("#9caf88", 0.16)).toBe("rgba(156, 175, 136, 0.16)");
    expect(withAlpha("#fff", 0.5)).toBe("rgba(255, 255, 255, 0.5)");
    expect(withAlpha("not-a-color", 0.2)).toBe("rgba(0, 0, 0, 0.2)");
  });
});
