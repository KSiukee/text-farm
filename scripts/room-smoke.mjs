import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = join(root, "文字农场.exe");
const shadowDir = join(root, "shadow", "room-smoke");
const savePath = join(shadowDir, "save.json");
const shot = join(root, "docs", "room-v056.png");
const editShot = join(root, "docs", "room-edit-v056.png");
const port = 9336;
const checks = [];
let child = null;
let ws = null;
let nextId = 0;
const pending = new Map();

function check(name, ok, detail = "") {
  checks.push({ name, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
}
async function waitFor(fn, timeoutMs = 10000) {
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
  await new Promise((r) => setTimeout(r, 250));
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

try {
  if (spawnedAlreadyRunning()) throw new Error("检测到已有文字农场进程，请先关掉再跑");
  rmSync(shadowDir, { recursive: true, force: true });
  mkdirSync(shadowDir, { recursive: true });
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
  await waitFor(async () => {
    try {
      return (await bodyText()).includes("我的农场");
    } catch {
      return false;
    }
  });

  await clickText("牧村", true, 0);
  await waitFor(async () => (await bodyText()).includes("牧村 · 小屋"));
  check("牧村页出现小屋", true);
  check("小屋画布存在", !!(await evaluate("!!document.querySelector('.room-canvas')")));
  check("普通模式有装修按钮", (await bodyText()).includes("装修"));
  check("普通模式有解锁里屋按钮", (await bodyText()).includes("解锁里屋"));
  await screenshot(shot);

  await clickText("装修", false);
  await waitFor(async () => (await bodyText()).includes("完成"));
  check("进入装修模式", true);
  check("编辑器有家具面板", (await bodyText()).includes("+绿植"));
  check("默认有一扇窗", (await bodyText()).includes("窗户 1/5"));
  await clickText("+绿植", true);
  await new Promise((r) => setTimeout(r, 200));
  check("加绿植后出现选中", (await bodyText()).includes("选中：绿植"));
  await clickText("+小窗", true);
  await new Promise((r) => setTimeout(r, 200));
  check("加窗后 2/5", (await bodyText()).includes("窗户 2/5"));
  await evaluate(`(() => {
    const select = document.querySelectorAll('.room-editor select')[0];
    if (!select) return false;
    select.value = 'tile';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`);
  await new Promise((r) => setTimeout(r, 200));
  await screenshot(editShot);
  await clickText("完成", false);
  await waitFor(async () => !!(await bodyText()).includes("装修"), 8000);
  await new Promise((r) => setTimeout(r, 500));
  const save = JSON.parse(readFileSync(savePath, "utf8")).state;
  const layout = save.room.rooms.living;
  check("完成写回存档", layout.furniture.length >= 7, `furniture=${layout.furniture.length}`);
  check("地板样式写回", layout.floor === "tile", `floor=${layout.floor}`);
  check("窗户写回", layout.windows.length >= 2, `windows=${layout.windows.length}`);
  check("退出装修模式", (await bodyText()).includes("装修") && !(await bodyText()).includes("完成"));
  check("小屋截图", existsSync(shot), shot);
  check("装修截图", existsSync(editShot), editShot);

  const failed = checks.filter((c) => !c.ok);
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
