import { spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// 等级奖励 + 鱼饵品质的针对性验收（影子库，不碰真机档）。
const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = join(root, "文字农场.exe");
const shadowDir = join(root, "shadow", "rewards-check");
const savePath = join(shadowDir, "save.json");
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

function readSave() {
  return JSON.parse(readFileSync(savePath, "utf8"));
}

function saveState() {
  return readSave().state;
}

function seedSave() {
  const sources = [
    join(root, "shadow", "ui-smoke", "save.json"),
    join(process.env.APPDATA || "", "app.textfarm.desktop", "save.json"),
  ];
  const source = sources.find((path) => existsSync(path));
  if (!source) throw new Error(`找不到可用存档：${sources.join(" / ")}`);
  mkdirSync(shadowDir, { recursive: true });
  copyFileSync(source, savePath);
  const save = JSON.parse(readFileSync(savePath, "utf8"));
  const state = save.state;
  state.level = 12;
  state.exp = 0;
  state.coins = 50_000;
  state.levelRewardsClaimed = [];
  state.daily = {
    ...state.daily,
    ranchCollects: 0,
    fishCatches: 0,
    claimedRanch: false,
    claimedFish: false,
  };
  state.fishing.baitFine = 0;
  state.fishing.bait = 10;
  state.fishing.rod = "bamboo";
  state.fishing.session = null;
  state.fishing.auto.enabled = false;
  writeFileSync(savePath, JSON.stringify(save, null, 2), "utf8");
}

async function waitFor(fn, timeoutMs = 15_000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const value = await fn();
      if (value) return value;
    } catch {}
    await new Promise((r) => setTimeout(r, 150));
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

async function clickText(text, exact = true, nth = 0) {
  const found = await evaluate(`(() => {
    const wanted = ${JSON.stringify(text)};
    const exact = ${exact};
    const list = [...document.querySelectorAll('button, a, summary, [role="button"]')].filter((el) => {
      const value = (el.innerText || el.textContent || '').trim();
      const visible = el.getClientRects().length > 0;
      return visible && (exact ? value === wanted : value.includes(wanted));
    });
    const el = list[${nth}];
    if (!el) return { ok: false, candidates: list.map((x) => x.innerText.trim()).slice(0, 12) };
    el.scrollIntoView({ block: 'center', inline: 'center' });
    const rect = el.getBoundingClientRect();
    return { ok: true, x: rect.left + rect.width / 2, y: rect.top + rect.height / 2, text: el.innerText.trim() };
  })()`);
  if (!found.ok) {
    throw new Error(`找不到「${text}」：${JSON.stringify(found.candidates)}`);
  }
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 300));
}

async function screenshot(path) {
  const shot = await send("Page.captureScreenshot", { format: "png", fromSurface: true });
  writeFileSync(path, Buffer.from(shot.result.data, "base64"));
}

