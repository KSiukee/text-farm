import { spawn } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = join(root, "文字农场.exe");
const shadowDir = join(root, "shadow", "pet-room-smoke");
const savePath = join(shadowDir, "save.json");
const source = join(root, "shadow", "breeds", "save.json");
const shot = join(root, "docs", "pet-room-v061.png");
const port = 9337;
const checks = [];
let child = null;
let ws = null;
let nextId = 0;
const pending = new Map();

function check(name, ok, detail = "") {
  checks.push({ name, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
}
async function waitFor(fn, timeoutMs = 12000) {
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
  const message = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (message.result.exceptionDetails) throw new Error(JSON.stringify(message.result.exceptionDetails));
  return message.result.result.value;
}
async function bodyText() {
  return evaluate("document.body.innerText");
}
async function clickText(text, exact = false) {
  const found = await evaluate(`(() => {
    const list = [...document.querySelectorAll('button, a, [role="button"]')].filter((el) => {
      const value = (el.innerText || el.textContent || '').trim();
      return el.getClientRects().length > 0 && (${exact} ? value === ${JSON.stringify(text)} : value.includes(${JSON.stringify(text)}));
    });
    const el = list[0];
    if (!el) return null;
    el.scrollIntoView({ block: 'center', inline: 'center' });
    const rect = el.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  if (!found) throw new Error(`找不到按钮「${text}」`);
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: found.x, y: found.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 300));
}
async function screenshot(path) {
  const capture = await send("Page.captureScreenshot", { format: "png", fromSurface: true });
  writeFileSync(path, Buffer.from(capture.result.data, "base64"));
}
async function connect() {
  const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
  const page = list.find((item) => item.type === "page");
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
    }
  };
  await send("Page.enable");
  await send("Runtime.enable");
}
async function main() {
  rmSync(shadowDir, { recursive: true, force: true });
  mkdirSync(shadowDir, { recursive: true });
  copyFileSync(source, savePath);
  child = spawn(exe, [], {
    cwd: root,
    env: {
      ...process.env,
      TEXT_FARM_DATA_DIR: shadowDir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    stdio: "ignore",
  });
  try {
    await waitFor(async () => {
      try {
        const response = await fetch(`http://127.0.0.1:${port}/json`);
        return response.ok;
      } catch {
        return false;
      }
    }, 20000);
    await connect();
    await waitFor(async () => (await evaluate("document.readyState")) === "complete");
    await waitFor(async () => (await bodyText()).includes("我的农场"));
    await clickText("牧村", true);
    await waitFor(async () => (await bodyText()).includes("牧村 · 小屋"));
    check("牧村页出现小屋", true);
    check("有可呼唤宠物", (await bodyText()).includes("呼唤宠物"));
    await clickText("呼唤宠物");
    await new Promise((r) => setTimeout(r, 1200));
    const text = await bodyText();
    check("呼唤后 3/3", text.includes("3/3"), text.match(/呼唤宠物[^\n]*/)?.[0] ?? "");
    const sampleFrame = async () =>
      evaluate(`(() => {
        const c = document.querySelector('.room-canvas');
        const data = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
        let hash = 0;
        for (let i = 0; i < data.length; i += 97) hash = (hash * 31 + data[i]) >>> 0;
        return hash;
      })()`);
    const frameA = await sampleFrame();
    await new Promise((r) => setTimeout(r, 800));
    const frameB = await sampleFrame();
    check("房间画面逐帧变化（宠物动画）", frameA !== frameB, `${frameA} -> ${frameB}`);
    await screenshot(shot);
    check("宠物房间截图", true, shot);
  } finally {
    if (ws) ws.close();
    if (child) child.kill();
  }
  const passed = checks.filter((item) => item.ok).length;
  console.log(`\n${passed}/${checks.length} 通过`);
  if (passed !== checks.length) process.exit(1);
}

main().catch((error) => {
  console.error(error);
  if (child) child.kill();
  process.exit(1);
});
