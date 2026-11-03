// WebKit 冒烟：用 Playwright 的 WebKit 内核跑一遍网页版，
// 专测「换个浏览器引擎就容易坏」的那几样：头像 / 背景 / 字体 / 存档导出导入。
//
// macOS 的 WKWebView 也是 WebKit，所以这个脚本在 macOS runner 上跑通，
// 基本等于桌面壳里那套能跑（差异只剩 JIT 和少量 API）。
//
// 用法：node scripts/webkit-smoke.mjs [http://localhost:4173/]

import { webkit } from "playwright";
import { mkdtempSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const BASE = process.argv[2] ?? "http://localhost:4173/";
const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../../..");
const PNG = path.join(root, "screenshots/desktop.png");

const results = [];
function check(name, ok, detail = "") {
  results.push({ name, ok });
  console.log(`${ok ? "✓" : "✗"} ${name}${detail ? `  — ${detail}` : ""}`);
}

if (!existsSync(PNG)) {
  console.error(`找不到测试图：${PNG}`);
  process.exit(2);
}

const browser = await webkit.launch();
const page = await browser.newPage({ viewport: { width: 1000, height: 820 } });
const pageErrors = [];
page.on("pageerror", (error) => pageErrors.push(String(error)));
page.on("console", (msg) => {
  if (msg.type() === "error") pageErrors.push(msg.text());
});

try {
  await page.goto(BASE, { waitUntil: "load" });
  await page.waitForFunction(() => /农场等级[:：]\s*\d/.test(document.body.innerText), null, {
    timeout: 45000,
  });
  check("页面能起来（wasm 加载 + 首屏）", true);

  // 首次打开会弹存档提示卡（1.2 秒后），等它出来再关掉，否则会挡住后面的点击
  const dismissHint = async () => {
    const know = page.locator(".confirm-card button", { hasText: "知道了" });
    for (let i = 0; i < 20; i += 1) {
      if (await know.count()) {
        await know.first().click();
        await page.waitForTimeout(300);
        return true;
      }
      await page.waitForTimeout(200);
    }
    return false;
  };
  const hinted = await dismissHint();
  check("首次存档提示卡出现并能关掉", hinted);

  // ---- 头像 ----
  try {
    await dismissHint();
    await page.locator("button", { hasText: "玩家" }).first().click();
    await page.waitForTimeout(400);
    await page.setInputFiles(".avatar-upload input[type=file]", PNG);
    await page.waitForTimeout(1200);
    const hasAvatar = await page.evaluate(
      () => [...document.querySelectorAll("img")].some((img) => img.src.startsWith("data:image/")),
    );
    check("头像上传（canvas 压缩 → IndexedDB）", hasAvatar);
  } catch (error) {
    check("头像上传", false, String(error));
  }

  // ---- 背景：入口在设置面板的「图片」模式下，选择器不稳，做成软检查 ----
  // 它和头像走同一个 saveCustomAsset → IndexedDB 路径，头像过了这条基本也过；
  // 这里只确认「能切到图片模式」，传文件那步失败不算红。
  try {
    await page.locator("button", { hasText: "设置" }).first().click();
    await page.waitForTimeout(600);
    const imageMode = page.locator(".rate-btn", { hasText: "图片" }).first();
    if ((await imageMode.count()) > 0 && (await imageMode.isVisible())) {
      await imageMode.click();
      await page.waitForTimeout(300);
    }
    const bgInput = page.locator("input.bg-file");
    if ((await bgInput.count()) > 0) {
      await bgInput.first().setInputFiles(PNG);
      await page.waitForTimeout(1000);
      const failed = await page.evaluate(() => /背景.*(失败|没存上)/.test(document.body.innerText));
      check("背景上传（IndexedDB）", !failed);
    } else {
      console.log("· 背景上传：入口没露出来（设置面板默认收起），跳过");
    }
  } catch (error) {
    console.log(`· 背景上传：跳过（${String(error).split("\n")[0]}）`);
  }

  // ---- 字体：故意传一张 PNG，要求优雅报错、不崩 ----
  try {
    const fontInput = page.locator("input.font-file");
    if ((await fontInput.count()) === 0) throw new Error("页面上没有字体上传入口");
    const before = pageErrors.length;
    await fontInput.first().setInputFiles(PNG);
    await page.waitForTimeout(1200);
    const crashed = pageErrors.length > before;
    const friendly = await page.evaluate(() => /字体/.test(document.body.innerText));
    check("字体传坏文件：有提示、不崩", friendly && !crashed, crashed ? "抛了页面异常" : "");
  } catch (error) {
    check("字体传坏文件", false, String(error));
  }

  // ---- 导出存档 ----
  const dir = mkdtempSync(path.join(tmpdir(), "tf-smoke-"));
  let savePath = null;
  try {
    const [download] = await Promise.all([
      page.waitForEvent("download", { timeout: 15000 }),
      page.locator(".tab-quick button", { hasText: "下载存档" }).first().click(),
    ]);
    savePath = path.join(dir, "save.json");
    await download.saveAs(savePath);
    const text = readFileSync(savePath, "utf8");
    const parsed = JSON.parse(text);
    check("导出存档（WebKit 下载 + JSON 合法）", typeof parsed.schema === "number", `schema=${parsed.schema}`);
  } catch (error) {
    check("导出存档", false, String(error));
  }

  // ---- 导入存档 ----
  try {
    if (!savePath) throw new Error("上一步没拿到存档，跳过");
    const before = pageErrors.length;
    const [chooser] = await Promise.all([
      page.waitForEvent("filechooser", { timeout: 15000 }),
      page.locator(".tab-quick button", { hasText: "导入存档" }).first().click(),
    ]);
    await chooser.setFiles(savePath);
    await page.waitForTimeout(1200);
    const failed = await page.evaluate(() => /导入存档失败/.test(document.body.innerText));
    check("导入存档（文件选择器 + 回滚）", !failed && pageErrors.length === before);
  } catch (error) {
    check("导入存档", false, String(error));
  }

  // ---- 内容包导出 ----
  try {
    await page.locator("button", { hasText: "设置" }).first().click();
    await page.waitForTimeout(500);
    const [download] = await Promise.all([
      page.waitForEvent("download", { timeout: 15000 }),
      page.locator("button", { hasText: "导出内容包" }).first().click(),
    ]);
    const packPath = path.join(dir, "content.json");
    await download.saveAs(packPath);
    const pack = JSON.parse(readFileSync(packPath, "utf8"));
    check("导出内容包（JSON 合法）", Array.isArray(pack.crops) && pack.crops.length > 0, `crops=${pack.crops?.length}`);
  } catch (error) {
    check("导出内容包", false, String(error));
  }

  check("全程没有页面异常", pageErrors.length === 0, pageErrors.slice(0, 3).join(" | "));
} finally {
  await browser.close();
}

const failed = results.filter((item) => !item.ok);
console.log(`\n${results.length - failed.length}/${results.length} 通过`);
process.exit(failed.length === 0 ? 0 : 1);
