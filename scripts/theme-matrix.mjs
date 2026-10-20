/**
 * 主题矩阵（#77）：5 色板 × 明/暗 × 农场/玩家/设置/钓鱼 = 40 张截图。
 * 额外补：经典布局 2 张、纸纹背景 2 张；顺手生成一个可点的 HTML 总览。
 *
 * 用法：node scripts/theme-matrix.mjs
 * 产物：docs/theme-matrix/*.png + docs/theme-matrix.html + docs/theme-matrix-v054.md
 */
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = join(root, "文字农场.exe");
const shadowDir = join(root, "shadow", "theme-matrix");
const outDir = join(root, "docs", "theme-matrix");
const htmlPath = join(root, "docs", "theme-matrix.html");
const reportPath = join(root, "docs", "theme-matrix-v054.md");
const port = 9335;

const PALETTES = [
  { id: "sage", name: "鼠尾草绿" },
  { id: "mint", name: "薄荷绿" },
  { id: "matcha", name: "抹茶绿" },
  { id: "mist", name: "雾灰绿" },
  { id: "bean", name: "豆绿" },
];
const THEMES = [
  { id: "light", name: "浅色" },
  { id: "dark", name: "深色" },
];
const PAGES = [
  { id: "farm", name: "农场" },
  { id: "player", name: "玩家" },
  { id: "settings", name: "设置" },
  { id: "fishing", name: "钓鱼" },
];

const checks = [];
let child = null;
let ws = null;
let nextId = 0;
const pending = new Map();
let originalPrefs = null;

const PREF_KEYS = [
  "farm.ui.themeMode",
  "farm.ui.palette",
  "farm.ui.stylePack",
  "farm.ui.bgMode",
  "farm.ui.bgPaper",
  "farm.ui.bgFit",
  "farm.ui.bgImageAlpha",
  "farm.ui.bgPanelAlpha",
];

function check(name, ok, detail = "") {
  checks.push({ name, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
}

async function waitFor(fn, timeoutMs = 8000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const value = await fn();
      if (value) return value;
    } catch {}
    await new Promise((r) => setTimeout(r, 120));
  }
  throw new Error("等待超时");
}

function send(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    pending.set(id, (message) => {
      if (message.error) reject(new Error(JSON.stringify(message.error)));
      else resolve(message);
    });
    ws.send(JSON.stringify({ id, method, params }));
  });
}

async function evaluate(expression) {
  const message = await send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  if (message.result.exceptionDetails) {
    throw new Error(JSON.stringify(message.result.exceptionDetails));
  }
  return message.result.result.value;
}

async function bodyText() {
  return evaluate("document.body.innerText");
}

async function clickText(text, exact = true, nth = 0) {
  const found = await evaluate(`(() => {
    const wanted = ${JSON.stringify(text)};
    const exact = ${exact};
    const list = [...document.querySelectorAll('button, a, [role="button"]')].filter((el) => {
      const value = (el.innerText || el.textContent || '').trim();
      const visible = el.getClientRects().length > 0;
      return visible && (exact ? value === wanted : value.includes(wanted));
    });
    const el = list[${nth}];
    if (!el) return null;
    el.scrollIntoView({ block: 'center', inline: 'center' });
    const rect = el.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  if (!found) throw new Error(`找不到按钮「${text}」`);
  await send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    x: found.x,
    y: found.y,
    button: "left",
    clickCount: 1,
  });
  await send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    x: found.x,
    y: found.y,
    button: "left",
    clickCount: 1,
  });
  await new Promise((r) => setTimeout(r, 180));
}

async function screenshot(path) {
  const shot = await send("Page.captureScreenshot", { format: "png", fromSurface: true });
  writeFileSync(path, Buffer.from(shot.result.data, "base64"));
}

function spawnedAlreadyRunning() {
  try {
    const out = execFileSync("tasklist", ["/FI", "IMAGENAME eq 文字农场.exe"], {
      encoding: "utf8",
    });
    return out.includes("文字农场.exe");
  } catch {
    return false;
  }
}

async function startApp() {
  if (spawnedAlreadyRunning()) throw new Error("检测到已有文字农场进程，请先关掉再跑");
  rmSync(shadowDir, { recursive: true, force: true });
  mkdirSync(shadowDir, { recursive: true });
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });
  const launchedAt = Date.now();
  child = spawn(exe, [], {
    env: {
      ...process.env,
      TEXT_FARM_DATA_DIR: shadowDir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    cwd: root,
    stdio: "ignore",
  });
  await new Promise((r) => setTimeout(r, 1200));
  const targets = await waitFor(async () => {
    const response = await fetch(`http://127.0.0.1:${port}/json`);
    if (!response.ok) return null;
    const list = await response.json();
    return list.length ? list : null;
  });
  ws = new WebSocket(targets[0].webSocketDebuggerUrl);
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
    }
  };
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  await send("Page.enable");
  await send("Runtime.enable");
  await waitFor(async () => (await evaluate("document.readyState")) === "complete");
  await waitFor(async () => (await bodyText()).includes("我的农场"));
  // 窗口由 ui_ready 显示；等它真正可见，避免在隐藏窗口上点空。
  await waitFor(async () => (await evaluate("document.visibilityState")) === "visible");
  const bootMs = Date.now() - launchedAt;
  check("启动显示走 ui_ready（<2.5s 兜底）", bootMs < 2400, `${bootMs}ms`);
  await new Promise((r) => setTimeout(r, 300));
  // 记下用户原来的偏好，跑完恢复（矩阵会临时改主题/布局/背景）。
  originalPrefs = await evaluate(
    `Object.fromEntries(${JSON.stringify(PREF_KEYS)}.map((key) => [key, localStorage.getItem(key)]))`,
  );
}

