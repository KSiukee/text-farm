/**
 * 主题守卫（#75 起）：
 * 1) 组件样式里不许出现裸色（#hex / rgb() / rgba()）；
 * 2) 暗色必须逐名覆盖浅色 token（accent 六件由 theme.ts 内联，单独放行）；
 * 3) 用到的 var(--token) 必须有定义，别写出幽灵变量。
 *
 * 用法：node scripts/theme-guard.mjs
 */
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const themePath = join(root, "app", "frontend", "src", "theme.css");
const themeCss = readFileSync(themePath, "utf8");
const themeClean = themeCss.replace(/\/\*[\s\S]*?\*\//g, "");

const failures = [];

function fail(message) {
  failures.push(message);
}

function blockAfter(source, pattern, label) {
  const match = source.match(pattern);
  if (!match) {
    fail(`找不到${label} token 块`);
    return "";
  }
  return match[1];
}

const lightBlock = blockAfter(themeClean, /:root\s*\{([\s\S]*?)\}/, "浅色");
const darkBlock = blockAfter(
  themeClean,
  /:root\[data-theme="dark"\]\s*\{([\s\S]*?)\}/,
  "暗色",
);

function tokenNames(block) {
  return new Set([...block.matchAll(/(--[a-z0-9-]+)\s*:/g)].map((match) => match[1]));
}

const lightTokens = tokenNames(lightBlock);
const darkTokens = tokenNames(darkBlock);

// accent 六件由 theme.ts 内联写到 <html>，暗色不重定义；--on-accent 明暗共用。
const INLINE_TOKENS = new Set([
  "--accent",
  "--accent-hover",
  "--accent-soft",
  "--accent-line",
  "--accent-ink",
  "--accent-ink-dark",
  "--on-accent",
]);

for (const token of lightTokens) {
  if (!darkTokens.has(token) && !INLINE_TOKENS.has(token)) {
    fail(`暗色缺少 token：${token}`);
  }
}
for (const token of darkTokens) {
  if (!lightTokens.has(token)) fail(`暗色多出未定义的 token：${token}`);
}

const componentFiles = [
  "app/frontend/src/style.css",
  "app/frontend/src/App.vue",
  "app/frontend/src/FishingGame.vue",
];

const rawColor = /#[0-9a-fA-F]{3,8}\b|\brgba?\(/g;
for (const relative of componentFiles) {
  const source = readFileSync(join(root, relative), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  for (const match of source.matchAll(rawColor)) {
    fail(`${relative} 出现裸色：${match[0]}（只允许写在 theme.css 的 token 块里）`);
  }
}

const themeOutsideBlocks = themeClean.replace(lightBlock, "").replace(darkBlock, "");
for (const match of themeOutsideBlocks.matchAll(rawColor)) {
  fail(`theme.css 的 token 块外出现裸色：${match[0]}`);
}

const used = new Map();
for (const relative of [...componentFiles, "app/frontend/src/theme.css"]) {
  const source = readFileSync(join(root, relative), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  for (const match of source.matchAll(/var\((--[a-z0-9-]+)\)/g)) {
    if (!used.has(match[1])) used.set(match[1], relative);
  }
}
for (const [token, relative] of used) {
  if (!lightTokens.has(token)) fail(`${relative} 用到未定义的变量：${token}`);
}

const unused = [...lightTokens].filter((token) => !used.has(token));
if (unused.length) {
  console.warn(`[theme-guard] 提示：${unused.length} 个 token 暂时没用上：${unused.join(", ")}`);
}

if (failures.length) {
  console.error(`[theme-guard] 失败 ${failures.length} 项`);
  for (const message of failures) console.error(` - ${message}`);
  process.exitCode = 1;
} else {
  console.log(
    `[theme-guard] 通过：浅色 ${lightTokens.size} 个 token，暗色 ${darkTokens.size} 个，组件裸色 0，幽灵变量 0`,
  );
}
