// Android WebView 调试驱动（开发期工具，不是产品代码；后续 #191/#192 的验收都用它）
//
// 原理：debug 构建的 WebView 会开 devtools socket；adb forward 之后用 CDP 直接驱动页面，
// 鼠标/触摸事件都走 Input.dispatchTouchEvent（真触摸），不是合成 click。
//
// 用法（先让 app 在前台跑着）：
//   node scripts/android-driver.mjs title
//   node scripts/android-driver.mjs text
//   node scripts/android-driver.mjs eval "<js 表达式>"
//   node scripts/android-driver.mjs click "<文本>" [选择器]
//   node scripts/android-driver.mjs tap <x> <y>
//   node scripts/android-driver.mjs swipe <x1> <y1> <x2> <y2> [ms]
//   node scripts/android-driver.mjs shot <输出路径>
//   node scripts/android-driver.mjs tabs              逐页签量横向溢出
//   node scripts/android-driver.mjs stick [dx] [dy]   拖钓鱼摇杆并采样鱼框/张力
//   node scripts/android-driver.mjs fishflow [截图路径] 抛竿→咬钩→收线→测摇杆
//
// 环境变量：ANDROID_HOME（默认 D:\deepseek-files\tools\android-sdk）、ANDROID_PKG、CDP_PORT
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

const SDK = process.env.ANDROID_HOME || "D:\\deepseek-files\\tools\\android-sdk";
const ADB = join(SDK, "platform-tools", process.platform === "win32" ? "adb.exe" : "adb");
const PKG = process.env.ANDROID_PKG || "app.textfarm.desktop";
const PORT = Number(process.env.CDP_PORT || 9333);

function adb(args) {
  return execFileSync(ADB, args, { encoding: "utf8" });
}

function forward() {
  let pid = "";
  try {
    pid = adb(["shell", "pidof", PKG]).trim();
  } catch {
    /* 没起来 */
  }
  if (!pid) {
    const unix = adb(["shell", "cat /proc/net/unix"]);
    const m = unix.match(/webview_devtools_remote_(\d+)/);
    if (!m) throw new Error("找不到 WebView 调试 socket（app 没起来，或不是 debug 构建）");
    pid = m[1];
  }
  const name = `webview_devtools_remote_${pid.split(/\s+/)[0]}`;
  adb(["forward", `tcp:${PORT}`, `localabstract:${name}`]);
  return name;
}

