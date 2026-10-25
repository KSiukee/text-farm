// 手机竖屏体检（A163/#191 的验收装置）
//
// 做三件事，按 360 / 390 / 430 三种竖屏宽度各跑一遍：
//   1) 横向溢出：scrollWidth > innerWidth，并列出越界元素
//   2) 触摸目标：可点元素小于 40×40 的清单
//   3) 安全区：页面顶部第一行内容有没有被状态栏压住（拿 env(safe-area-inset-top) 对照）
//
// 用法：
//   node scripts/android-audit.mjs
//   node scripts/android-audit.mjs --sizes 360,390,430 --json docs/android-audit.json
//
// 前置：模拟器已起、app 已安装并在前台、debug 构建（要 WebView devtools socket）。
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

const SDK = process.env.ANDROID_HOME || "D:\\deepseek-files\\tools\\android-sdk";
const ADB = join(SDK, "platform-tools", process.platform === "win32" ? "adb.exe" : "adb");
const PKG = process.env.ANDROID_PKG || "app.textfarm.desktop";
const PORT = Number(process.env.CDP_PORT || 9333);
const DENSITY = 480; // 3x，见 wm density
const TAP_MIN = 40;

const args = process.argv.slice(2);
function argOf(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
}
const sizes = argOf("--sizes", "360,390,430").split(",").map(Number);
const jsonOut = argOf("--json", "");

