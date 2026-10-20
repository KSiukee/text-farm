import { createApp } from "vue";
import App from "./App.vue";
import "./theme.css";
import "./style.css";
import { applyBackgroundPrefs, readBackgroundPrefs } from "./background";
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

/// 窗口底色跟当前纸面走（Tauri 专有；浏览器里直接跳过）。
function paintWindowBackground() {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const paper = getComputedStyle(document.documentElement).getPropertyValue("--paper").trim();
  if (!paper) return;
  void import("@tauri-apps/api/window")
    .then(({ getCurrentWindow }) => getCurrentWindow().setBackgroundColor(paper))
    .catch(() => {
      // 没有窗口权限就只靠 CSS 底色，不影响使用。
    });
}

paintWindowBackground();

createApp(App).mount("#app");
