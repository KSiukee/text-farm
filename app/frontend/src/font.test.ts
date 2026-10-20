import { describe, expect, it } from "vitest";
import {
  applyFontPrefs,
  CUSTOM_FONT_FAMILY,
  DEFAULT_FONT_PREFS,
  DEFAULT_FONT_PRESET,
  FONT_FALLBACK,
  FONT_KEYS,
  FONT_PRESETS,
  fontFamilyFor,
  isFontChoiceId,
  isFontPresetId,
  normalizeFontPrefs,
  readFontPrefs,
  sanitizeFontName,
  writeFontPrefs,
  type FontPrefs,
  type FontRoot,
  type FontStorage,
} from "./font";

function memoryStorage(initial: Record<string, string> = {}): FontStorage & {
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

function fakeRoot(): FontRoot & { vars: Map<string, string> } {
  const vars = new Map<string, string>();
  return {
    vars,
    dataset: {},
    style: {
      setProperty: (name, value) => {
        vars.set(name, value);
      },
    },
  };
}

describe("font presets", () => {
  it("ships six distinct system presets", () => {
    expect(FONT_PRESETS).toHaveLength(6);
    expect(new Set(FONT_PRESETS.map((preset) => preset.id)).size).toBe(6);
    expect(FONT_PRESETS.map((preset) => preset.id)).toContain(DEFAULT_FONT_PRESET);
    for (const preset of FONT_PRESETS) {
      expect(preset.name.length).toBeGreaterThan(0);
      expect(preset.family.length).toBeGreaterThan(0);
    }
  });

  it("maps presets to font-family values", () => {
    expect(fontFamilyFor("song")).toBe(FONT_FALLBACK);
    expect(fontFamilyFor("hei")).toContain("Microsoft YaHei");
    expect(fontFamilyFor("kai")).toContain("KaiTi");
    expect(fontFamilyFor("fang")).toContain("FangSong");
    expect(fontFamilyFor("mono")).toContain("Consolas");
    expect(fontFamilyFor("system")).toContain("system-ui");
  });

  it("handles custom families with and without a loaded face", () => {
    expect(fontFamilyFor("custom", CUSTOM_FONT_FAMILY)).toBe(
      `"${CUSTOM_FONT_FAMILY}", ${FONT_FALLBACK}`,
    );
    expect(fontFamilyFor("custom")).toBe(FONT_FALLBACK);
    expect(fontFamilyFor("nope" as never)).toBe(FONT_FALLBACK);
  });

  it("guards the union values", () => {
    expect(isFontPresetId("kai")).toBe(true);
    expect(isFontPresetId("custom")).toBe(false);
    expect(isFontChoiceId("custom")).toBe(true);
    expect(isFontChoiceId("nope")).toBe(false);
  });
});

describe("font prefs", () => {
  it("round-trips a custom font choice", () => {
    const storage = memoryStorage();
    const prefs: FontPrefs = { id: "custom", name: "霞鹜文楷" };
    expect(writeFontPrefs(storage, prefs)).toBe(true);
    expect(storage.data[FONT_KEYS.id]).toBe("custom");
    expect(storage.data[FONT_KEYS.name]).toBe("霞鹜文楷");
    expect(readFontPrefs(storage)).toEqual(prefs);
  });

  it("keeps the stored custom name while a preset is active", () => {
    const storage = memoryStorage({
      [FONT_KEYS.id]: "song",
      [FONT_KEYS.name]: "consola",
    });
    expect(readFontPrefs(storage)).toEqual({ id: "song", name: "consola" });
  });

  it("falls back for bad or incomplete values", () => {
    expect(readFontPrefs(memoryStorage())).toEqual(DEFAULT_FONT_PREFS);
    expect(
      readFontPrefs(memoryStorage({ [FONT_KEYS.id]: "nope", [FONT_KEYS.name]: "X" })),
    ).toEqual(DEFAULT_FONT_PREFS);
    // custom 但没名字 = 没有可用的字体文件，回默认。
    expect(
      readFontPrefs(memoryStorage({ [FONT_KEYS.id]: "custom", [FONT_KEYS.name]: "" })),
    ).toEqual(DEFAULT_FONT_PREFS);
  });

  it("survives storage failures", () => {
    const broken: FontStorage = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("quota");
      },
    };
    expect(readFontPrefs(broken)).toEqual(DEFAULT_FONT_PREFS);
    expect(writeFontPrefs(broken, DEFAULT_FONT_PREFS)).toBe(false);
    expect(readFontPrefs(null)).toEqual(DEFAULT_FONT_PREFS);
    expect(writeFontPrefs(null, DEFAULT_FONT_PREFS)).toBe(false);
  });

  it("sanitizes the custom display name", () => {
    expect(sanitizeFontName("  霞鹜   文楷  ")).toBe("霞鹜 文楷");
    expect(sanitizeFontName("x".repeat(80))).toHaveLength(40);
    expect(sanitizeFontName(null)).toBe("");
    expect(normalizeFontPrefs({ id: "custom", name: "  " })).toEqual(DEFAULT_FONT_PREFS);
  });
});

describe("font application", () => {
  it("writes data-font and --font-ui for a preset", () => {
    const root = fakeRoot();
    const applied = applyFontPrefs(root, { id: "kai", name: "" });
    expect(applied.id).toBe("kai");
    expect(root.dataset.font).toBe("kai");
    expect(root.vars.get("--font-ui")).toContain("KaiTi");
  });

  it("uses the custom family when one is loaded", () => {
    const root = fakeRoot();
    applyFontPrefs(root, { id: "custom", name: "我的字体" }, CUSTOM_FONT_FAMILY);
    expect(root.dataset.font).toBe("custom");
    expect(root.vars.get("--font-ui")).toBe(
      `"${CUSTOM_FONT_FAMILY}", ${FONT_FALLBACK}`,
    );
  });

  it("falls back when custom is selected but not loaded", () => {
    const root = fakeRoot();
    applyFontPrefs(root, { id: "custom", name: "我的字体" });
    expect(root.vars.get("--font-ui")).toBe(FONT_FALLBACK);
  });
});
