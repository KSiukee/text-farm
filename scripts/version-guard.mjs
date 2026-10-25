#!/usr/bin/env node
// 兼容入口：守卫本体已迁到 scripts/version_guard.py（圈记版：--selftest + 防空转）。
// 旧命令 `node scripts/version-guard.mjs ...` 仍然可用。
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("version_guard.py", import.meta.url));
const result = spawnSync("python", [script, ...process.argv.slice(2)], {
  stdio: "inherit",
});
if (result.error) {
  console.error(`启动 python 失败：${result.error.message}`);
  process.exit(2);
}
process.exit(result.status ?? 1);