async function connect() {
  const socket = forward();
  const deadline = Date.now() + 20000;
  let page = null;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json`);
      if (r.ok) {
        const list = await r.json();
        page = list.find((t) => t.type === "page");
        if (page) break;
      }
    } catch {
      /* 再等 */
    }
    await new Promise((r) => setTimeout(r, 400));
  }
  if (!page) throw new Error(`连不上 ${socket} 的 /json`);
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  const pending = new Map();
  let nextId = 0;
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = (e) => reject(new Error(`WebSocket 失败：${e.message || e}`));
  });
  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) reject(new Error(JSON.stringify(msg.error)));
      else resolve(msg.result);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = ++nextId;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  await send("Runtime.enable");
  await send("Page.enable").catch(() => {});
  return { send, close: () => ws.close() };
}

async function evalJs(cdp, expression) {
  const r = await cdp.send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
    userGesture: true,
  });
  if (r.exceptionDetails) {
    throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
  }
  return r.result?.value;
}

async function touch(cdp, type, points) {
  await cdp.send("Input.dispatchTouchEvent", {
    type,
    touchPoints: points.map((p, i) => ({ x: p[0], y: p[1], id: i + 1 })),
  });
}

async function tap(cdp, x, y) {
  await touch(cdp, "touchStart", [[x, y]]);
  await new Promise((r) => setTimeout(r, 60));
  await touch(cdp, "touchEnd", []);
}

async function swipe(cdp, x1, y1, x2, y2, ms = 400) {
  const steps = Math.max(6, Math.round(ms / 40));
  await touch(cdp, "touchStart", [[x1, y1]]);
  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    await touch(cdp, "touchMove", [[x1 + (x2 - x1) * t, y1 + (y2 - y1) * t]]);
    await new Promise((r) => setTimeout(r, ms / steps));
  }
  await touch(cdp, "touchEnd", []);
}

async function clickText(cdp, text, tag = "button, a, [role=button], label") {
  const hit = await evalJs(
    cdp,
    `(() => {
      const nodes = [...document.querySelectorAll(${JSON.stringify(tag)})];
      const t = nodes.find((n) => ((n.innerText || "").trim()).includes(${JSON.stringify(text)}));
      if (!t) return null;
      t.scrollIntoView({ block: "center" });
      const r = t.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2, text: (t.innerText || "").trim().slice(0, 40), w: r.width, h: r.height };
    })()`,
  );
  if (!hit) throw new Error(`找不到含「${text}」的可点元素`);
  await tap(cdp, hit.x, hit.y);
  return hit;
}

const OVERFLOW_JS = `(() => {
  const d = document.documentElement;
  const over = [...document.querySelectorAll("*")]
    .filter((el) => {
      const r = el.getBoundingClientRect();
      return r.width > 0 && (r.right > window.innerWidth + 0.5 || r.left < -0.5);
    })
    .slice(0, 5)
    .map((el) => ({ tag: el.tagName, cls: (el.className || "").toString().slice(0, 30), text: (el.textContent || "").trim().slice(0, 20) }));
  const active = document.querySelector(".tab.active") || document.querySelector("nav.tabs .tab.active");
  return { tab: active ? active.innerText.trim() : "", innerWidth: window.innerWidth, scrollWidth: d.scrollWidth, clientWidth: d.clientWidth, overflow: over };
})()`;

const SAMPLE_JS = `(() => {
  const f = document.querySelector('.fg-frame');
  const st = document.querySelector('.fg-stick');
  const k = document.querySelector('.fg-stick-knob');
  const t = document.querySelector('.fg-tension-value');
  const p = document.querySelector('.fg-progress span');
  return {
    frame: f ? f.style.cssText.slice(0, 70) : "",
    knob: k ? k.style.cssText.slice(0, 50) : "",
    active: st ? st.classList.contains('active') : false,
    tension: t ? t.textContent : "",
    progress: p ? p.style.cssText.slice(0, 50) : "",
  };
})()`;

async function probeStick(cdp, dx = 34, dy = -34, ms = 800) {
  const geo = await evalJs(
    cdp,
    `(() => {
      const st = document.querySelector('.fg-stick');
      if (!st) return null;
      const b = st.getBoundingClientRect();
      return { cx: b.left + b.width / 2, cy: b.top + b.height / 2, w: b.width, h: b.height };
    })()`,
  );
  if (!geo) throw new Error("找不到摇杆（不在打架阶段？）");
  const before = await evalJs(cdp, SAMPLE_JS);
  const samples = [];
  const steps = 8;
  await touch(cdp, "touchStart", [[geo.cx, geo.cy]]);
  for (let i = 1; i <= steps; i++) {
    await touch(cdp, "touchMove", [[geo.cx + (dx * i) / steps, geo.cy + (dy * i) / steps]]);
    await new Promise((r) => setTimeout(r, ms / steps));
    samples.push(await evalJs(cdp, SAMPLE_JS));
  }
  await touch(cdp, "touchEnd", []);
  const after = await evalJs(cdp, SAMPLE_JS);
  return { geo, before, samples, after };
}

async function waitFor(cdp, jsExpr, timeoutMs = 25000, stepMs = 250) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await evalJs(cdp, jsExpr)) return true;
    await new Promise((r) => setTimeout(r, stepMs));
  }
  return false;
}

const [cmd, ...rest] = process.argv.slice(2);
if (!cmd) {
  console.error("缺命令；看文件头注释");
  process.exit(2);
}
const cdp = await connect();
try {
  if (cmd === "text") {
    console.log(await evalJs(cdp, "document.body ? document.body.innerText.slice(0, 2000) : '(no body)'"));
  } else if (cmd === "title") {
    console.log(JSON.stringify(await evalJs(cdp, "({title: document.title, url: location.href, ready: document.readyState})"), null, 2));
  } else if (cmd === "eval") {
    const value = await evalJs(cdp, rest.join(" "));
    console.log(typeof value === "string" ? value : JSON.stringify(value, null, 2));
  } else if (cmd === "click") {
    console.log("clicked", JSON.stringify(await clickText(cdp, rest[0], rest[1] || "button, a, [role=button], label")));
  } else if (cmd === "tap") {
    await tap(cdp, Number(rest[0]), Number(rest[1]));
    console.log("tapped", rest[0], rest[1]);
  } else if (cmd === "swipe") {
    await swipe(cdp, Number(rest[0]), Number(rest[1]), Number(rest[2]), Number(rest[3]), Number(rest[4] || 400));
    console.log("swiped");
  } else if (cmd === "shot") {
    const r = await cdp.send("Page.captureScreenshot", { format: "png" });
    writeFileSync(rest[0], Buffer.from(r.data, "base64"));
    console.log("saved", rest[0]);
  } else if (cmd === "tabs") {
    const list = await evalJs(cdp, "[...document.querySelectorAll('nav.tabs .tab')].map((b) => b.innerText.trim())");
    const report = [];
    for (const name of list) {
      try {
        await clickText(cdp, name, "nav.tabs .tab");
      } catch {
        continue;
      }
      await new Promise((r) => setTimeout(r, 600));
      report.push(await evalJs(cdp, OVERFLOW_JS));
    }
    console.log(JSON.stringify(report, null, 2));
  } else if (cmd === "overflow") {
    console.log(JSON.stringify(await evalJs(cdp, OVERFLOW_JS), null, 2));
  } else if (cmd === "stick") {
    console.log(JSON.stringify(await probeStick(cdp, Number(rest[0] ?? 34), Number(rest[1] ?? -34), Number(rest[2] ?? 800)), null, 2));
  } else if (cmd === "fishflow") {
    const log = [];
    const inPond = () => evalJs(cdp, "!!document.querySelector('.fg-pond')");
    if (!(await inPond())) {
      await clickText(cdp, "抛竿");
      log.push("cast");
      const bitten = await waitFor(cdp, "document.body.innerText.includes('[开始收线]') || !!document.querySelector('.fg-pond')");
      log.push(bitten ? "bite" : "no-bite");
      if (!bitten) {
        console.log(JSON.stringify({ log }, null, 2));
        process.exit(3);
      }
      if (!(await inPond())) {
        await clickText(cdp, "[开始收线]");
        await new Promise((r) => setTimeout(r, 400));
      }
    }
    log.push("playing");
    if (rest[0]) {
      const shot = await cdp.send("Page.captureScreenshot", { format: "png" });
      writeFileSync(rest[0], Buffer.from(shot.data, "base64"));
      log.push(`shot ${rest[0]}`);
    }
    console.log(JSON.stringify({ log, probe: await probeStick(cdp, 34, -34, 900) }, null, 2));
  } else {
    console.error(`未知命令：${cmd}`);
    process.exit(2);
  }
} finally {
  cdp.close();
}
