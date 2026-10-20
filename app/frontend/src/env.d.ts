/// <reference types="vite/client" />

// 测试里读 theme.css 用；只声明这一处，避免为一个像素测试引入整套 @types/node。
declare module "node:fs" {
  export function readFileSync(path: string | URL, encoding: "utf8"): string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>;
  export default component;
}
