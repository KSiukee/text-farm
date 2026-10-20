import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { PALETTES } from "./theme";

/**
 * 对比度抽查（#77）：直接读 theme.css 的 token 块，避免测试里再抄一份颜色。
 * 阈值：正文/次要/语义色 ≥ 4.5；弱提示 ≥ 2.2；禁用 ≥ 1.9（禁用本来就该淡）。
 */

const css = readFileSync(new URL("./theme.css", import.meta.url), "utf8");

function block(pattern: RegExp): string {
  const match = css.match(pattern);
  if (!match) throw new Error(`theme.css 缺少 token 块：${pattern}`);
  return match[1];
}

const light = block(/:root\s*\{([\s\S]*?)\}/);
const dark = block(/:root\[data-theme="dark"\]\s*\{([\s\S]*?)\}/);

function token(source: string, name: string): string {
  const match = source.match(new RegExp(`${name}\\s*:\\s*([^;]+);`));
  if (!match) throw new Error(`缺少 token ${name}`);
  return match[1].trim();
}

function channel(value: number): number {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(color: string): number {
  const raw = color.replace(/^#/, "");
  const full =
    raw.length === 3
      ? raw
          .split("")
          .map((ch) => ch + ch)
          .join("")
      : raw;
  if (!/^[0-9a-fA-F]{6}$/.test(full)) throw new Error(`不是十六进制颜色：${color}`);
  const value = Number.parseInt(full, 16);
  const r = (value >> 16) & 0xff;
  const g = (value >> 8) & 0xff;
  const b = value & 0xff;
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

function pairs(source: string): {
  paper: string;
  ink: string;
  ink2: string;
  muted: string;
  faint: string;
  disabled: string;
  dangerStrong: string;
  dangerBg: string;
  success: string;
  warn: string;
  warnBg: string;
  goldStrong: string;
  goldBg: string;
  info: string;
  infoBg: string;
} {
  return {
    paper: token(source, "--paper"),
    ink: token(source, "--ink"),
    ink2: token(source, "--ink-2"),
    muted: token(source, "--muted"),
    faint: token(source, "--faint"),
    disabled: token(source, "--disabled"),
    dangerStrong: token(source, "--danger-strong"),
    dangerBg: token(source, "--danger-bg"),
    success: token(source, "--success"),
    warn: token(source, "--warn"),
    warnBg: token(source, "--warn-bg"),
    goldStrong: token(source, "--gold-strong"),
    goldBg: token(source, "--gold-bg"),
    info: token(source, "--info"),
    infoBg: token(source, "--info-bg"),
  };
}

describe.each([
  ["light", light],
  ["dark", dark],
])("token contrast (%s)", (_name, source) => {
  const t = pairs(source);

  it("keeps body and secondary text readable", () => {
    expect(contrast(t.ink, t.paper)).toBeGreaterThanOrEqual(7);
    expect(contrast(t.ink2, t.paper)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t.muted, t.paper)).toBeGreaterThanOrEqual(4.5);
  });

  it("records weak/disabled text as intentionally lower", () => {
    expect(contrast(t.faint, t.paper)).toBeGreaterThanOrEqual(2.2);
    expect(contrast(t.disabled, t.paper)).toBeGreaterThanOrEqual(1.9);
  });

  it("keeps semantic colors readable", () => {
    expect(contrast(t.dangerStrong, t.dangerBg)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t.success, t.paper)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t.warn, t.warnBg)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t.goldStrong, t.goldBg)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t.info, t.infoBg)).toBeGreaterThanOrEqual(4.5);
  });
});

describe("palette link/ink contrast", () => {
  const lightPaper = token(light, "--paper");
  const darkPaper = token(dark, "--paper");

  it("keeps all five palettes readable on both papers", () => {
    for (const palette of PALETTES) {
      expect(contrast(palette.ink, lightPaper), `${palette.id} light`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(palette.inkDark, darkPaper), `${palette.id} dark`).toBeGreaterThanOrEqual(4.5);
      expect(contrast("#2b3325", palette.accent), `${palette.id} on-accent`).toBeGreaterThanOrEqual(
        4.5,
      );
    }
  });
});
