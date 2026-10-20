#!/usr/bin/env node
// 版本号门卫：版本必须四处 + lock + state + CHANGELOG 一致，且按改动类型只升对应位。
// 用法：
//   node scripts/version-guard.mjs --kind fix   微调：只允许第三位 +1
//   node scripts/version-guard.mjs --kind feat  功能：只允许中位 +1、第三位归零
//   node scripts/version-guard.mjs              只查一致性与不跳号
// 一次性纠正旧号（例如 0.57.0 -> 0.55.3）加 --allow-downgrade。
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const argv = process.argv.slice(2);
const kindArg = argv.indexOf("--kind");
const kind = kindArg >= 0 ? argv[kindArg + 1] : "none";
const allowDowngrade = argv.includes("--allow-downgrade");

if (!["none", "fix", "feat"].includes(kind)) {
  console.error("用法：node scripts/version-guard.mjs --kind fix|feat（或省略 --kind）");
  process.exit(2);
}

function read(relative) {
  try {
    return readFileSync(resolve(root, relative), "utf8");
  } catch {
    console.error(`读不到 ${relative}`);
    process.exit(2);
  }
}

const failures = [];
function fail(message) {
  failures.push(message);
}
function check(label, relative, version) {
  console.log(`${version === null ? "✗" : "✓"} ${label.padEnd(12)} ${version ?? "未找到"}  ${relative}`);
  if (version === null) fail(`${relative} 里找不到版本号`);
}

const cargo = read("Cargo.toml");
const cargoVersion = cargo.match(/\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
check("Cargo.toml", "Cargo.toml", cargoVersion);

const tauri = JSON.parse(read("app/tauri.conf.json"));
check("tauri.conf", "app/tauri.conf.json", tauri.version ?? null);

const pkg = JSON.parse(read("app/frontend/package.json"));
check("frontend pkg", "app/frontend/package.json", pkg.version ?? null);

const readmeVersion = read("README.md").match(/^v(\d+\.\d+\.\d+)：/m)?.[1] ?? null;
check("README", "README.md", readmeVersion);

const lockVersion =
  read("Cargo.lock").match(/\[\[package\]\]\nname = "text-farm"\nversion = "([^"]+)"/)?.[1] ?? null;
check("Cargo.lock", "Cargo.lock", lockVersion);

const state = JSON.parse(read(".codex/state.json"));
check("state", ".codex/state.json", state.version ?? null);

const changelogVersion = read("CHANGELOG.md").match(/^## (\d+\.\d+\.\d+) ·/m)?.[1] ?? null;
check("CHANGELOG", "CHANGELOG.md", changelogVersion);

const all = [
  ["Cargo.toml", cargoVersion],
  ["app/tauri.conf.json", tauri.version ?? null],
  ["app/frontend/package.json", pkg.version ?? null],
  ["README.md", readmeVersion],
  ["Cargo.lock", lockVersion],
  [".codex/state.json", state.version ?? null],
  ["CHANGELOG.md", changelogVersion],
];
const distinct = new Set(all.map(([, version]) => version).filter(Boolean));
if (distinct.size > 1) {
  fail(`版本号不一致：${all.map(([name, version]) => `${name}=${version}`).join("，")}`);
}

function parse(version) {
  const match = version?.match(/^(\d+)\.(\d+)\.(\d+)$/);
  return match ? match.slice(1).map(Number) : null;
}

function headVersion() {
  try {
    const text = execFileSync("git", ["show", "HEAD:Cargo.toml"], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return text.match(/\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
  } catch {
    return null;
  }
}

if (cargoVersion) {
  const current = parse(cargoVersion);
  const previousText = headVersion();
  const previous = parse(previousText);
  console.log(`\n当前 ${cargoVersion}，HEAD ${previousText ?? "（无 git 基线）"}`);

  if (!current) {
    fail(`当前版本不是 x.y.z：${cargoVersion}`);
  } else if (!previous) {
    console.log("· 没有 HEAD 基线，只校验一致性");
  } else if (cargoVersion === previousText) {
    if (kind !== "none") {
      fail(`--kind ${kind} 要求升版本，但仍是 ${cargoVersion}`);
    } else {
      console.log("· 未升版本（一致性通过）");
    }
  } else if (
    current[0] < previous[0] ||
    (current[0] === previous[0] && current[1] < previous[1]) ||
    (current[0] === previous[0] && current[1] === previous[1] && current[2] < previous[2])
  ) {
    if (!allowDowngrade) {
      fail(`版本回退：${previousText} → ${cargoVersion}；一次性纠正请加 --allow-downgrade`);
    } else {
      console.log("⚠ 允许一次性版本回退（纠正旧号）");
    }
  } else {
    const sameMajor = current[0] === previous[0];
    const sameMinor = sameMajor && current[1] === previous[1];
    const patchStep = sameMinor && current[2] - previous[2];
    const minorStep = sameMajor && current[1] - previous[1];

    if (kind === "fix" && !(sameMinor && patchStep === 1)) {
      fail(`--kind fix 只允许第三位 +1：${previousText} → ${cargoVersion}`);
    } else if (kind === "feat" && !(sameMajor && current[1] - previous[1] === 1 && current[2] === 0)) {
      fail(`--kind feat 只允许中位 +1、第三位归零：${previousText} → ${cargoVersion}`);
    } else if (kind === "none") {
      if (sameMinor && patchStep > 1) fail(`跳号：第三位一次 +${patchStep}（${previousText} → ${cargoVersion}）`);
      if (sameMajor && minorStep > 1) fail(`跳号：中位一次 +${minorStep}（${previousText} → ${cargoVersion}）`);
    }
  }
}

if (failures.length) {
  console.error("\n版本门卫未通过：");
  for (const message of failures) console.error(` - ${message}`);
  process.exit(1);
}
console.log("\n版本门卫通过。");
