import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  // GitHub Pages 的项目站挂在 /<仓库名>/ 下；自定义域名或本地构建保持 "/"。
  // 构建时用 PAGES_BASE 覆盖，例如 PAGES_BASE=/text-farm/ npm run build。
  base: (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env
    ?.PAGES_BASE ?? "/",
  clearScreen: false,
  // 只是为了用隧道把本地预览给手机看：Vite 默认会拦未知 Host（防 DNS rebinding）。
  // 只放行内网穿透常用的域名，别的一律不认。
  preview: {
    allowedHosts: [".lhr.life", ".loca.lt", ".ngrok-free.app", ".trycloudflare.com"],
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: [
        "**/src-tauri/**",
        "**/target/**",
        "**/*.tmp",
        "**/*.tmpdir/**",
        "**/.*.tmpdir/**",
      ],
    },
  },
});
