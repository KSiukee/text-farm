/**
 * 样式包（#78）：把「布局 / 密度 / 装饰」从主题色里分出来。
 *
 * v1 只有两个内置包：
 * - harbor：当前 hero + 卡片 + 折叠（默认）
 * - classic：钓鱼页平铺、无卡片、紧凑
 *
 * 约定：
 * - 颜色不在这里定义；配色永远走 theme.css 的 token（#75 的单出口）。
 * - `tokens` / `css` 是给以后用户样式包留的接口；v1 内置包不填、也不执行 css。
 * - 落 <html>：data-style（包 id）+ data-layout / data-density / data-decoration。
 */

export type LayoutId = "classic" | "harbor";
export type Density = "compact" | "comfortable";
export type Decoration = "flat" | "card";

export interface StylePack {
  id: string;
  name: string;
  version: number;
  layout: LayoutId;
  density: Density;
  decoration: Decoration;
  /** 可选覆盖（以后用户包用）；内置包留空 */
  tokens?: Record<string, string>;
  /** v1 不注入任意 CSS，只留字段 */
  css?: string;
}

export const STYLE_PACKS: readonly StylePack[] = [
  {
    id: "harbor",
    name: "新版",
    version: 1,
    layout: "harbor",
    density: "comfortable",
    decoration: "card",
  },
  {
    id: "classic",
    name: "经典",
    version: 1,
    layout: "classic",
    density: "compact",
    decoration: "flat",
  },
];

export const DEFAULT_STYLE_PACK_ID = "harbor";
export const STYLE_PACK_STORAGE_KEY = "farm.ui.stylePack";

export interface StylePackStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** `<html>` 的最小接口，单测不依赖 DOM。 */
export interface StylePackRoot {
  dataset: { [key: string]: string | undefined };
  style: {
    setProperty(name: string, value: string): void;
    removeProperty(name: string): void;
  };
}

export function stylePackById(id: string | null | undefined): StylePack {
  return (
    STYLE_PACKS.find((pack) => pack.id === id) ??
    STYLE_PACKS.find((pack) => pack.id === DEFAULT_STYLE_PACK_ID) ??
    STYLE_PACKS[0]
  );
}

export function isStylePackId(value: unknown): value is string {
  return typeof value === "string" && STYLE_PACKS.some((pack) => pack.id === value);
}

export function readStylePack(storage: StylePackStorage | null | undefined): StylePack {
  if (!storage) return stylePackById(DEFAULT_STYLE_PACK_ID);
  try {
    return stylePackById(storage.getItem(STYLE_PACK_STORAGE_KEY));
  } catch {
    return stylePackById(DEFAULT_STYLE_PACK_ID);
  }
}

export function writeStylePack(
  storage: StylePackStorage | null | undefined,
  pack: StylePack,
): boolean {
  if (!storage) return false;
  try {
    storage.setItem(STYLE_PACK_STORAGE_KEY, pack.id);
    return true;
  } catch {
    return false;
  }
}

// 每个根元素上一次应用过哪些 token，换包时先撤掉，避免残留。
const appliedTokens = new WeakMap<object, string[]>();

/** 把外部传进来的包收口成合法值；未知/坏的布局字段回 harbor。 */
function normalizeStylePack(pack: StylePack): StylePack {
  if (!pack || typeof pack.id !== "string" || pack.id.trim() === "") {
    return stylePackById(DEFAULT_STYLE_PACK_ID);
  }
  return {
    ...pack,
    layout: pack.layout === "classic" ? "classic" : "harbor",
    density: pack.density === "compact" ? "compact" : "comfortable",
    decoration: pack.decoration === "flat" ? "flat" : "card",
    version: Number.isFinite(pack.version) && pack.version > 0 ? pack.version : 1,
  };
}

/**
 * 把样式包落到 <html>。返回实际生效的包。
 * `tokens` 只接受 `--` 开头的自定义属性；`css` 在 v1 不执行。
 */
export function applyStylePack(root: StylePackRoot, pack: StylePack): StylePack {
  const resolved = normalizeStylePack(pack);
  root.dataset.style = resolved.id;
  root.dataset.layout = resolved.layout;
  root.dataset.density = resolved.density;
  root.dataset.decoration = resolved.decoration;

  const previous = appliedTokens.get(root) ?? [];
  for (const name of previous) root.style.removeProperty(name);

  const names: string[] = [];
  for (const [name, value] of Object.entries(resolved.tokens ?? {})) {
    if (!name.startsWith("--") || typeof value !== "string") continue;
    root.style.setProperty(name, value);
    names.push(name);
  }
  appliedTokens.set(root, names);
  return resolved;
}