async function restorePrefs() {
  if (!ws || !originalPrefs) return;
  try {
    await evaluate(`(() => {
      const data = ${JSON.stringify(originalPrefs)};
      for (const [key, value] of Object.entries(data)) {
        if (value === null) localStorage.removeItem(key);
        else localStorage.setItem(key, value);
      }
    })()`);
  } catch {}
}

async function goFarmTop() {
  await clickText("我的农场");
  await evaluate("window.scrollTo(0, 0)");
  await new Promise((r) => setTimeout(r, 120));
}

async function goPlayer() {
  await clickText("玩家");
  await waitFor(async () => (await bodyText()).includes("玩家信息"));
  await evaluate("window.scrollTo(0, 0)");
  await new Promise((r) => setTimeout(r, 120));
}

async function goSettings() {
  await clickText("我的农场");
  await evaluate(`(() => {
    const title = [...document.querySelectorAll('.panel-title')].find((el) =>
      (el.textContent || '').trim().startsWith('外观'),
    );
    const panel = title?.closest('.panel');
    panel?.scrollIntoView({ block: 'start' });
    window.scrollBy(0, -10);
    return !!panel;
  })()`);
  await new Promise((r) => setTimeout(r, 180));
}

async function goFishing() {
  await clickText("钓鱼", false);
  await waitFor(async () => (await bodyText()).includes("鱼类图鉴"));
  await evaluate(
    "document.querySelector('.fishing-page')?.scrollIntoView({ block: 'start' })",
  );
  await new Promise((r) => setTimeout(r, 180));
}

async function setTheme(theme) {
  await clickText(theme.name);
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.theme")) === theme.id,
  );
}

async function setPalette(palette) {
  await clickText(palette.name);
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.palette")) === palette.id,
  );
}

function reportRows() {
  return checks.map((c) => `- ${c.ok ? "PASS" : "FAIL"} ${c.name}${c.detail ? ` — ${c.detail}` : ""}`);
}

