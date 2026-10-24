import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = join(root, "文字农场.exe");
const shadowDir = join(root, "shadow", "ui-smoke");
const savePath = join(shadowDir, "save.json");
const reportPath = join(root, "docs", "ui-smoke-report.md");
const screenshotPath = join(root, "docs", "v0210-ui-smoke.png");
const animationScreenshotPath = join(root, "docs", "pet-anim-v071.png");
const trainingScreenshotPath = join(root, "docs", "v0210-ui-smoke-training.png");
const ranchScreenshotPath = join(root, "docs", "ranch-v028.png");
const workshopScreenshotPath = join(root, "docs", "workshop-v031.png");
const fishingScreenshotPath = join(root, "docs", "fishing-v032.png");
const fishingGameScreenshotPath = join(root, "docs", "fishing-game-v032.png");
const fishingBgDir = join(root, "app", "frontend", "public", "fishing");
const farmOrderScreenshotPath = join(root, "docs", "farm-orders-v079.png");
const playerScreenshotPath = join(root, "docs", "player-panel-v080.png");
const port = 9333;
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
  const real = join(process.env.APPDATA || "", "app.textfarm.desktop", "save.json");
  const source = existsSync(real) ? real : savePath;
  if (!existsSync(source)) throw new Error(`找不到可用存档：${source}`);
  const save = JSON.parse(readFileSync(source, "utf8"));
  const state = save.state;
  save.schema = 18;
  state.schema = 18;
  state.level = 12;
  state.exp = 0;
  state.coins = 50_000;
  state.daily = {
    day: "2026-10-19",
    harvests: 0,
    waterings: 0,
    pestsCleared: 0,
    steals: 0,
    helps: 0,
    ranchSteals: 0,
    ranchHelps: 0,
    signIn: false,
    claimedHarvest: false,
    claimedWater: false,
    claimedPest: false,
  };
  state.log = [
    { at: 1, text: "豆豆把家里弄乱了，损失 120 金币" },
    { at: 2, text: "雪球和豆豆为食盆打了一架" },
    { at: 3, text: "小柴踩坏了玉米×1，减产了" },
  ];
  state.pets = (state.pets || []).slice(0, 3);
  for (const pet of state.pets) {
    pet.boarded = false;
    pet.level = 20;
    pet.exp = 0;
    pet.satiety = 80;
    pet.energy = 80;
    pet.mood = 20;
    pet.health = 100;
    pet.sickness = null;
    pet.lastTickAt = Date.now();
    pet.lastSickDay = "2099-01-01";
    pet.training = { guard: 0, dig: 0, mischief: 0, companion: 0 };
    pet.lastMischiefDay = "2026-10-18";
    pet.lastActivityDay = "2026-10-18";
  }
  const boarded = JSON.parse(JSON.stringify(state.pets[0]));
  boarded.id = "pet_4";
  boarded.name = "布丁";
  boarded.boarded = true;
  boarded.level = 1;
  boarded.mood = 60;
  state.pets.push(boarded);
  state.petSeq = 4;
  state.ranch = { pens: [] };
  // 真机档里邻居牧场可能今天已经帮过/偷过；测试档统一重置，否则帮忙按钮全是灰的。
  state.npcs = (state.npcs || []).map((npc) => ({
    ...npc,
    ranch: (npc.ranch || []).map((animal) => ({
      ...animal,
      helped: false,
      readyAt: 0,
      remaining: 1,
    })),
  }));
  state.inventory = state.inventory || {};
  state.inventory["crop:yumi"] = 260;
  state.inventory["crop:xiaobaicai"] = 2;
  for (const crop of [
    "xiaobaicai",
    "luobo",
    "caomei",
    "yumi",
    "ganzhe",
    "xiangrikui",
    "putao",
    "xigua",
    "renshen",
    "lingzhi",
    "huolongguo",
    "kafei",
    "yuejiancao",
  ]) {
    state.inventory[`crop:${crop}`] = 99;
  }
  state.inventory["ranch:milk"] = 4;
  state.inventory["ranch:wool"] = 2;
  state.lastBoardingMonth = "2026-09";
  state.boardingDebt = 0;
  state.facilities = state.facilities || {};
  state.facilities.trainingCenter = true;
  state.petCare = {
    dogBed: true,
    catBed: true,
    dogBedBroken: true,
    catBedBroken: false,
    bowls: 1,
    dogToy: true,
    catToy: true,
    dogToyBroken: false,
    catToyBroken: true,
    medicine: 3,
  };
  state.boardingDebt = 0;
  state.lastBoardingMonth = "2026-09";
  mkdirSync(shadowDir, { recursive: true });
  rmSync(savePath, { force: true });
  rmSync(`${savePath}.bak`, { force: true });
  rmSync(join(shadowDir, "backups"), { recursive: true, force: true });
  writeFileSync(savePath, JSON.stringify(save, null, 2), "utf8");
}

