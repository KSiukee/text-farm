import { describe, expect, it } from "vitest";
import {
  applyStylePack,
  DEFAULT_STYLE_PACK_ID,
  isStylePackId,
  readStylePack,
  STYLE_PACKS,
  STYLE_PACK_STORAGE_KEY,
  stylePackById,
  writeStylePack,
  type StylePack,
  type StylePackRoot,
  type StylePackStorage,
} from "./style-pack";

function memoryStorage(initial: Record<string, string> = {}): StylePackStorage & {
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

function fakeRoot(): StylePackRoot & { vars: Map<string, string> } {
  const vars = new Map<string, string>();
  return {
    vars,
    dataset: {},
    style: {
      setProperty: (name, value) => {
        vars.set(name, value);
      },
      removeProperty: (name) => {
        vars.delete(name);
      },
    },
  };
}

describe("style packs", () => {
  it("ships harbor and classic", () => {
    expect(STYLE_PACKS.map((pack) => pack.id)).toEqual(["harbor", "classic"]);
    expect(new Set(STYLE_PACKS.map((pack) => pack.id)).size).toBe(2);
    for (const pack of STYLE_PACKS) {
      expect(pack.version).toBeGreaterThan(0);
      expect(pack.name.length).toBeGreaterThan(0);
    }
    expect(stylePackById("classic").layout).toBe("classic");
    expect(stylePackById("classic").decoration).toBe("flat");
    expect(stylePackById("classic").density).toBe("compact");
    expect(stylePackById("harbor").layout).toBe("harbor");
  });

  it("falls back to harbor for unknown ids", () => {
    expect(stylePackById("nope").id).toBe(DEFAULT_STYLE_PACK_ID);
    expect(stylePackById(null).id).toBe(DEFAULT_STYLE_PACK_ID);
    expect(isStylePackId("classic")).toBe(true);
    expect(isStylePackId("nope")).toBe(false);
  });

  it("round-trips through storage", () => {
    const storage = memoryStorage();
    expect(writeStylePack(storage, stylePackById("classic"))).toBe(true);
    expect(storage.data[STYLE_PACK_STORAGE_KEY]).toBe("classic");
    expect(readStylePack(storage).id).toBe("classic");
    expect(readStylePack(memoryStorage()).id).toBe(DEFAULT_STYLE_PACK_ID);
    expect(
      readStylePack(memoryStorage({ [STYLE_PACK_STORAGE_KEY]: "nope" })).id,
    ).toBe(DEFAULT_STYLE_PACK_ID);
  });

  it("survives storage failures", () => {
    const broken: StylePackStorage = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("quota");
      },
    };
    expect(readStylePack(broken).id).toBe(DEFAULT_STYLE_PACK_ID);
    expect(writeStylePack(broken, stylePackById("classic"))).toBe(false);
    expect(readStylePack(null).id).toBe(DEFAULT_STYLE_PACK_ID);
    expect(writeStylePack(null, stylePackById("classic"))).toBe(false);
  });

  it("writes data attributes for the chosen layout", () => {
    const root = fakeRoot();
    const applied = applyStylePack(root, stylePackById("classic"));
    expect(applied.id).toBe("classic");
    expect(root.dataset.style).toBe("classic");
    expect(root.dataset.layout).toBe("classic");
    expect(root.dataset.density).toBe("compact");
    expect(root.dataset.decoration).toBe("flat");
  });

  it("normalizes invalid layout fields", () => {
    const root = fakeRoot();
    const applied = applyStylePack(root, {
      ...stylePackById("classic"),
      id: "ghost",
      layout: "nope" as unknown as StylePack["layout"],
    });
    expect(applied.id).toBe("ghost");
    expect(applied.layout).toBe("harbor");
    expect(root.dataset.layout).toBe("harbor");
  });

  it("applies and then clears pack tokens", () => {
    const root = fakeRoot();
    const custom: StylePack = {
      ...stylePackById("classic"),
      id: "classic",
      tokens: { "--link": "#123456", "--panel": "#ffffff" },
    };
    applyStylePack(root, custom);
    expect(root.vars.get("--link")).toBe("#123456");
    expect(root.vars.get("--panel")).toBe("#ffffff");
    applyStylePack(root, stylePackById("harbor"));
    expect(root.vars.size).toBe(0);
  });

  it("ignores non-custom-property tokens and never injects css", () => {
    const root = fakeRoot();
    const custom: StylePack = {
      ...stylePackById("harbor"),
      tokens: { color: "red", "--ok": "1px" },
      css: "body { display: none; }",
    };
    expect(() => applyStylePack(root, custom)).not.toThrow();
    expect(root.vars.has("color")).toBe(false);
    expect(root.vars.get("--ok")).toBe("1px");
  });
});
