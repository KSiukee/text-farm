import { spawn } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const exe = join(root, "文字农场.exe");
const shadowDir = join(root, "shadow", "tank-smoke");
const source = join(root, "shadow", "tank-fixture.json");
const shot = join(root, "docs", "tank-room-v062.png");
const port = 9338;
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
  await new Promise((r) => setTimeout(r, 350));
}
async function clickCanvas(cx, cy) {
  const pos = await evaluate(`(() => {
    const c = document.querySelector('.room-canvas');
    if (!c) return null;
    const r = c.getBoundingClientRect();
    return { x: r.left + r.width * ${cx} / 900, y: r.top + r.height * ${cy} / 480 };
  })()`);
  if (!pos) throw new Error("找不到画布");
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: pos.x, y: pos.y, button: "left", clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: pos.x, y: pos.y, button: "left", clickCount: 1 });
  await new Promise((r) => setTimeout(r, 500));
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
  copyFileSync(source, join(shadowDir, "save.json"));
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
        return (await fetch(`http://127.0.0.1:${port}/json`)).ok;
      } catch {
        return false;
      }
    }, 20000);
    await connect();
    await waitFor(async () => (await evaluate("document.readyState")) === "complete");
    await waitFor(async () => (await bodyText()).includes("我的农场"));
    await clickText("牧村", true);
    await waitFor(async () => (await bodyText()).includes("牧村 · 小屋"));
    await clickCanvas(351, 297);
    const text = await bodyText();
    check("点鱼缸打开面板", text.includes("小鱼缸") || text.includes("仓库里的鱼"), text.match(/小鱼缸[^\n]*/)?.[0] ?? "");
    await clickText("放入");
    await new Promise((r) => setTimeout(r, 500));
    check("鱼放进鱼缸", (await bodyText()).includes("1/2"));
    await screenshot(shot);
    check("鱼缸房间截图", true, shot);
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