async function waitFor(fn, timeoutMs = 8000) {
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

async function connect() {
  const targets = await waitFor(async () => {
    const response = await fetch(`http://127.0.0.1:${port}/json`);
    if (!response.ok) return null;
    const list = await response.json();
    return list.length ? list : null;
  });
  ws = new WebSocket(targets[0].webSocketDebuggerUrl);
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.method === "Page.javascriptDialogOpening") {
      send("Page.handleJavaScriptDialog", { accept: true }).catch(() => {});
    }
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

async function clickHidden(text) {
  const clicked = await evaluate(`(() => {
    const el = [...document.querySelectorAll('button')].find((item) => (item.innerText || '').trim() === ${JSON.stringify(text)});
    if (!el) return false;
    el.click();
    return true;
  })()`);
  if (!clicked) throw new Error(`找不到隐藏按钮「${text}」`);
  await new Promise((r) => setTimeout(r, 180));
}

async function setInput(selector, value) {
  await evaluate(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return false;
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    setter?.call(el, ${JSON.stringify(value)});
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.focus();
    return true;
  })()`);
}

async function advanceTimeControl(label) {
  await clickText("[玩家]", false);
  await waitFor(async () => (await evaluate(`!!document.querySelector('.player-profile')`)) === true);
  if (!(await bodyText()).includes(label)) {
    await clickText("调试：时间加速");
  }
  await clickText(label);
  await clickText("[返回农场]");
  await waitFor(async () => (await evaluate(`!!document.querySelector('.player-profile')`)) === false);
}

async function clickText(text, exact = true, nth = 0) {  const found = await evaluate(`(() => {
    const wanted = ${JSON.stringify(text)};
    const exact = ${exact};
    const list = [...document.querySelectorAll('button, a, [role="button"]')].filter((el) => {
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
    throw new Error(`找不到按钮「${text}」：${JSON.stringify(found.candidates)}`);
  }
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 260));
  return found.text;
}

async function clickDom(selector, text) {
  const clicked = await evaluate(`(() => {
    const el = [...document.querySelectorAll(${JSON.stringify(selector)})].find(
      (item) => (item.innerText || '').trim() === ${JSON.stringify(text)}
    );
    if (!el) return false;
    el.click();
    return true;
  })()`);
  if (!clicked) throw new Error(`找不到 ${selector} 里的「${text}」`);
  await new Promise((r) => setTimeout(r, 250));
}

async function openFold(text) {
  const opened = await evaluate(`(() => {
    const fold = [...document.querySelectorAll('details.ws-fold, details.fish-fold')].find((item) =>
      (item.querySelector('summary')?.innerText || '').includes(${JSON.stringify(text)})
    );
    if (!fold) return false;
    fold.open = true;
    return true;
  })()`);
  if (!opened) throw new Error(`找不到折叠「${text}」`);
  await new Promise((r) => setTimeout(r, 200));
}

async function pressKey(key, code, vk) {
  await send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key,
    code,
    windowsVirtualKeyCode: vk,
    nativeVirtualKeyCode: vk,
  });
  await new Promise((r) => setTimeout(r, 60));
  await send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key,
    code,
    windowsVirtualKeyCode: vk,
    nativeVirtualKeyCode: vk,
  });
}