async function startApp() {
  child = spawn(exe, [], {
    cwd: root,
    env: {
      ...process.env,
      TEXT_FARM_DATA_DIR: shadowDir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    stdio: "ignore",
    windowsHide: false,
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
}

async function stopApp() {
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
  await new Promise((r) => setTimeout(r, 400));
}

try {
  seedSave();
  await startApp();
  await new Promise((r) => setTimeout(r, 1200));

  check("等级奖励面板存在", (await evaluate("document.body.innerText")).includes("等级奖励"));
  const opened = await evaluate(`(() => {
    const summary = [...document.querySelectorAll('summary')].find((el) => (el.innerText || '').includes('等级奖励'));
    if (!summary) return false;
    summary.click();
    return true;
  })()`);
  check("展开等级奖励面板", opened === true);
  await new Promise((r) => setTimeout(r, 300));
  const claimCount = await evaluate(
    "[...document.querySelectorAll('button')].filter((el) => (el.innerText || '').trim() === '[领奖]' && el.getClientRects().length > 0).length",
  );
  check("12 级有 11 份可领", claimCount >= 11, `buttons=${claimCount}`);

  const body = await evaluate("document.body.innerText");
  check("今日任务含牧场收取", body.includes("牧场收 3 次"));
  check("今日任务含钓鱼", body.includes("钓上 3 条鱼"));

  const beforeCoins = saveState().coins;
  const clicked = await evaluate(`(() => {
    const panel = [...document.querySelectorAll('details')].find((item) =>
      (item.querySelector('summary')?.innerText || '').includes('等级奖励')
    );
    if (!panel) return -1;
    const buttons = [...panel.querySelectorAll('button')].filter(
      (item) => (item.innerText || '').trim() === '[领奖]'
    );
    if (!buttons.length) return -2;
    buttons[0].click();
    return buttons.length;
  })()`);
  check("点第一个等级奖励", clicked > 0, `claim buttons=${clicked}`);
  await waitFor(() => {
    try {
      return saveState().levelRewardsClaimed.includes(2);
    } catch {
      return false;
    }
  });
  const after = saveState();
  check("领 2 级奖励记入存档", after.levelRewardsClaimed.includes(2), JSON.stringify(after.levelRewardsClaimed));
  check("2 级奖励金币 +90", after.coins === beforeCoins + 90, `${beforeCoins} -> ${after.coins}`);
  await evaluate(`(() => {
    const panel = [...document.querySelectorAll('details')].find((item) =>
      (item.querySelector('summary')?.innerText || '').includes('等级奖励')
    );
    if (panel) panel.scrollIntoView({ block: 'center' });
    return true;
  })()`);
  await new Promise((r) => setTimeout(r, 400));
  await screenshot(join(root, "docs", "level-rewards-v036.png"));
  check("等级奖励截图已保存", existsSync(join(root, "docs", "level-rewards-v036.png")));

  await send("Emulation.setDeviceMetricsOverride", {
    width: 900,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await new Promise((r) => setTimeout(r, 300));
  const narrow = await evaluate(`({
    overflow: document.documentElement.scrollWidth <= window.innerWidth + 2,
    width: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  })`);
  check("900px 宽无横向溢出", narrow.overflow, `${narrow.width}/${narrow.scrollWidth}`);
  await send("Emulation.clearDeviceMetricsOverride");

  const openedFishing = await evaluate(`(() => {
    const el = [...document.querySelectorAll('.icon-item')].find((item) =>
      (item.innerText || '').includes('钓鱼')
    );
    if (!el) return false;
    el.click();
    return true;
  })()`);
  check("切到钓鱼页", openedFishing === true);
  await waitFor(async () => (await evaluate("document.body.innerText")).includes("鱼类图鉴"));
  const beforeBait = saveState().fishing.baitFine;
  const beforeBaitCoins = saveState().coins;
  const changed = await evaluate(`(() => {
    const select = document.querySelector('.bait-select');
    if (!select) return false;
    select.value = 'fine';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`);
  check("切到优良鱼饵", changed === true);
  await new Promise((r) => setTimeout(r, 300));
  const buyClicked = await evaluate(`(() => {
    const el = [...document.querySelectorAll('button')].find((item) =>
      (item.innerText || '').trim().startsWith('[买10份')
    );
    if (!el) return null;
    el.click();
    return el.innerText.trim();
  })()`);
  check("点买 10 份鱼饵", buyClicked === "[买10份 250]", String(buyClicked));
  await waitFor(() => saveState().fishing.baitFine >= beforeBait + 10);
  const baitAfter = saveState();
  check("买到的是优良鱼饵", baitAfter.fishing.baitFine === beforeBait + 10, `baitFine=${baitAfter.fishing.baitFine}`);
  check("扣的是优良价 250", baitAfter.coins === beforeBaitCoins - 250, `${beforeBaitCoins} -> ${baitAfter.coins}`);
  const subText = await evaluate(
    `[...document.querySelectorAll('.sub')].map((el) => el.innerText).join(' | ')`,
  );
  check("顶部鱼饵显示所选品质与数量", subText.includes("优良 ×10"), subText.replaceAll("\n", " "));

  const castClicked = await evaluate(`(() => {
    const el = [...document.querySelectorAll('button')].find((item) =>
      (item.innerText || '').trim().startsWith('[抛竿')
    );
    if (!el) return false;
    el.click();
    return true;
  })()`);
  check("点抛竿", castClicked === true);
  await waitFor(() => saveState().fishing.session !== null);
  const cast = saveState();
  check("抛竿用的是优良鱼饵", cast.fishing.session?.baitQuality === "fine", JSON.stringify(cast.fishing.session?.baitQuality));
  check("activeBait 同步为优良", cast.fishing.activeBait === "fine", cast.fishing.activeBait);
  await screenshot(join(root, "docs", "bait-quality-v036.png"));

  const autoChanged = await evaluate(`(() => {
    const selects = document.querySelectorAll('.auto-settings select');
    if (selects.length < 3) return false;
    selects[0].value = 'river';
    selects[0].dispatchEvent(new Event('change', { bubbles: true }));
    selects[1].value = 'bamboo';
    selects[1].dispatchEvent(new Event('change', { bubbles: true }));
    selects[2].value = 'fine';
    selects[2].dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`);
  check("自动钓鱼设置可改", autoChanged === true);
  await new Promise((r) => setTimeout(r, 300));
  const autoClicked = await evaluate(`(() => {
    const el = [...document.querySelectorAll('button')].find((item) =>
      (item.innerText || '').trim() === '[开自动]'
    );
    if (!el) return false;
    el.click();
    return true;
  })()`);
  check("点开自动", autoClicked === true);
  await waitFor(() => saveState().fishing.auto.enabled === true);
  const auto = saveState().fishing.auto;
  check("自动钓点用所选河流", auto.spot === "river", auto.spot);
  check("自动鱼竿用所选竹竿", auto.rod === "bamboo", auto.rod);
  check("自动鱼饵用所选优良", auto.baitQuality === "fine", auto.baitQuality);

  const failed = checks.filter((c) => !c.ok);
  console.log(`\n${checks.length - failed.length}/${checks.length} 通过`);
  if (failed.length) process.exitCode = 1;
} catch (error) {
  console.error("验收失败：", error);
  process.exitCode = 2;
} finally {
  await stopApp();
}