const adb = (a) => execFileSync(ADB, a, { encoding: "utf8" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function setWidth(dp) {
  const px = Math.round((dp * DENSITY) / 160);
  // 高度按 19.5:9 给足，避免内容被高度卡住
  const h = Math.round(px * 2.1667);
  adb(["shell", `wm size ${px}x${h}`]);
  adb(["shell", `wm density ${DENSITY}`]);
}

function forward() {
  let pid = "";
  try {
    pid = adb(["shell", "pidof", PKG]).trim();
  } catch {
    /* 没起来 */
  }
  if (!pid) throw new Error("app 没在跑");
  adb(["forward", `tcp:${PORT}`, `localabstract:webview_devtools_remote_${pid.split(/\s+/)[0]}`]);
}

async function connect() {
  forward();
  const deadline = Date.now() + 20000;
  let page = null;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json`);
      if (r.ok) {
        page = (await r.json()).find((t) => t.type === "page");
        if (page) break;
      }
    } catch {
      /* 再等 */
    }
    await sleep(400);
  }
  if (!page) throw new Error("连不上 WebView devtools");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  const pending = new Map();
  let nextId = 0;
  await new Promise((res, rej) => {
    ws.onopen = res;
    ws.onerror = (e) => rej(new Error(String(e.message || e)));
  });
  ws.onmessage = (ev) => {
    const m = JSON.parse(ev.data);
    if (m.id && pending.has(m.id)) {
      const { res, rej } = pending.get(m.id);
      pending.delete(m.id);
      m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result);
    }
  };
  const send = (method, params = {}) =>
    new Promise((res, rej) => {
      const id = ++nextId;
      pending.set(id, { res, rej });
      ws.send(JSON.stringify({ id, method, params }));
    });
  await send("Runtime.enable");
  return { send, close: () => ws.close() };
}

async function evalJs(cdp, expression) {
  const r = await cdp.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true, userGesture: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
  return r.result?.value;
}

async function tap(cdp, x, y) {
  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x, y, id: 1 }] });
  await sleep(60);
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
}

async function clickText(cdp, text, tag) {
  const hit = await evalJs(
    cdp,
    `(() => {
      const want = ${JSON.stringify(text)}.replace(/\\s+/g, "");
      const nodes = [...document.querySelectorAll(${JSON.stringify(tag)})];
      const t = nodes.find((n) => (n.innerText || "").replace(/\\s+/g, "").includes(want));
      if (!t) return null;
      t.scrollIntoView({ block: "center" });
      const r = t.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    })()`,
  );
  if (!hit) return false;
  await tap(cdp, hit.x, hit.y);
  await sleep(500);
  return true;
}

const MEASURE_JS = `(() => {
  const probe = document.createElement("div");
  probe.style.cssText = "position:absolute;top:0;left:0;padding-top:env(safe-area-inset-top);padding-bottom:env(safe-area-inset-bottom)";
  document.body.appendChild(probe);
  const cs = getComputedStyle(probe);
  const safeTop = parseFloat(cs.paddingTop) || 0;
  const safeBottom = parseFloat(cs.paddingBottom) || 0;
  probe.remove();

  const over = [...document.querySelectorAll("*")]
    .filter((el) => {
      const r = el.getBoundingClientRect();
      return r.width > 0 && (r.right > window.innerWidth + 0.5 || r.left < -0.5);
    })
    .slice(0, 6)
    .map((el) => ({ tag: el.tagName, cls: (el.className || "").toString().slice(0, 28), text: (el.textContent || "").trim().slice(0, 18) }));

  // 可点元素：button / a / [role=button] / input[type=checkbox]
  const small = [];
  for (const el of document.querySelectorAll("button, a, [role=button], input[type=checkbox]")) {
    if (el.disabled) continue; // 灰掉的按钮不可点，不算触摸目标
    // 复选框的触摸目标是它的 label
    const target = el.type === "checkbox" && el.closest("label") ? el.closest("label") : el;
    const r = target.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) continue;
    const cs2 = getComputedStyle(target);
    if (cs2.visibility === "hidden" || cs2.display === "none") continue;
    if (r.width < ${TAP_MIN} || r.height < ${TAP_MIN}) {
      small.push({
        tag: target.tagName,
        cls: (target.className || "").toString().slice(0, 24),
        text: (target.innerText || target.value || "").trim().slice(0, 16),
        w: Math.round(r.width),
        h: Math.round(r.height),
      });
    }
  }

  // 顶部第一行可见内容是否被状态栏压住（用文档坐标，跟当前滚动位置无关）
  const kids = [...document.querySelectorAll(".page > *")].filter((el) => el.getBoundingClientRect().height > 0);
  const firstRect = kids[0] ? kids[0].getBoundingClientRect() : null;
  const firstTop = firstRect ? Math.round(firstRect.top + window.scrollY) : null;

  return {
    innerWidth: window.innerWidth,
    innerHeight: window.innerHeight,
    scrollWidth: document.documentElement.scrollWidth,
    safeTop,
    safeBottom,
    firstTop,
    overflow: over,
    smallCount: small.length,
    small: small.slice(0, 12),
  };
})()`;

const report = [];
const cdp = await connect();
try {
  for (const w of sizes) {
    setWidth(w);
    await sleep(1200);
    // 页面清单：顶部图标导航（6 个）+ 农场页的文字页签
    const topNames = await evalJs(cdp, "[...document.querySelectorAll('.icons .icon-item')].map((b) => b.innerText.trim())");
    const tabNames = await evalJs(cdp, "[...document.querySelectorAll('nav.tabs .tab')].map((b) => b.innerText.trim())");
    const pages = [
      ...topNames.map((n) => ({ name: n.replace(/\s+/g, ""), kind: "top" })),
      ...tabNames.map((n) => ({ name: n, kind: "tab" })),
    ];
    for (const p of pages) {
      const ok =
        p.kind === "top"
          ? await clickText(cdp, p.name, ".icons .icon-item")
          : await clickText(cdp, p.name, "nav.tabs .tab");
      if (!ok) {
        report.push({ width: w, page: p.name, kind: p.kind, missing: true });
        continue;
      }
      // 统一滚到顶，量的就是"首屏"
      await evalJs(cdp, "(document.scrollingElement && (document.scrollingElement.scrollTop = 0), window.scrollTo(0, 0), true)");
      await sleep(300);
      const m = await evalJs(cdp, MEASURE_JS);
      report.push({ width: w, page: p.name, kind: p.kind, ...m });
    }
    // 回到农场，下一轮从同一起点开始
    await clickText(cdp, "农场", ".icons .icon-item");
  }
} finally {
  cdp.close();
}

if (jsonOut) writeFileSync(jsonOut, JSON.stringify(report, null, 2));

// ---- 控制台报告 ----
let bad = 0;
for (const w of sizes) {
  const rows = report.filter((r) => r.width === w);
  console.log(`\n=== ${w}dp ===`);
  for (const r of rows) {
    if (r.missing) {
      console.log(`  ? ${r.page}（点不到）`);
      continue;
    }
    const overflow = r.scrollWidth > r.innerWidth + 0.5;
    const stretched = r.innerWidth > w + 1; // 内容把 layout viewport 撑宽（Android WebView 会这么干）
    const flag = overflow || stretched || r.smallCount > 0 || (r.firstTop !== null && r.firstTop < r.safeTop);
    if (flag) bad += 1;
    console.log(
      `  ${flag ? "!" : "·"} ${r.page.padEnd(6)} 视口 ${r.innerWidth}(期望 ${w}) 宽 ${r.scrollWidth}/${r.innerWidth}` +
        ` 安全区 ${r.safeTop}/${r.safeBottom} 首行 ${r.firstTop}` +
        ` 小按钮 ${r.smallCount}` +
        (overflow || stretched ? ` 溢出 ${JSON.stringify(r.overflow)}` : ""),
    );
    if (r.smallCount) console.log(`      小按钮样本：${JSON.stringify(r.small.slice(0, 6))}`);
  }
}
console.log(`\n有问题的页/宽度组合：${bad}`);
process.exit(bad ? 1 : 0);
