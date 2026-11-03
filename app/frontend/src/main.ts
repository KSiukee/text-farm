import { invoke } from "@tauri-apps/api/core";
import { createApp } from "vue";
import App from "./App.vue";
import "./theme.css";
import "./style.css";
import "./mobile.css";

// 有的 WebView 冷启动时 (hover: none) 还没生效，悬浮摇杆/大圆键会一整个不出现，
// 摸几下屏幕才"卡出来"。这里直接按触摸能力打一个类，CSS 用它兜底。
if (typeof navigator !== "undefined" && navigator.maxTouchPoints > 0) {
  document.documentElement.classList.add("touch");
}
import { applyBackgroundPrefs, readBackgroundPrefs } from "./background";
import { applyFontPrefs, readFontPrefs } from "./font";
import { applyStylePack, readStylePack } from "./style-pack";
import { applyTheme, readThemePrefs } from "./theme";

// 挂载前先把上次那套主题/布局/背景参数贴上，避免先白闪一下再变色。
const prefs = readThemePrefs(window.localStorage);
const systemDark =
  typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)").matches
    : false;
applyTheme(document.documentElement, prefs, systemDark);
applyStylePack(document.documentElement, readStylePack(window.localStorage));
applyBackgroundPrefs(
  document.documentElement,
  readBackgroundPrefs(window.localStorage),
);
// 自定义字体是异步从 IndexedDB 读的；这里先落 fallback / 预设，App 挂载后再换。
applyFontPrefs(document.documentElement, readFontPrefs(window.localStorage));

/// 窗口底色跟当前纸面走（Tauri 专有；浏览器里直接跳过）。
async function paintWindowBackground(): Promise<void> {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const paper = getComputedStyle(document.documentElement).getPropertyValue("--paper").trim();
  if (!paper) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().setBackgroundColor(paper);
  } catch {
    // 没有窗口权限就只靠 CSS 底色，不影响使用。
  }
}

createApp(App).mount("#app");

// 窗口由 Rust 先隐藏；主题/背景/窗口底色都贴好后再 show，首帧就是对的。
// Rust 侧另有 2.5 秒兜底，前端没起来也不会一直看不见窗口。
void paintWindowBackground().finally(() => {
  if (!("__TAURI_INTERNALS__" in window)) return;
  window.setTimeout(() => {
    void invoke("ui_ready").catch(() => {
      // 兜底由 Rust 计时器接管。
    });
  }, 40);
});
