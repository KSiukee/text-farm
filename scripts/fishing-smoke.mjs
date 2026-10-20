import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = join(root, "文字农场.exe");
const shadowDir = join(root, "shadow", "fishing-smoke");
const savePath = join(shadowDir, "save.json");
const reportPath = join(root, "docs", "fishing-smoke-report.md");
const gameShot = join(root, "docs", "fishing-game-v032.png");
const mobileShot = join(root, "docs", "fishing-game-mobile-v049.png");
const layoutShot = join(root, "docs", "fishing-page-v050.png");
const waitShot = join(root, "docs", "fishing-wait-v032.png");
const catchShot = join(root, "docs", "fishing-catch-v032.png");
const codexShot = join(root, "docs", "fishing-codex-v032.png");
const autoReportShot = join(root, "docs", "fishing-auto-report-v032.png");
const bootReportShot = join(root, "docs", "fishing-boot-report-v037.png");
const autoSettingsShot = join(root, "docs", "fishing-auto-settings-v032.png");
const pageShot = join(root, "docs", "fishing-v032.png");
const port = 9334;
const checks = [];
let child = null;
let ws = null;
let nextId = 0;
const pending = new Map();

function check(name, ok, detail = "") {
  checks.push({ name, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
}
function saveState() {
  return JSON.parse(readFileSync(savePath, "utf8")).state;
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
  if (message.result.exceptionDetails) throw new Error(JSON.stringify(message.result.exceptionDetails));
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
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 220));
}
async function pressKey(key, code, vk) {
  await send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  await new Promise((r) => setTimeout(r, 60));
  await send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
}
async function screenshot(path) {
  const shot = await send("Page.captureScreenshot", { format: "png", fromSurface: true });
  writeFileSync(path, Buffer.from(shot.result.data, "base64"));
}
function spawnedAlreadyRunning() {
  try {
    const out = execFileSync("tasklist", ["/FI", "IMAGENAME eq 文字农场.exe"], { encoding: "utf8" });
    return out.includes("文字农场.exe");
  } catch {
    return false;
  }
}
async function startApp({ fresh = true } = {}) {
  if (spawnedAlreadyRunning()) throw new Error("检测到已有文字农场进程，请先关掉再跑");
  if (fresh) {
    rmSync(shadowDir, { recursive: true, force: true });
    mkdirSync(shadowDir, { recursive: true });
  }
  child = spawn(exe, [], {
    cwd: root,
    env: {
      ...process.env,
      TEXT_FARM_DATA_DIR: shadowDir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
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
  // 上一跑收尾的「启动时弹报告」会把已读时间写到未来 60 秒；这值存在 WebView profile 里、
  // 不随 shadow 目录清掉。每跑先清零再重载，否则下一跑会把「该弹的报告」误判成已读。
  try {
    await evaluate(
      "localStorage.removeItem('farm.autoReportShownAt'); localStorage.removeItem('farm.ui.themeMode'); localStorage.removeItem('farm.ui.palette')",
    );
    await send("Page.reload", { ignoreCache: true });
    await waitFor(async () => (await evaluate("document.readyState")) === "complete");
    await waitFor(async () => (await bodyText()).includes("我的农场"));
  } catch {}
}

try {
  await startApp();
  const themeDefaultShot = join(root, "docs", "theme-default-v051.png");
  const themeDarkShot = join(root, "docs", "theme-dark-v051.png");
  const themePageText = await bodyText();
  check(
    "外观设置已显示",
    themePageText.includes("外观") && themePageText.includes("明暗") && themePageText.includes("配色"),
  );
  check("5 个色板按钮", (await evaluate("document.querySelectorAll('.swatch').length")) === 5);
  const themeInitial = await evaluate(`({
    theme: document.documentElement.dataset.theme,
    palette: document.documentElement.dataset.palette,
    systemDark: matchMedia('(prefers-color-scheme: dark)').matches
  })`);
  check(
    "默认跟随系统 + sage",
    themeInitial.palette === "sage" &&
      themeInitial.theme === (themeInitial.systemDark ? "dark" : "light"),
    JSON.stringify(themeInitial),
  );
  await clickText("深色");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.theme")) === "dark",
  );
  const darkBg = await evaluate("getComputedStyle(document.body).backgroundColor");
  check("切深色立即生效", darkBg === "rgb(32, 37, 29)", darkBg);
  await clickText("薄荷绿");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.palette")) === "mint",
  );
  const mintAccent = await evaluate(
    "getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()",
  );
  check("切色板立即生效", mintAccent === "#a8d8c9", mintAccent);
  await screenshot(themeDarkShot);
  check("暗色截图", existsSync(themeDarkShot), themeDarkShot);
  await evaluate("window.__themeReloadProbe = 1");
  await send("Page.reload", { ignoreCache: true });
  await waitFor(
    async () => (await evaluate("typeof window.__themeReloadProbe")) === "undefined",
  );
  await waitFor(async () => (await bodyText()).includes("我的农场"));
  const themeReload = await evaluate(`({
    theme: document.documentElement.dataset.theme,
    palette: document.documentElement.dataset.palette,
    accent: getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()
  })`);
  check(
    "刷新保持主题与色板",
    themeReload.theme === "dark" && themeReload.palette === "mint" && themeReload.accent === "#a8d8c9",
    JSON.stringify(themeReload),
  );
  await clickText("恢复默认");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.palette")) === "sage",
  );
  const themeReset = await evaluate(`({
    theme: document.documentElement.dataset.theme,
    palette: document.documentElement.dataset.palette,
    systemDark: matchMedia('(prefers-color-scheme: dark)').matches
  })`);
  check(
    "恢复默认（跟随系统 + sage）",
    themeReset.palette === "sage" &&
      themeReset.theme === (themeReset.systemDark ? "dark" : "light"),
    JSON.stringify(themeReset),
  );
  await screenshot(themeDefaultShot);
  check("默认主题截图", existsSync(themeDefaultShot), themeDefaultShot);
  await clickText("浅色");
  await waitFor(async () => (await evaluate("document.documentElement.dataset.theme")) === "light");
  const themeLightBg = await evaluate("getComputedStyle(document.body).backgroundColor");
  check("切浅色立即生效", themeLightBg === "rgb(246, 243, 236)", themeLightBg);
  const themeLightShot = join(root, "docs", "theme-light-v051.png");
  await screenshot(themeLightShot);
  check("浅色截图", existsSync(themeLightShot), themeLightShot);
  await clickText("跟随系统");

  await clickText("钓鱼", false);
  await waitFor(async () => (await bodyText()).includes("鱼类图鉴"));
  const state = saveState();
  check("钓鱼页已打开", (await bodyText()).includes("鱼类图鉴"));
  await screenshot(layoutShot);
  check("钓鱼页布局截图", existsSync(layoutShot), layoutShot);
  check("新档 10 份鱼饵", state.fishing.bait === 10, `bait=${state.fishing.bait}`);
  const pageText = await bodyText();
  check("钓鱼等级面板", pageText.includes("钓鱼等级") && pageText.includes("技能点"));
  check(
    "四个钓术已显示",
    ["省饵", "收线", "巨物", "自动化"].every((name) => pageText.includes(name)),
  );
  check(
    "新档钓鱼 Lv1",
    state.skills.fishing.level === 1,
    `level=${state.skills.fishing.level}`,
  );

  // 玩家面板：四专业 + 钓鱼专精 + 与钓鱼页同源
  await clickText("玩家", true);
  await waitFor(async () => (await bodyText()).includes("玩家信息"));
  const playerText = await bodyText();
  check("玩家信息面板已打开", playerText.includes("玩家信息"));
  check(
    "四专业已显示",
    ["种植", "畜牧", "钓鱼", "加工"].every((name) => playerText.includes(name)),
  );
  check(
    "玩家面板种植专精已显示",
    ["精耕", "时令", "保墒", "留种"].every((name) => playerText.includes(name)),
  );
  check(
    "玩家面板畜牧专精已显示",
    ["优育", "增产", "省料", "照料"].every((name) => playerText.includes(name)),
  );
  check(
    "玩家面板加工专精已显示",
    ["出品", "精制", "熟练", "省料"].every((name) => playerText.includes(name)),
  );
  check("玩家面板加成食物入口", playerText.includes("加成食物"));
  check(
    "玩家面板钓鱼专精与钓鱼页同源",
    ["省饵", "收线", "巨物", "自动化"].every((name) => playerText.includes(name)),
  );
  const playerState = saveState();
  check(
    "新档四专业 Lv1",
    playerState.skills.planting.level === 1 &&
      playerState.skills.ranching.level === 1 &&
      playerState.skills.fishing.level === 1 &&
      playerState.skills.processing.level === 1,
  );
  await screenshot(join(root, "docs", "player-panel-v044.png"));
  await clickText("钓鱼", false);
  await waitFor(async () => (await bodyText()).includes("鱼类图鉴"));

  const baitBefore = saveState().fishing.bait;
  await clickText("[买10份 100]");
  await waitFor(async () => saveState().fishing.bait >= baitBefore + 10);
  check("买 10 份鱼饵", saveState().fishing.bait >= baitBefore + 10);

  await pressKey("f", "KeyF", 70);
  await waitFor(async () => saveState().fishing.session !== null);
  check("F 抛竿生成会话", saveState().fishing.session !== null);
  await waitFor(async () => (await evaluate("document.querySelector('.fishing-game')?.dataset.phase")) === "waiting");
  check("等待咬钩动画", true);
  await screenshot(waitShot);
  check("等待动画截图", existsSync(waitShot), waitShot);
  await waitFor(async () => (await evaluate("document.querySelector('.fishing-game')?.dataset.phase")) === "bite", 6000);
  check("鱼咬钩提示", true);
  await pressKey("f", "KeyF", 70);
  await waitFor(async () => (await evaluate("document.querySelector('.fishing-game')?.dataset.phase")) === "playing");
  check("F 开始收线", true);
  await send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 844,
    deviceScaleFactor: 1,
    mobile: true,
  });
  await new Promise((r) => setTimeout(r, 250));
  const stickDisplay = await evaluate(
    "getComputedStyle(document.querySelector('.fg-stick-wrap')).display",
  );
  const stickWidth = await evaluate(
    "document.querySelector('.fg-stick-wrap')?.getBoundingClientRect().width || 0",
  );
  check("窄屏触屏轮盘显示", stickDisplay !== "none" && stickWidth > 0, `${stickDisplay} / ${stickWidth}`);
  await screenshot(mobileShot);
  check("触屏轮盘截图", existsSync(mobileShot), mobileShot);
  await send("Emulation.clearDeviceMetricsOverride");
  await new Promise((r) => setTimeout(r, 120));
  const fishTransform = await evaluate("document.querySelector('.fg-fish')?.style.transform || ''");
  check("鱼头朝向（scaleX 翻转）", /scaleX\(-?1\)/.test(fishTransform), fishTransform);
  const fishBefore = await evaluate("document.querySelector('.fg-fish')?.style.left || ''");
  await new Promise((r) => setTimeout(r, 350));
  const fishAfter = await evaluate("document.querySelector('.fg-fish')?.style.left || ''");
  check("鱼在游动（不停住）", fishBefore !== fishAfter, `${fishBefore} -> ${fishAfter}`);

  const before = await evaluate("document.querySelector('.fg-frame')?.style.left || ''");
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: "d", code: "KeyD", windowsVirtualKeyCode: 68, nativeVirtualKeyCode: 68 });
  await new Promise((r) => setTimeout(r, 400));
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: "d", code: "KeyD", windowsVirtualKeyCode: 68, nativeVirtualKeyCode: 68 });
  const after = await evaluate("document.querySelector('.fg-frame')?.style.left || ''");
  check("WASD 移动框", before !== after, `${before} -> ${after}`);
  await screenshot(gameShot);

  await clickText("[收竿]");
  await waitFor(async () => saveState().fishing.session === null);
  check("收竿清空会话", saveState().fishing.session === null);
  check("失败计数 +1", saveState().fishing.stats.fails >= 1);

  await pressKey("f", "KeyF", 70);
  await waitFor(async () => saveState().fishing.session !== null);
  await waitFor(async () => (await evaluate("document.querySelector('.fishing-game')?.dataset.phase")) === "bite", 6000);
  await pressKey("f", "KeyF", 70);
  await waitFor(async () => (await evaluate("document.querySelector('.fishing-game')?.dataset.phase")) === "playing");
  await evaluate("window.__TAURI_INTERNALS__.invoke('dispatch', { action: { type: 'resolveFishing', success: true } })");
  await waitFor(async () => (await evaluate("document.querySelector('.catch-card')?.innerText || ''")).includes("品质"));
  const catchText = await evaluate("document.querySelector('.catch-card')?.innerText || ''");
  check("钓获过渡卡片", catchText.includes("kg") && catchText.includes("品质"), catchText.replace(/\n/g, " / "));
  await screenshot(catchShot);
  check("钓获截图", existsSync(catchShot), catchShot);
  await pressKey("f", "KeyF", 70);
  await waitFor(async () => !(await evaluate("!!document.querySelector('.catch-card')")));
  check("F 关闭渔获卡片", true);

  const activeSelects = await evaluate("document.querySelectorAll('.bait-select').length");
  check("主动鱼饵选择（种类+品质）", activeSelects === 2, `select=${activeSelects}`);
  const autoSelects = await evaluate("document.querySelectorAll('.auto-settings select').length");
  check("自动设置四项（钓点/鱼竿/饵料/鱼饵）", autoSelects === 4, `select=${autoSelects}`);
  await screenshot(autoSettingsShot);
  check("自动设置截图", existsSync(autoSettingsShot), autoSettingsShot);

  await clickText("[开自动]");
  await waitFor(async () => saveState().fishing.auto.enabled === true);
  check("开自动钓鱼", saveState().fishing.auto.enabled === true);
  await clickText("我的农场");
  await clickText("+1 天");
  await new Promise((r) => setTimeout(r, 1200));
  await waitFor(async () => saveState().fishing.stats.autoCatches >= 1);
  check("自动钓鱼收鱼", saveState().fishing.stats.autoCatches >= 1);
  check("图鉴记录", Object.keys(saveState().fishing.codex).length >= 1);
  check(
    "自动报告不在农场页弹",
    !(await evaluate("!!document.querySelector('.auto-report-card')")),
  );
  // 前端每 5 秒才轮询一次快照；等一轮再进钓鱼页，避免拿到还没带上报告的旧快照。
  await new Promise((r) => setTimeout(r, 6000));

  // 报告只在进入钓鱼页时弹一次；先回农场再进来。
  await clickText("钓鱼", false);
  try {
    await waitFor(
      async () => !!(await evaluate("!!document.querySelector('.auto-report-card')")),
      3000,
    );
  } catch {
    // 偶发点击没切过去：再点一次，避免把「没弹」误判成功能坏了。
    await clickText("钓鱼", false);
    await waitFor(async () => !!(await evaluate("!!document.querySelector('.auto-report-card')")));
  }
  const autoText = await evaluate("document.querySelector('.auto-report-card')?.innerText || ''");
  check(
    "自动钓鱼报告",
    autoText.includes("自动钓鱼报告") &&
      autoText.includes("买了鱼饵") &&
      autoText.includes("消耗鱼饵") &&
      autoText.includes("花费金币"),
    autoText.replace(/\n/g, " / "),
  );
  await screenshot(autoReportShot);
  check("自动报告截图", existsSync(autoReportShot), autoReportShot);
  await clickText("[知道了]");

  await clickText("[锁定]");
  await waitFor(async () => saveState().fishing.lockedFish.length === 1);
  check("锁定鱼不出售", saveState().fishing.lockedFish.length === 1);
  const lockedId = saveState().fishing.lockedFish[0];
  await clickText("[解锁]");
  await waitFor(async () => saveState().fishing.lockedFish.length === 0);
  check("解锁鱼", saveState().fishing.lockedFish.length === 0, lockedId);

  const coins = saveState().coins;
  await clickText("[一键全卖（跳过锁定）]");
  await waitFor(
    async () =>
      saveState().coins > coins &&
      !Object.keys(saveState().inventory).some((key) => key.startsWith("fish:")),
  );
  check("一键全卖", saveState().coins > coins, `${coins} -> ${saveState().coins}`);

  await evaluate("document.querySelector('.fish-codex-item.caught')?.click()");
  await waitFor(async () => (await evaluate("!!document.querySelector('.fish-modal')")));
  const modalText = await evaluate("document.querySelector('.fish-modal')?.innerText || ''");
  check(
    "图鉴放大卡片",
    modalText.includes("主动钓鱼") &&
      modalText.includes("自动钓鱼") &&
      modalText.includes("累计钓到") &&
      modalText.includes("最大重量") &&
      modalText.includes("最高售价"),
    modalText.replace(/\n/g, " / "),
  );
  await screenshot(codexShot);
  check("图鉴截图", existsSync(codexShot), codexShot);
  await clickText("[关闭]");

  await screenshot(pageShot);

  // 关掉再开：把报告时间改成未来并关自动，验证「首次进程序弹一次」。
  if (ws) {
    try { await send("Browser.close"); } catch {}
    ws.close();
    ws = null;
  }
  if (child) {
    try { child.kill(); } catch {}
    child = null;
  }
  for (let i = 0; i < 20 && spawnedAlreadyRunning(); i += 1) {
    await new Promise((r) => setTimeout(r, 150));
  }
  const bootSave = JSON.parse(readFileSync(savePath, "utf8"));
  bootSave.state.fishing.auto.enabled = false;
  bootSave.state.fishing.lastAutoReport = {
    ...bootSave.state.fishing.lastAutoReport,
    at: Date.now() + 60_000,
  };
  writeFileSync(savePath, JSON.stringify(bootSave, null, 2), "utf8");
  await startApp({ fresh: false });
  await waitFor(
    async () => !!(await evaluate("!!document.querySelector('.auto-report-card')")),
    8000,
  );
  check("启动时弹自动报告", true);
  await screenshot(bootReportShot);

  const failed = checks.filter((c) => !c.ok);
  const lines = [
    "# 文字农场 钓鱼 UI 冒烟报告",
    "",
    `- 时间：${new Date().toISOString()}`,
    `- 结果：${checks.length - failed.length}/${checks.length} 通过`,
    "",
    ...checks.map((c) => `- ${c.ok ? "PASS" : "FAIL"} ${c.name}${c.detail ? ` — ${c.detail}` : ""}`),
  ];
  writeFileSync(reportPath, lines.join("\n") + "\n", "utf8");
  console.log(`\n${checks.length - failed.length}/${checks.length} 通过`);
  if (failed.length) process.exitCode = 1;
} finally {
  if (ws) {
    try { await send("Browser.close"); } catch {}
    ws.close();
  }
  if (child) {
    try { child.kill(); } catch {}
  }
  await new Promise((r) => setTimeout(r, 300));
}
