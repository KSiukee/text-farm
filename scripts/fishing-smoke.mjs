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
      "localStorage.removeItem('farm.autoReportShownAt'); localStorage.removeItem('farm.ui.themeMode'); localStorage.removeItem('farm.ui.palette'); localStorage.removeItem('farm.ui.stylePack'); localStorage.removeItem('farm.ui.bgMode'); localStorage.removeItem('farm.ui.bgPaper'); localStorage.removeItem('farm.ui.bgFit'); localStorage.removeItem('farm.ui.bgImageAlpha'); localStorage.removeItem('farm.ui.bgPanelAlpha'); localStorage.removeItem('farm.ui.fontId'); localStorage.removeItem('farm.ui.fontName')",
    );
    await evaluate(`new Promise((resolve) => {
      const request = indexedDB.open('text-farm-ui', 1);
      request.onupgradeneeded = () => {
        const database = request.result;
        if (!database.objectStoreNames.contains('kv')) database.createObjectStore('kv');
      };
      request.onsuccess = () => {
        const database = request.result;
        if (!database.objectStoreNames.contains('kv')) {
          database.close();
          resolve(true);
          return;
        }
        const tx = database.transaction('kv', 'readwrite');
        tx.objectStore('kv').delete('background-image');
        tx.objectStore('kv').delete('font-file');
        tx.oncomplete = () => {
          database.close();
          resolve(true);
        };
        tx.onerror = () => {
          database.close();
          resolve(false);
        };
      };
      request.onerror = () => resolve(false);
    })`);
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
  check(
    "布局按钮（经典/新版）",
    (await evaluate("document.querySelectorAll('.style-pack-btn').length")) === 2,
  );
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

  // #76 背景与不透明度：纸纹 → 面板滑杆 → 图片上传 → 刷新保持 → 清理
  await evaluate("document.querySelector('.appearance-sub > summary')?.click()");
  await waitFor(
    async () => (await evaluate("document.querySelector('.appearance-sub')?.open")) === true,
  );
  check("背景设置已展开", true);
  await clickText("纸纹");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.bgMode")) === "paper",
  );
  await clickText("点阵");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.bgPaper")) === "dots",
  );
  const paperState = await evaluate(`({
    className: document.querySelector('.app-bg')?.className || '',
    image: getComputedStyle(document.querySelector('.app-bg')).backgroundImage
  })`);
  check(
    "纸纹：点阵生效",
    paperState.className.includes("app-bg--dots") && paperState.image !== "none",
    JSON.stringify(paperState),
  );
  await evaluate(`(() => {
    const el = document.querySelector('.bg-slider--panel');
    el.value = '78';
    el.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await waitFor(
    async () =>
      (await evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--panel-alpha').trim()",
      )) === "0.78",
  );
  const panelState = await evaluate(`({
    alpha: getComputedStyle(document.documentElement).getPropertyValue('--panel-alpha').trim(),
    panelBg: getComputedStyle(document.querySelector('.panel')).backgroundColor,
    stored: localStorage.getItem('farm.ui.bgPanelAlpha')
  })`);
  check(
    "面板不透明度 78% 生效",
    panelState.alpha === "0.78" &&
      panelState.panelBg.endsWith("0.78)") &&
      panelState.stored === "78",
    JSON.stringify(panelState),
  );
  await screenshot(join(root, "docs", "background-paper-v053.png"));
  await clickText("图片");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.bgMode")) === "image",
  );
  await send("DOM.enable");
  const documentTree = await send("DOM.getDocument", { depth: -1 });
  const fileNode = await send("DOM.querySelector", {
    nodeId: documentTree.result.root.nodeId,
    selector: "input.bg-file",
  });
  check("图片模式出现文件框", !!fileNode.result.nodeId);
  const bgFile = join(root, "docs", "theme-light-v051.png");
  await send("DOM.setFileInputFiles", { nodeId: fileNode.result.nodeId, files: [bgFile] });
  await waitFor(
    async () =>
      (await evaluate("localStorage.getItem('farm.ui.bgMode')")) === "image" &&
      (await evaluate(
        "(document.querySelector('.app-bg-image')?.style.backgroundImage || '').includes('data:image')",
      )),
    15000,
  );
  const imageState = await evaluate(`({
    image: (document.querySelector('.app-bg-image')?.style.backgroundImage || '').slice(0, 30),
    alpha: getComputedStyle(document.documentElement).getPropertyValue('--bg-image-alpha').trim(),
    preview: !!document.querySelector('.bg-preview')
  })`);
  check(
    "背景图已压缩并生效",
    imageState.image.includes("data:image") && imageState.alpha === "0.3" && imageState.preview,
    JSON.stringify(imageState),
  );
  await screenshot(join(root, "docs", "background-image-v053.png"));
  await evaluate(`(() => {
    const el = document.querySelector('.bg-slider--image');
    el.value = '45';
    el.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await waitFor(
    async () =>
      (await evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--bg-image-alpha').trim()",
      )) === "0.45",
  );
  check("图片不透明度 45% 生效", true);
  await clickText("纸纹");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.bgMode")) === "paper",
  );
  check(
    "切纸纹后图片层隐藏",
    (await evaluate("!document.querySelector('.app-bg-image')")) === true,
  );
  await clickText("图片");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.bgMode")) === "image",
  );
  await waitFor(
    async () => (await evaluate("!!document.querySelector('.app-bg-image')")) === true,
  );
  check("切回图片层恢复", true);
  await evaluate("window.__bgReloadProbe = 1");
  await send("Page.reload", { ignoreCache: true });
  await waitFor(
    async () => (await evaluate("typeof window.__bgReloadProbe")) === "undefined",
  );
  await waitFor(async () => (await bodyText()).includes("我的农场"));
  await waitFor(
    async () =>
      (await evaluate(
        "(document.querySelector('.app-bg-image')?.style.backgroundImage || '').includes('data:image')",
      )),
    10000,
  );
  const reloadBg = await evaluate(`({
    mode: document.documentElement.dataset.bgMode,
    alpha: getComputedStyle(document.documentElement).getPropertyValue('--bg-image-alpha').trim(),
    panel: getComputedStyle(document.documentElement).getPropertyValue('--panel-alpha').trim()
  })`);
  check(
    "刷新后背景图仍在（IndexedDB）",
    reloadBg.mode === "image" && reloadBg.alpha === "0.45" && reloadBg.panel === "0.78",
    JSON.stringify(reloadBg),
  );
  await evaluate("document.querySelector('.appearance-sub > summary')?.click()");
  await waitFor(
    async () => (await evaluate("document.querySelector('.appearance-sub')?.open")) === true,
  );
  await clickText("[移除图片]");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.bgMode")) === "solid",
  );
  check("移除图片回纯色", true);
  await clickText("恢复背景默认");
  await waitFor(
    async () =>
      (await evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--panel-alpha').trim()",
      )) === "0.92",
  );
  check("恢复背景默认（面板 92%）", true);

  // #79 字体：预设 → 上传真实 TTF → 刷新后从 IndexedDB 恢复 → 切回/移除
  check(
    "6 个字体预设 + 自定义",
    (await evaluate("document.querySelectorAll('.font-preset').length")) === 6 &&
      (await evaluate("!!document.querySelector('.font-custom')")),
  );
  await clickText("黑体");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.font")) === "hei",
  );
  const heiFont = await evaluate(`({
    css: getComputedStyle(document.documentElement).getPropertyValue('--font-ui').trim(),
    body: getComputedStyle(document.body).fontFamily,
    stored: localStorage.getItem('farm.ui.fontId')
  })`);
  check(
    "黑体预设生效",
    heiFont.css.includes("Microsoft YaHei") &&
      heiFont.body.includes("Microsoft YaHei") &&
      heiFont.stored === "hei",
    JSON.stringify(heiFont),
  );
  await clickText("宋体");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.font")) === "song",
  );
  check(
    "回宋体",
    (await evaluate("getComputedStyle(document.body).fontFamily")).includes("SimSun"),
  );
  const testFont = "C:\\Windows\\Fonts\\consola.ttf";
  check("测试字体文件存在", existsSync(testFont), testFont);
  await evaluate("document.querySelector('.font-file')?.scrollIntoView({ block: 'center' })");
  await send("DOM.enable");
  const fontTree = await send("DOM.getDocument", { depth: -1 });
  const fontNode = await send("DOM.querySelector", {
    nodeId: fontTree.result.root.nodeId,
    selector: "input.font-file",
  });
  check("字体文件框存在", !!fontNode.result.nodeId);
  await send("DOM.setFileInputFiles", { nodeId: fontNode.result.nodeId, files: [testFont] });
  await waitFor(
    async () =>
      (await evaluate("localStorage.getItem('farm.ui.fontId')")) === "custom" &&
      (await evaluate("document.fonts.check('16px FarmCustom')")) === true,
    20000,
  );
  const customFont = await evaluate(`({
    css: getComputedStyle(document.documentElement).getPropertyValue('--font-ui').trim(),
    body: getComputedStyle(document.body).fontFamily,
    name: localStorage.getItem('farm.ui.fontName'),
    stored: localStorage.getItem('farm.ui.fontId')
  })`);
  check(
    "自定义字体已加载",
    customFont.css.includes("FarmCustom") &&
      customFont.body.includes("FarmCustom") &&
      customFont.name === "consola" &&
      customFont.stored === "custom",
    JSON.stringify(customFont),
  );
  await screenshot(join(root, "docs", "font-custom-v055.png"));
  await evaluate("window.__fontReloadProbe = 1");
  await send("Page.reload", { ignoreCache: true });
  await waitFor(
    async () => (await evaluate("typeof window.__fontReloadProbe")) === "undefined",
  );
  await waitFor(async () => (await bodyText()).includes("我的农场"));
  await waitFor(
    async () =>
      (await evaluate("document.documentElement.dataset.font")) === "custom" &&
      (await evaluate("document.fonts.check('16px FarmCustom')")) === true,
    20000,
  );
  check(
    "刷新后自定义字体仍在（IndexedDB）",
    (await evaluate("getComputedStyle(document.body).fontFamily")).includes("FarmCustom"),
  );
  await clickText("宋体");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.font")) === "song",
  );
  check(
    "切回宋体后自定义字体文件还在",
    (await evaluate("localStorage.getItem('farm.ui.fontName')")) === "consola",
  );
  await clickText("自定义");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.font")) === "custom",
  );
  check(
    "点自定义可复用已存字体",
    (await evaluate("document.fonts.check('16px FarmCustom')")) === true,
  );
  await clickText("[移除自定义字体]");
  await waitFor(
    async () => (await evaluate("localStorage.getItem('farm.ui.fontName')")) === "",
  );
  check(
    "移除自定义字体回宋体",
    (await evaluate("document.documentElement.dataset.font")) === "song" &&
      (await evaluate("document.querySelector('.font-custom')?.disabled")) === true,
  );

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
  const waitBox = await evaluate(`(() => {
    const r = document.querySelector('.fg-wait')?.getBoundingClientRect();
    return r ? { w: r.width, h: r.height } : null;
  })()`);
  check(
    "等待窗口宽屏比例",
    !!waitBox && Math.abs(waitBox.w / waitBox.h - 2.3) < 0.08,
    JSON.stringify(waitBox),
  );
  await screenshot(waitShot);
  check("等待动画截图", existsSync(waitShot), waitShot);
  await waitFor(async () => (await evaluate("document.querySelector('.fishing-game')?.dataset.phase")) === "bite", 6000);
  check("鱼咬钩提示", true);
  await pressKey("f", "KeyF", 70);
  await waitFor(async () => (await evaluate("document.querySelector('.fishing-game')?.dataset.phase")) === "playing");
  check("F 开始收线", true);
  const pondBox = await evaluate(`(() => {
    const r = document.querySelector('.fg-pond')?.getBoundingClientRect();
    return r ? { w: r.width, h: r.height } : null;
  })()`);
  check(
    "收线窗口与等待同尺寸",
    !!pondBox &&
      !!waitBox &&
      Math.abs(pondBox.w - waitBox.w) < 2 &&
      Math.abs(pondBox.h - waitBox.h) < 2,
    JSON.stringify({ wait: waitBox, pond: pondBox }),
  );
  const frameColorStart = await evaluate(
    "getComputedStyle(document.querySelector('.fg-frame')).borderColor",
  );
  const frameBox = await evaluate(`(() => {
    const r = document.querySelector('.fg-frame')?.getBoundingClientRect();
    return r ? { w: r.width, h: r.height } : null;
  })()`);
  check(
    "框保持正方形",
    !!frameBox && Math.abs(frameBox.w / frameBox.h - 1) < 0.06,
    JSON.stringify(frameBox),
  );
  const frameBadge = await evaluate(
    "document.querySelector('.fg-frame-pct')?.textContent.trim() || ''",
  );
  check("框上百分比", /^\d+%$/.test(frameBadge), frameBadge);
  await new Promise((r) => setTimeout(r, 1200));
  const trailCount = await evaluate("document.querySelectorAll('.fg-trail img').length");
  check("拖影最多 2 段", trailCount <= 2, `trail=${trailCount}`);
  const frameColorNow = await evaluate(
    "getComputedStyle(document.querySelector('.fg-frame')).borderColor",
  );
  check("框颜色随进度变化", frameColorStart !== frameColorNow, `${frameColorStart} -> ${frameColorNow}`);
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
  await clickText("[收下]");
  await waitFor(async () => !(await evaluate("!!document.querySelector('.catch-card')")));
  check("收下关闭渔获卡片", true);

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

  // #78 经典布局：平铺、section 常开、无卡片；切回新版收合。
  async function closeAutoReportIfAny() {
    await new Promise((r) => setTimeout(r, 200));
    if (await evaluate("!!document.querySelector('.auto-report-card')")) {
      await clickText("[知道了]");
      await waitFor(
        async () => !(await evaluate("!!document.querySelector('.auto-report-card')")),
      );
    }
  }

  const classicShot = join(root, "docs", "fishing-classic-v052.png");
  await clickText("我的农场");
  await clickText("经典");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.layout")) === "classic",
  );
  check("切经典布局", true);
  await clickText("钓鱼", false);
  await waitFor(async () => (await bodyText()).includes("鱼类图鉴"));
  await closeAutoReportIfAny();
  await evaluate("document.querySelector('.fishing-page')?.scrollIntoView({ block: 'start' })");
  await new Promise((r) => setTimeout(r, 260));
  const classicState = await evaluate(`(() => {
    const page = document.querySelector('.fishing-page');
    const gear = document.querySelector('.fish-section--gear');
    const auto = document.querySelector('.fish-section--auto');
    const play = document.querySelector('.fish-section--play');
    const codex = document.querySelector('.fish-section--codex');
    const card = document.querySelector('.fish-card--play');
    const style = getComputedStyle(card);
    const order = [play, gear, auto, codex].filter(Boolean);
    const orderOk =
      order.length === 4 &&
      order.every(
        (el, i) =>
          i === 0 ||
          (order[i - 1].compareDocumentPosition(el) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0,
      );
    return {
      gearOpen: gear?.open === true,
      autoOpen: auto?.open === true,
      sections: page ? page.querySelectorAll('.fish-section').length : 0,
      playBg: style.backgroundColor,
      playBorder: style.borderLeftWidth,
      orderOk,
    };
  })()`);
  check(
    "经典：钓具/自动常开",
    classicState.gearOpen && classicState.autoOpen,
    JSON.stringify(classicState),
  );
  check(
    "经典：去掉卡片底色与左边框",
    classicState.playBg === "rgba(0, 0, 0, 0)" && classicState.playBorder === "0px",
    `${classicState.playBg} / ${classicState.playBorder}`,
  );
  check("经典：6 个稳定区块", classicState.sections === 6, `sections=${classicState.sections}`);
  check("经典：顺序 play→gear→auto→codex", classicState.orderOk);
  await screenshot(classicShot);
  check("经典布局截图", existsSync(classicShot), classicShot);
  await evaluate("document.querySelector('.fish-section--gear > summary')?.click()");
  check(
    "经典：点标题不会合上",
    (await evaluate("document.querySelector('.fish-section--gear')?.open")) === true,
  );
  await clickText("我的农场");
  await clickText("新版");
  await waitFor(
    async () => (await evaluate("document.documentElement.dataset.layout")) === "harbor",
  );
  await clickText("钓鱼", false);
  await waitFor(async () => (await bodyText()).includes("鱼类图鉴"));
  await closeAutoReportIfAny();
  check(
    "切回新版：折叠恢复",
    (await evaluate("document.querySelector('.fish-section--gear')?.open")) === false,
  );

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