async function clickEnabled(text) {
  const found = await evaluate(`(() => {
    const el = [...document.querySelectorAll('button')].find((item) =>
      (item.innerText || '').trim() === ${JSON.stringify(text)} &&
      !item.disabled &&
      item.getClientRects().length > 0
    );
    if (!el) return null;
    el.scrollIntoView({ block: 'center', inline: 'center' });
    const rect = el.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  if (!found) throw new Error(`找不到可点的「${text}」`);
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 260));
}

async function clickInRow(rowText, buttonText) {
  const found = await evaluate(`(() => {
    const row = [...document.querySelectorAll('.ranch-recipe')].find((item) =>
      (item.innerText || '').includes(${JSON.stringify(rowText)})
    );
    if (!row) return null;
    const btn = [...row.querySelectorAll('button')].find(
      (item) => (item.innerText || '').trim() === ${JSON.stringify(buttonText)} && !item.disabled
    );
    if (!btn) return null;
    btn.scrollIntoView({ block: 'center', inline: 'center' });
    const rect = btn.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  if (!found) throw new Error(`找不到行「${rowText}」里的「${buttonText}」`);
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 260));
}

async function clickTopIcon(name) {
  const found = await evaluate(`(() => {
    const el = [...document.querySelectorAll('header.icons .icon-item')].find((item) =>
      (item.innerText || '').includes(${JSON.stringify(name)})
    );
    if (!el) return null;
    el.scrollIntoView({ block: 'center', inline: 'center' });
    const rect = el.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  if (!found) throw new Error(`找不到顶部图标「${name}」`);
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 300));
}

async function startApp() {  if (spawnedAlreadyRunning()) throw new Error("检测到已有文字农场进程，请先关掉再跑 UI 测试");
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
  await connect();
}

function spawnedAlreadyRunning() {
  try {
    const out = execFileSync("tasklist", ["/FI", "IMAGENAME eq 文字农场.exe"], { encoding: "utf8" });
    return out.includes("文字农场.exe");
  } catch {
    return false;
  }
}

async function stopApp() {
  if (ws) {
    try { await send("Browser.close"); } catch {}
    ws.close();
  }
  if (child) {
    try { child.kill(); } catch {}
  }
  await new Promise((r) => setTimeout(r, 300));
}

async function screenshot(path = screenshotPath) {
  const shot = await send("Page.captureScreenshot", { format: "png", fromSurface: true });
  writeFileSync(path, Buffer.from(shot.result.data, "base64"));
}

try {
  seedSave();
  await startApp();
  const title = await evaluate("document.title");
  check("页面标题为文字农场", title.includes("文字农场"), title);
  await waitFor(async () => (saveState().orders?.offers.length ?? 0) >= 1);
  check("村庄订单已生成", (saveState().orders?.offers.length ?? 0) >= 1);
  check("村庄订单面板已显示", (await bodyText()).includes("村庄订单"));
  const orderCoins = saveState().coins;
  await clickText("[接单]");
  await waitFor(async () => saveState().orders.active.length === 1);
  check("接单进入进行中", saveState().orders.active.length === 1);
  await screenshot(farmOrderScreenshotPath);
  check("村庄订单截图已保存", existsSync(farmOrderScreenshotPath), farmOrderScreenshotPath);
  await clickText("[交付", false);
  await waitFor(async () => saveState().orders.active.length === 0);
  check("交付订单", saveState().orders.active.length === 0);
  check("订单奖励到账", saveState().coins > orderCoins);
  check(
    "四种水域背景已生成",
    ["creek", "river", "jiang", "sea"].every((name) =>
      existsSync(join(fishingBgDir, `bg_${name}.png`)),
    ),
  );
  check("玩家信息栏已显示", (await bodyText()).includes("农场陪伴你"));
  await clickText("[玩家]", false);
  await waitFor(async () => (await evaluate(`!!document.querySelector('.player-profile')`)) === true);
  check("打开玩家页", await evaluate(`!!document.querySelector('.player-profile')`));
  const playerText = await bodyText();
  check(
    "玩家页整合外观/设置/存档/编辑器",
    ["专精技能", "外观", "设置", "存档与内容包", "内容编辑器", "A2 存档/内容包已接上"].every(
      (text) => playerText.includes(text),
    ),
  );
  check(
    "玩家 tab 已移除",
    !(await evaluate(`[...document.querySelectorAll('.tabs .tab')].some((el) => el.innerText.trim() === '玩家')`)),
  );
  await setInput(".player-name-row input", "测试玩家");
  await clickText("[保存昵称]");
  await waitFor(async () => saveState().ownerName === "测试玩家");
  check("昵称已保存", saveState().ownerName === "测试玩家");
  await clickText("🌻");
  check(
    "头像已切换",
    (await evaluate(`window.localStorage.getItem('farm.ui.playerAvatar')`)) === "🌻",
  );
  await screenshot(playerScreenshotPath);
  check("玩家页截图已保存", existsSync(playerScreenshotPath), playerScreenshotPath);
  await clickText("[返回农场]");
  await waitFor(async () => (await evaluate(`!!document.querySelector('.player-profile')`)) === false);
  check("返回农场", true);

  await clickText("宠物");
  await waitFor(async () => (await bodyText()).includes("宠物用品"));
  check("切到宠物页", (await bodyText()).includes("宠物用品"));

  const before = saveState();
  check(
    "测试档 3 当值 + 1 寄养",
    before.pets.length === 4 && before.pets.filter((pet) => !pet.boarded).length === 3,
    `pets=${before.pets.length}`,
  );
  check("狗窝初始为损坏", before.petCare.dogBedBroken === true);
  const animCards = await evaluate(
    `[...document.querySelectorAll('.pet-select-card')].map((el) => el.innerText.trim()).filter((text) => !text.includes('寄养'))`,
  );
  for (const name of animCards) {
    await clickDom(".pet-select-card", name);
    const still = await evaluate(
      `document.querySelector('.pet-portrait-img')?.className.includes('pet-still')`,
    );
    if (!still) break;
  }
  const idleState = await evaluate(`({
    reduce: matchMedia('(prefers-reduced-motion: reduce)').matches,
    name: getComputedStyle(document.querySelector('.pet-portrait-img')).animationName,
    classes: document.querySelector('.pet-portrait-img').className,
  })`);
  check(
    "宠物头像有待机动画",
    !idleState.classes.includes("pet-still") &&
      (idleState.reduce
        ? idleState.name === "none"
        : idleState.name.includes("pet-idle-bob")),
    JSON.stringify(idleState),
  );
  await clickText("[喂食]", false);
  await new Promise((r) => setTimeout(r, 120));
  const petEmote = await evaluate(`document.querySelector('.pet-emote')?.textContent || ''`);
  check("互动触发宠物动画和表情", petEmote.includes("🍖"), petEmote);
  await screenshot(animationScreenshotPath);
  check("宠物动画截图已保存", existsSync(animationScreenshotPath), animationScreenshotPath);
  await new Promise((r) => setTimeout(r, 900));
  await clickText("畜牧场", false);
  const ranchText = await bodyText();
  check("畜牧场 3 个畜栏", ranchText.includes("畜栏 1") && ranchText.includes("畜栏 3"));
  check("牧场顶部是牧场导航", ranchText.includes("返回农场") && ranchText.includes("畜牧场"));
  check("畜牧场显示金币", ranchText.includes("金币") && ranchText.includes(String(saveState().coins)));
  check("畜牧场可扩建", ranchText.includes("扩建畜栏"));
  check("邻居牧场串门入口", ranchText.includes("邻居牧场"));
  await clickText("[帮忙]");
  await waitFor(async () => saveState().daily.ranchHelps === 1);
  check("牧场帮忙次数独立", saveState().daily.ranchHelps === 1 && saveState().daily.helps === 0, JSON.stringify(saveState().daily));
  await clickText("母鸡 800", false);
  await new Promise((r) => setTimeout(r, 300));
  check(
    "买母鸡进畜栏",
    saveState().ranch.pens.some((pen) => pen.animal && pen.animal.kind === "chicken"),
  );
  const feedBefore = saveState().inventory["ranch:feed"] || 0;
  await clickText("[买专用饲料 300]");
  await waitFor(async () => (saveState().inventory["ranch:feed"] || 0) > feedBefore);
  check("买到专用饲料", (saveState().inventory["ranch:feed"] || 0) > feedBefore);
  await clickText("[喂专用饲料]");
  check("专用饲料提升品质", saveState().ranch.pens[0].animal.quality === 2);
  await clickText("母鸡 800", false);
  await clickText("[买专用饲料 300]");
  await clickText("[繁殖]");
  check("繁殖出幼崽", saveState().ranch.pens[2].animal?.generation === 1);
  await clickText("[买专用饲料 300]");
  await clickText("[喂专用饲料]");
  await new Promise((r) => setTimeout(r, 300));
  check("专用饲料可升到精品", saveState().ranch.pens[0].animal?.quality === 3);
  check("牧场页显示精品品质", (await bodyText()).includes("精品"));
  await clickText("[派去牧场]", false);
  await new Promise((r) => setTimeout(r, 250));
  check(
    "宠物可派去牧场",
    saveState().pets.some((pet) => pet.location === "ranch"),
  );
  await clickText("[扩建畜栏", false);
  await new Promise((r) => setTimeout(r, 300));
  check("扩建畜栏到 4 个", saveState().ranch.pens.length === 4);

  await screenshot(ranchScreenshotPath);
  check("牧场截图已保存", existsSync(ranchScreenshotPath), ranchScreenshotPath);

  check("牧场有制造屋入口", (await bodyText()).includes("去制造屋"));
  await clickText("制造屋", false);
  await waitFor(async () => (await bodyText()).includes("建制造屋"));
  check("制造屋页面已打开", (await bodyText()).includes("建制造屋"));
  await clickText("[建制造屋", false);
  await waitFor(async () => saveState().workshop.built === true);
  check("点击建造制造屋", saveState().workshop.built === true);
  const workshopModernText = await bodyText();
  check(
    "制造屋新版状态卡已显示",
    workshopModernText.includes("现在能开工") && workshopModernText.includes("加工槽"),
  );
  check(
    "制造屋配方已折叠分类",
    workshopModernText.includes("手工台 · 牧场配方") &&
      workshopModernText.includes("手工台 · 作物礼盒") &&
      workshopModernText.includes("设备配方"),
  );
  await openFold("手工台 · 牧场配方");
  await openFold("手工台 · 作物礼盒");
  const craftText = await bodyText();
  check("加工配方已显示", craftText.includes("奶酪：") && craftText.includes("→ 奶酪"));
  check("作物礼盒配方已显示", craftText.includes("作物礼盒") && craftText.includes("玉米礼盒"));
  await clickText("[开工]");
  await waitFor(async () => saveState().workshop.jobs[0]?.recipe === "cheese");
  check("制造屋开工（奶酪）", saveState().workshop.jobs[0]?.recipe === "cheese");
  await clickText("返回农场", false);
  await advanceTimeControl("+1 天");
  await new Promise((r) => setTimeout(r, 600));
  await clickText("制造屋", false);
  await clickText("[取货]");
  await waitFor(async () => (saveState().inventory["ranch:cheese"] || 0) >= 1);
  check("取出一份奶酪", (saveState().inventory["ranch:cheese"] || 0) >= 1);
  await clickInRow("玉米礼盒", "[做礼盒]");
  await waitFor(async () => saveState().workshop.jobs[0]?.cropId === "yumi");
  check("制造屋开工（玉米礼盒）", saveState().workshop.jobs[0]?.cropId === "yumi");
  await clickText("返回农场", false);
  await advanceTimeControl("+1 天");
  await new Promise((r) => setTimeout(r, 600));
  await clickText("制造屋", false);
  await clickText("[取货]");
  await waitFor(async () => (saveState().inventory["craft:box:yumi"] || 0) >= 1);
  check("取出玉米礼盒", (saveState().inventory["craft:box:yumi"] || 0) >= 1);
  const boxCoins = saveState().coins;
  await clickText("[卖1]");
  await waitFor(async () => (saveState().inventory["craft:box:yumi"] || 0) === 0);
  check("礼盒可直接卖", saveState().coins > boxCoins);

  await openFold("设备");
  await openFold("设备配方");
  const equipmentText = await bodyText();
  check(
    "P2 设备已列出",
    ["烤箱", "搅拌台", "榨油机", "纺织机", "蒸馏器"].every((name) =>
      equipmentText.includes(name),
    ),
  );
  check("设备列表已显示", equipmentText.includes("石磨") && equipmentText.includes("装到"));
  await clickText("[装到 1 号槽]");
  await waitFor(async () => saveState().workshop.equipment.includes("stone_mill"));
  check("装石磨到 1 号槽", saveState().workshop.equipment.includes("stone_mill"));
  await clickInRow("石磨 · 玉米淀粉", "[开工]");
  await waitFor(async () => saveState().workshop.jobs[1]?.recipe === "corn_starch");
  check("石磨开工（玉米淀粉）", saveState().workshop.jobs[1]?.recipe === "corn_starch");
  await clickText("返回农场", false);
  await advanceTimeControl("+1 天");
  await new Promise((r) => setTimeout(r, 600));
  await clickText("制造屋", false);
  await clickText("[取货]");
  await waitFor(async () => (saveState().inventory["craft:corn_starch"] || 0) >= 1);
  check("取出玉米淀粉", (saveState().inventory["craft:corn_starch"] || 0) >= 1);
  await clickText("[升级", false);
  await waitFor(async () => saveState().workshop.equipmentLevels?.[1] === 2);
  check("设备升级到 2 级", saveState().workshop.equipmentLevels?.[1] === 2);
  await screenshot(workshopScreenshotPath);
  check("制造屋截图已保存", existsSync(workshopScreenshotPath), workshopScreenshotPath);

  await clickText("畜牧场", false);
  await clickText("宠物");
  const personalityText = await bodyText();
  check(
    "宠物个性/小癖好已显示",
    personalityText.includes("个性：") && personalityText.includes("小癖好："),
  );
  check("品种技能已显示", personalityText.includes("品种技能 ·"));
  check("主动技能按钮已显示", personalityText.includes("[主动 ·"));
  check("行为树当前活动已显示", personalityText.includes("正在："));
  check("牧场岗位已显示", personalityText.includes("牧场岗位"));
  await clickText("钓鱼地", true);
  await waitFor(async () => saveState().pets.some((pet) => pet.location === "fishing"));
  check("宠物可派去钓鱼地", saveState().pets.some((pet) => pet.location === "fishing"));
  check("钓鱼地岗位已显示", (await bodyText()).includes("钓鱼地岗位"));
  await clickTopIcon("钓鱼");
  await waitFor(async () => (await bodyText()).includes("在岸边帮忙"));
  check("钓鱼页显示岸边帮忙的宠物", (await bodyText()).includes("在岸边帮忙"));
  await clickText("宠物", true);
  await waitFor(async () => (await bodyText()).includes("宠物用品"));
  const talentPet = saveState().pets.find((pet) => !pet.boarded);
  if (talentPet) {
    await clickText(talentPet.name);
    await new Promise((r) => setTimeout(r, 250));
    const talentText = await bodyText();
    check("天赋面板已显示", talentText.includes("天赋") && talentText.includes("Lv5 天赋"));
    const talentLabel = talentPet.species === "dog" ? "守门" : "利爪";
    await clickText(talentLabel, false);
    await waitFor(async () => {
      const current = saveState().pets.find((pet) => pet.id === talentPet.id);
      return Boolean(current?.talents?.tier1);
    });
    const picked = saveState().pets.find((pet) => pet.id === talentPet.id)?.talents?.tier1;
    check(`选择天赋（${talentLabel}）`, Boolean(picked), String(picked));
  }
  const moodBeforeSkill = saveState().pets[0].mood;
  const activeLabel = personalityText.match(/\[主动 · ([^\]]+)\]/)?.[1] || "主动技能";
  await clickText("主动 ·", false);
  await waitFor(async () => saveState().pets[0].lastSkillAt > 0);
  check(`点击主动技能（${activeLabel}）`, saveState().pets[0].lastSkillAt > 0);
  check(
    "主动技能产生即时收益或状态变化",
    saveState().pets[0].mood >= moodBeforeSkill,
  );
  const activeDisabled = await evaluate(`[...document.querySelectorAll('button')]
    .filter((el) => el.innerText.includes('主动 ·'))
    .every((el) => el.disabled)`);
  check("主动技能进入冷却", activeDisabled);
  const feeState = saveState();
  const boardingCharged =
    feeState.log.some((entry) => entry.text.includes("月费")) ||
    feeState.lastBoardingMonth !== "2026-09";
  check(
    "寄养月费自动扣款",
    boardingCharged && feeState.coins < 48_000,
    `coins=${feeState.coins}, month=${feeState.lastBoardingMonth}`,
  );
  const boardedPet = saveState().pets.find((pet) => pet.boarded);
  if (boardedPet) {
    await clickText(boardedPet.name, false);
    await new Promise((r) => setTimeout(r, 250));
    const text = await bodyText();
    check("寄养宠物显示月费", text.includes("寄养中") && text.includes("3000"));
    await clickText(saveState().pets.find((pet) => !pet.boarded).name);
    await new Promise((r) => setTimeout(r, 200));
  }
  const layout = await evaluate(`(() => {
    const cards = [...document.querySelectorAll('.pet-care-card')];
    const training = [...document.querySelectorAll('.pet-training')];
    return {
      overflow: document.documentElement.scrollWidth <= window.innerWidth + 2,
      cards: cards.length,
      cardsSized: cards.every((el) => el.getBoundingClientRect().width > 100),
      training: training.length > 0 && training.every((el) => el.getBoundingClientRect().height > 40),
    };
  })()`);
  check("宠物页无横向溢出", layout.overflow);
  check("用品卡片尺寸正常", layout.cards >= 6 && layout.cardsSized, `cards=${layout.cards}`);
  check("训练中心区块有正常高度", layout.training);

  await clickText("[修补 1200]");
  await waitFor(async () => saveState().petCare.dogBedBroken === false);
  check("点击修补狗窝", saveState().petCare.dogBedBroken === false);

  await clickText("[训练]");
  await waitFor(async () => saveState().pets[0].training.guard === 1);
  check("点击训练第一项（守护）", saveState().pets[0].training.guard === 1);
  const afterTrain = await bodyText();
  check("训练点显示 1/4", afterTrain.includes("训练点 1/4"), afterTrain.match(/训练点[^\n]*/)?.[0] || "");

  const petNames = saveState().pets.map((pet) => pet.name);
  if (petNames[1]) {
    await clickText(petNames[1]);
    await new Promise((r) => setTimeout(r, 300));
    const text = await bodyText();
    check(`切换到第二只宠物（${petNames[1]}）`, text.includes(petNames[1]));

    for (let index = 0; index < 4; index += 1) {
      await clickText("[训练]");
    }
    await waitFor(async () => saveState().pets[1].training.guard
      + saveState().pets[1].training.dig
      + saveState().pets[1].training.mischief
      + saveState().pets[1].training.companion === 4);
    check("第二只宠物训练点用满 4/4", (await bodyText()).includes("训练点 4/4"));
    const allDisabled = await evaluate(`[...document.querySelectorAll('button')]
      .filter((el) => el.innerText.trim() === '[训练]')
      .every((el) => el.disabled)`);
    check("训练点用满后四个训练按钮全部禁用", allDisabled);

    await clickText("[送去寄养 3000/月]");
    await waitFor(async () => saveState().pets[1].boarded === true);
    check("点击送去寄养", saveState().pets[1].boarded === true);
    check("寄养中状态已显示", (await bodyText()).includes("寄养中"));

    await clickText("[接回]");
    await waitFor(async () => saveState().pets[1].boarded === false);
    check("点击接回", saveState().pets[1].boarded === false);
  }

  await clickText("[修补 240]");
  await waitFor(async () => saveState().petCare.catToyBroken === false);
  check("点击修补猫玩具", saveState().petCare.catToyBroken === false);

  await clickText("[购买 300]");
  await waitFor(async () => saveState().petCare.medicine === 4);
  check("点击购买药品", saveState().petCare.medicine === 4);

  const inventoryBefore = JSON.stringify(saveState().inventory);
  await clickText("[喂食]");
  await new Promise((r) => setTimeout(r, 400));
  check("点击喂食不报错（库存/饱食度变化）", JSON.stringify(saveState().inventory) !== inventoryBefore || saveState().pets.some((p) => p.satiety !== 80));

  await screenshot();
  check("用品截图已保存", existsSync(screenshotPath), screenshotPath);

  await evaluate(`[...document.querySelectorAll('.pet-training')].at(-1)?.scrollIntoView({ block: 'center' })`);
  await new Promise((r) => setTimeout(r, 250));
  await screenshot(trainingScreenshotPath);
  check("训练中心截图已保存", existsSync(trainingScreenshotPath), trainingScreenshotPath);

  let fightSeen = false;
  for (let index = 0; index < 30 && !fightSeen; index += 1) {
    await clickText("我的农场");
    await advanceTimeControl("+1 小时");
    await new Promise((r) => setTimeout(r, 500));
    await clickText("宠物");
    await clickText("[喂食]");
    await new Promise((r) => setTimeout(r, 150));
    fightSeen = saveState().log.some(
      (entry) => entry.text.includes("抢食") || entry.text.includes("打了一架"),
    );
  }
  check("食盆不足实际触发抢食", fightSeen);

  let mischiefSeen = false;
  for (let index = 0; index < 90 && !mischiefSeen; index += 1) {
    await clickText("我的农场");
    await advanceTimeControl("+1 天");
    await new Promise((r) => setTimeout(r, 400));
    await clickText("宠物");
    const before = saveState();
    if (before.pets.some((pet) => pet.sickness)) {
      try {
        await clickEnabled("[看兽医 500]");
      } catch {}
    }
    await clickEnabled("[喂食]");
    await clickEnabled("[喂食]");
    mischiefSeen = saveState().log.some(
      (entry) =>
        entry.text.includes("拆了一角") ||
        entry.text.includes("咬坏") ||
        entry.text.includes("踩坏"),
    );
  }
  await clickText("宠物");
  check("随机拆家/咬坏/踩踏实际触发", mischiefSeen);

  await clickText("钓鱼", false);
  check("钓鱼页已打开", (await bodyText()).includes("鱼类图鉴") && (await bodyText()).includes("鱼饵"));
  check("钓鱼页显示当前条件", (await bodyText()).includes("季节/昼夜/雨天"));
  await openFold("委托与比赛");
  check(
    "钓鱼委托与比赛已显示",
    (await bodyText()).includes("NPC 委托") && (await bodyText()).includes("今日最重一条"),
  );
  await clickText("[接委托]");
  await waitFor(async () => (saveState().fishing.quests?.active?.length ?? 0) === 1);
  check("接取钓鱼委托", (saveState().fishing.quests?.active?.length ?? 0) === 1);
  await clickText("[换一批]");
  await waitFor(async () => saveState().fishing.quests?.refreshes === 1);
  check("换一批钓鱼委托", saveState().fishing.quests?.refreshes === 1);
  const baitBefore = saveState().fishing.bait;
  await clickText("[买10份 100]");
  await waitFor(async () => saveState().fishing.bait >= baitBefore + 10);
  check("买鱼饵", saveState().fishing.bait >= baitBefore + 10);
  await pressKey("f", "KeyF", 70);
  await waitFor(async () => saveState().fishing.session !== null);
  check("F 抛竿生成会话", saveState().fishing.session !== null);
  await waitFor(
    async () =>
      (await evaluate(`document.querySelector('.fishing-game')?.dataset.phase`)) === "waiting",
  );
  check("等待咬钩动画", true);
  await waitFor(
    async () =>
      (await evaluate(`document.querySelector('.fishing-game')?.dataset.phase`)) === "bite",
    6000,
  );
  check("鱼咬钩提示", true);
  await pressKey("f", "KeyF", 70);
  await waitFor(
    async () =>
      (await evaluate(`document.querySelector('.fishing-game')?.dataset.phase`)) === "playing",
  );
  check("F 开始收线", true);
  check("张力第二维已显示", await evaluate(`!!document.querySelector('.fg-tension')`));
  check(
    "水域背景已加载",
    await evaluate(
      `(() => { const img = document.querySelector('.fg-bg'); return !!img && img.complete && img.naturalWidth > 0 && img.src.includes('/fishing/bg_'); })()`,
    ),
  );
  const gameText = await bodyText();
  check(
    "鱼种行为已显示",
    ["贴底", "上层", "洄游", "爆发"].some((label) => gameText.includes(`行为：${label}`)),
  );
  const frameBefore = await evaluate(`document.querySelector('.fg-frame')?.style.left || ''`);
  await send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key: "d",
    code: "KeyD",
    windowsVirtualKeyCode: 68,
    nativeVirtualKeyCode: 68,
  });
  await new Promise((r) => setTimeout(r, 350));
  await send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key: "d",
    code: "KeyD",
    windowsVirtualKeyCode: 68,
    nativeVirtualKeyCode: 68,
  });
  const frameAfter = await evaluate(`document.querySelector('.fg-frame')?.style.left || ''`);
  check("WASD 移动钓鱼框", frameBefore !== frameAfter, `${frameBefore} -> ${frameAfter}`);
  await screenshot(fishingGameScreenshotPath);
  check("钓鱼小游戏截图已保存", existsSync(fishingGameScreenshotPath), fishingGameScreenshotPath);
  await clickText("[收竿]");
  await waitFor(async () => saveState().fishing.session === null);
  check("收竿清空会话", saveState().fishing.session === null);
  check("钓鱼失败计数", saveState().fishing.stats.fails >= 1);
  await clickText("[开自动]");
  await waitFor(async () => saveState().fishing.auto.enabled === true);
  check("开自动钓鱼", saveState().fishing.auto.enabled === true);
  await clickText("我的农场");
  await advanceTimeControl("+1 天");
  await new Promise((r) => setTimeout(r, 1200));
  await clickText("钓鱼", false);
  await waitFor(async () => saveState().fishing.stats.autoCatches >= 1);
  check("自动钓鱼收鱼", saveState().fishing.stats.autoCatches >= 1);
  check("鱼类图鉴已记录", Object.keys(saveState().fishing.codex).length >= 1);
  await screenshot(fishingScreenshotPath);
  check("钓鱼截图已保存", existsSync(fishingScreenshotPath), fishingScreenshotPath);

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

  if (await evaluate(`!!document.querySelector('.auto-report-overlay')`)) {
    await clickText("[知道了]");
    await new Promise((r) => setTimeout(r, 250));
  }
  await clickText("宠物");
  await waitFor(async () => (await bodyText()).includes("宠物用品"));
  const releaseTarget = saveState().pets.find((pet) => !pet.boarded);
  if (releaseTarget) {
    await clickText(releaseTarget.name);
    await new Promise((r) => setTimeout(r, 250));
  }
  await clickText("[放生]");
  await waitFor(async () => saveState().pets.length === 3);
  check("点击放生（确认框自动接受）", saveState().pets.length === 3);

  const failed = checks.filter((c) => !c.ok);
  const lines = [
    "# 文字农场 UI 点击冒烟报告",
    "",
    `- 时间：${new Date().toISOString()}`,
    `- 影子库：${shadowDir}`,
    `- 结果：${checks.length - failed.length}/${checks.length} 通过`,
    "",
    "| 检查 | 结果 | 详情 |",
    "|---|---|---|",
    ...checks.map((c) => `| ${c.name} | ${c.ok ? "PASS" : "FAIL"} | ${String(c.detail).replaceAll("|", "\\|")} |`),
    "",
    `截图：${existsSync(screenshotPath) ? screenshotPath : "未生成"}`,
    `训练中心截图：${existsSync(trainingScreenshotPath) ? trainingScreenshotPath : "未生成"}`,
    `牧场截图：${existsSync(ranchScreenshotPath) ? ranchScreenshotPath : "未生成"}`,
    `制造屋截图：${existsSync(workshopScreenshotPath) ? workshopScreenshotPath : "未生成"}`,
  ];
  writeFileSync(reportPath, lines.join("\n"), "utf8");
  if (failed.length) process.exitCode = 1;
} catch (error) {
  console.error("UI 测试失败：", error);
  process.exitCode = 2;
} finally {
  await stopApp();
}