try {
  await startApp();
  const shots = [];

  // 先归零到已知状态（新版 / 跟随系统+sage / 纯色 92%），矩阵才好对照。
  await goFarmTop();
  await clickText("新版");
  await clickText("恢复默认");
  if (!(await evaluate("document.querySelector('.appearance-sub')?.open"))) {
    await evaluate("document.querySelector('.appearance-sub > summary')?.click()");
  }
  await clickText("恢复背景默认");

  for (const theme of THEMES) {
    for (const palette of PALETTES) {
      await goFarmTop();
      await setTheme(theme);
      await setPalette(palette);
      const state = await evaluate(`({
        theme: document.documentElement.dataset.theme,
        palette: document.documentElement.dataset.palette
      })`);
      check(
        `矩阵 ${theme.id}/${palette.id}`,
        state.theme === theme.id && state.palette === palette.id,
        JSON.stringify(state),
      );

      for (const page of PAGES) {
        if (page.id === "farm") await goFarmTop();
        if (page.id === "player") await goPlayer();
        if (page.id === "settings") await goSettings();
        if (page.id === "fishing") await goFishing();
        const file = `${page.id}-${palette.id}-${theme.id}.png`;
        const path = join(outDir, file);
        await screenshot(path);
        check(`截图 ${file}`, existsSync(path), `${page.name}`);
        shots.push({ page: page.id, palette: palette.id, theme: theme.id, file });
        if (page.id === "fishing") await goFarmTop();
      }
    }
  }

  // 额外：经典布局（浅/暗）
  await goFarmTop();
  await clickText("经典");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.layout")) === "classic",
  );
  await setTheme(THEMES[0]);
  await goFishing();
  const classicLight = "layout-classic-light.png";
  await screenshot(join(outDir, classicLight));
  check(`截图 ${classicLight}`, existsSync(join(outDir, classicLight)));
  await goFarmTop();
  await setTheme(THEMES[1]);
  await goFishing();
  const classicDark = "layout-classic-dark.png";
  await screenshot(join(outDir, classicDark));
  check(`截图 ${classicDark}`, existsSync(join(outDir, classicDark)));

  // 额外：纸纹背景（浅/暗），面板默认 92%
  await goFarmTop();
  await clickText("新版");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.layout")) === "harbor",
  );
  await evaluate("document.querySelector('.appearance-sub > summary')?.click()");
  if (!(await evaluate("document.querySelector('.appearance-sub')?.open"))) {
    await evaluate("document.querySelector('.appearance-sub > summary')?.click()");
  }
  await waitFor(
    async () => (await evaluate("document.querySelector('.appearance-sub')?.open")) === true,
  );
  await clickText("纸纹");
  await clickText("点阵");
  await setTheme(THEMES[0]);
  await goSettings();
  const paperLight = "background-paper-light.png";
  await screenshot(join(outDir, paperLight));
  check(`截图 ${paperLight}`, existsSync(join(outDir, paperLight)));
  await setTheme(THEMES[1]);
  const paperDark = "background-paper-dark.png";
  await screenshot(join(outDir, paperDark));
  check(`截图 ${paperDark}`, existsSync(join(outDir, paperDark)));

  // 报告
  const failed = checks.filter((c) => !c.ok);
  const lines = [
    "# 主题矩阵 · v0.54.0",
    "",
    `- 时间：${new Date().toISOString()}`,
    `- 结果：${checks.length - failed.length}/${checks.length} 通过`,
    `- 主矩阵：5 色板 × 明/暗 × 农场/玩家/设置/钓鱼 = 40 张`,
    `- 额外：经典布局 2 张、点阵纸纹背景 2 张`,
    `- 对比度：见 \`npm test\` 的 \`contrast.test.ts\`（7 例：正文/次要/弱/禁用/语义色/5 色板）`,
    `- 启动：窗口先隐藏，前端 \`ui_ready\` 后 show；Rust 2.5s 兜底（\`app/src/lib.rs\`）`,
    "",
    "## 明细",
    "",
    ...reportRows(),
    "",
  ];
  writeFileSync(reportPath, lines.join("\n"), "utf8");

  // HTML 总览
  const byPage = (page) => shots.filter((shot) => shot.page === page);
  const section = (title, items) => `
  <section>
    <h2>${title}</h2>
    <div class="grid">
      ${items
        .map(
          (item) =>
            `<figure><img src="./theme-matrix/${item.file}" loading="lazy" alt="${item.file}"><figcaption>${item.file}</figcaption></figure>`,
        )
        .join("\n")}
    </div>
  </section>`;
  const html = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<title>文字农场 · 主题矩阵 v0.54.0</title>
<style>
  :root { color-scheme: dark; }
  body { margin: 24px; background: #20251d; color: #e9eddf; font: 14px/1.5 "Microsoft YaHei", sans-serif; }
  h1 { font-size: 22px; margin: 0 0 8px; }
  h2 { font-size: 16px; margin: 24px 0 10px; color: #b7c9a4; }
  p { color: #c2cbb2; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 12px; }
  figure { margin: 0; border: 1px solid #454d3d; background: #282e23; padding: 6px; }
  img { width: 100%; display: block; }
  figcaption { margin-top: 4px; color: #9aa58d; font-size: 12px; word-break: break-all; }
</style>
</head>
<body>
  <h1>文字农场 · 主题矩阵 v0.54.0</h1>
  <p>5 色板 × 明/暗 × 农场/玩家/设置/钓鱼，共 40 张；另有经典布局与纸纹背景各 2 张。</p>
  ${PAGES.map((page) => section(page.name, byPage(page.id))).join("\n")}
  ${section("经典布局", [
    { file: classicLight },
    { file: classicDark },
  ])}
  ${section("点阵纸纹背景", [
    { file: paperLight },
    { file: paperDark },
  ])}
</body>
</html>`;
  writeFileSync(htmlPath, html, "utf8");

  console.log(`\n${checks.length - failed.length}/${checks.length} 通过`);
  console.log(`矩阵目录：${outDir}`);
  console.log(`总览：${htmlPath}`);
  if (failed.length) process.exitCode = 1;
} finally {
  await restorePrefs();
  if (ws) {
    try {
      await send("Browser.close");
    } catch {}
    ws.close();
  }
  if (child) {
    try {
      child.kill();
    } catch {}
  }
  await new Promise((r) => setTimeout(r, 300));
}
