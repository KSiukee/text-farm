import { describe, expect, it } from "vitest";
import {
  applyBackgroundPrefs,
  BACKGROUND_KEYS,
  backgroundClasses,
  clampPercent,
  DEFAULT_BACKGROUND_PREFS,
  imageFitStyle,
  isBackgroundFit,
  isBackgroundMode,
  isPaperPattern,
  normalizeBackgroundPrefs,
  readBackgroundPrefs,
  writeBackgroundPrefs,
  type BackgroundPrefs,
  type BackgroundRoot,
  type BackgroundStorage,
} from "./background";

function memoryStorage(initial: Record<string, string> = {}): BackgroundStorage & {
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

function fakeRoot(): BackgroundRoot & { vars: Map<string, string> } {
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

describe("background prefs", () => {
  it("has sane defaults", () => {
    expect(DEFAULT_BACKGROUND_PREFS).toEqual({
      mode: "solid",
      paper: "grain",
      fit: "cover",
      imageAlpha: 30,
      panelAlpha: 92,
    });
  });

  it("clamps the two sliders", () => {
    expect(clampPercent(59, 60, 100, 92)).toBe(60);
    expect(clampPercent(101, 60, 100, 92)).toBe(100);
    expect(clampPercent("80", 60, 100, 92)).toBe(80);
    expect(clampPercent("abc", 60, 100, 92)).toBe(92);
    expect(clampPercent(null, 0, 100, 30)).toBe(30);
    expect(clampPercent(undefined, 0, 100, 30)).toBe(30);
    expect(clampPercent(0, 0, 100, 30)).toBe(0);
  });

  it("falls back for bad modes/patterns/fits", () => {
    const prefs = normalizeBackgroundPrefs({
      mode: "neon",
      paper: "denim",
      fit: "stretch",
      imageAlpha: 999,
      panelAlpha: 10,
    });
    expect(prefs).toEqual({
      mode: "solid",
      paper: "grain",
      fit: "cover",
      imageAlpha: 100,
      panelAlpha: 60,
    });
  });

  it("round-trips through storage", () => {
    const storage = memoryStorage();
    const prefs: BackgroundPrefs = {
      mode: "paper",
      paper: "dots",
      fit: "tile",
      imageAlpha: 45,
      panelAlpha: 84,
    };
    expect(writeBackgroundPrefs(storage, prefs)).toBe(true);
    expect(storage.data[BACKGROUND_KEYS.mode]).toBe("paper");
    expect(storage.data[BACKGROUND_KEYS.imageAlpha]).toBe("45");
    expect(readBackgroundPrefs(storage)).toEqual(prefs);
  });

  it("falls back for missing or bad stored values", () => {
    expect(readBackgroundPrefs(memoryStorage())).toEqual(DEFAULT_BACKGROUND_PREFS);
    expect(
      readBackgroundPrefs(
        memoryStorage({
          [BACKGROUND_KEYS.mode]: "neon",
          [BACKGROUND_KEYS.paper]: "denim",
          [BACKGROUND_KEYS.fit]: "stretch",
          [BACKGROUND_KEYS.imageAlpha]: "abc",
          [BACKGROUND_KEYS.panelAlpha]: "5",
        }),
      ),
    ).toEqual({ ...DEFAULT_BACKGROUND_PREFS, panelAlpha: 60 });
  });

  it("survives storage failures", () => {
    const broken: BackgroundStorage = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("quota");
      },
    };
    expect(readBackgroundPrefs(broken)).toEqual(DEFAULT_BACKGROUND_PREFS);
    expect(writeBackgroundPrefs(broken, DEFAULT_BACKGROUND_PREFS)).toBe(false);
    expect(readBackgroundPrefs(null)).toEqual(DEFAULT_BACKGROUND_PREFS);
    expect(writeBackgroundPrefs(null, DEFAULT_BACKGROUND_PREFS)).toBe(false);
  });
});

describe("background application", () => {
  it("writes data attributes and the two alpha variables", () => {
    const root = fakeRoot();
    const applied = applyBackgroundPrefs(root, {
      mode: "paper",
      paper: "dots",
      fit: "tile",
      imageAlpha: 45,
      panelAlpha: 80,
    });
    expect(applied.panelAlpha).toBe(80);
    expect(root.dataset.bgMode).toBe("paper");
    expect(root.dataset.bgPaper).toBe("dots");
    expect(root.dataset.bgFit).toBe("tile");
    expect(root.vars.get("--panel-alpha")).toBe("0.8");
    expect(root.vars.get("--bg-image-alpha")).toBe("0.45");
  });

  it("normalizes bad values before applying", () => {
    const root = fakeRoot();
    applyBackgroundPrefs(root, {
      ...DEFAULT_BACKGROUND_PREFS,
      mode: "neon" as unknown as BackgroundPrefs["mode"],
      panelAlpha: 5,
    });
    expect(root.dataset.bgMode).toBe("solid");
    expect(root.vars.get("--panel-alpha")).toBe("0.6");
  });

  it("returns classes for the three modes", () => {
    expect(backgroundClasses({ ...DEFAULT_BACKGROUND_PREFS, mode: "solid" })).toEqual([
      "app-bg",
      "app-bg--solid",
    ]);
    expect(
      backgroundClasses({ ...DEFAULT_BACKGROUND_PREFS, mode: "paper", paper: "grid" }),
    ).toEqual(["app-bg", "app-bg--paper", "app-bg--grid"]);
    expect(
      backgroundClasses({ ...DEFAULT_BACKGROUND_PREFS, mode: "image", paper: "dots" }),
    ).toEqual(["app-bg", "app-bg--image"]);
  });

  it("maps fits to background size/repeat", () => {
    expect(imageFitStyle("cover")).toEqual({
      backgroundSize: "cover",
      backgroundRepeat: "no-repeat",
    });
    expect(imageFitStyle("contain")).toEqual({
      backgroundSize: "contain",
      backgroundRepeat: "no-repeat",
    });
    expect(imageFitStyle("tile")).toEqual({
      backgroundSize: "auto",
      backgroundRepeat: "repeat",
    });
  });

  it("guards the union values", () => {
    expect(isBackgroundMode("paper")).toBe(true);
    expect(isBackgroundMode("neon")).toBe(false);
    expect(isPaperPattern("grid")).toBe(true);
    expect(isPaperPattern("denim")).toBe(false);
    expect(isBackgroundFit("contain")).toBe(true);
    expect(isBackgroundFit("stretch")).toBe(false);
  });
});
