//! 浏览器后端：把 farm-core 的 wasm 模块接成和 Tauri 命令一样的形状。
//!
//! - 传输：一个 JSON 进、一个 JSON 出，走 C ABI 手写线性内存（见 crates/farm-wasm）
//! - 存档：IndexedDB（`text-farm-web` / `kv`），另留一份 `save:prev` 当上一次的档
//! - 时间：`Date.now()` 注入；时区偏移 `getTimezoneOffset()` 注入，内核不碰日历库

import { idbGet, idbSet } from "./web-storage";

const SAVE_KEY = "save";
const PREV_KEY = "save:prev";

type WasmExports = {
  memory: WebAssembly.Memory;
  farm_alloc: (len: number) => number;
  farm_dealloc: (ptr: number, len: number) => void;
  farm_call: (ptr: number, len: number) => number;
  farm_free_result: (ptr: number) => void;
};

let exportsRef: WasmExports | null = null;
let booting: Promise<void> | null = null;
let bootWarning: string | null = null;
let saveTimer: number | null = null;

export const isWeb = typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

// ------------------------------------------------------------------ wasm 加载

function wasmUrl(): string {
  const base = (import.meta as { env?: { BASE_URL?: string } }).env?.BASE_URL ?? "/";
  return `${base}${base.endsWith("/") ? "" : "/"}farm_wasm.wasm`;
}

async function loadWasm(): Promise<WasmExports> {
  const response = await fetch(wasmUrl());
  if (!response.ok) throw new Error(`加载 farm_wasm.wasm 失败：HTTP ${response.status}`);
  const bytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return instance.exports as unknown as WasmExports;
}

function callRaw(request: Record<string, unknown>): Record<string, unknown> {
  const wasm = exportsRef;
  if (!wasm) throw new Error("wasm 还没加载");

  const input = new TextEncoder().encode(JSON.stringify(request));
  const inPtr = wasm.farm_alloc(input.length);
  new Uint8Array(wasm.memory.buffer, inPtr, input.length).set(input);
  const outPtr = wasm.farm_call(inPtr, input.length);
  wasm.farm_dealloc(inPtr, input.length);

  const length = new DataView(wasm.memory.buffer).getUint32(outPtr, true);
  const text = new TextDecoder().decode(
    new Uint8Array(wasm.memory.buffer, outPtr + 4, length),
  );
  wasm.farm_free_result(outPtr);

  const value = JSON.parse(text) as Record<string, unknown>;
  if (!value.ok) {
    const error = value.error as { message?: string } | undefined;
    throw new Error(error?.message ?? "wasm 调用失败");
  }
  return value;
}

async function ensureBooted(): Promise<void> {
  if (!booting) {
    booting = (async () => {
      exportsRef = await loadWasm();
      const saved = await idbGet(SAVE_KEY);
      if (saved) await idbSet(PREV_KEY, saved);
      const result = callRaw({
        cmd: "boot",
        now: Date.now(),
        tz: new Date().getTimezoneOffset(),
        save: saved,
      });
      bootWarning = (result.warning as string | null) ?? null;
      installFlushHooks();
    })();
  }
  return booting;
}

// ------------------------------------------------------------------ 存档

async function flushSave(): Promise<void> {
  if (!exportsRef) return;
  const result = callRaw({ cmd: "save", now: Date.now() });
  await idbSet(SAVE_KEY, result.save as string);
}

function scheduleSave(): void {
  if (saveTimer !== null) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    saveTimer = null;
    void flushSave().catch((error) => console.warn("存档写入失败", error));
  }, 600);
}

function installFlushHooks(): void {
  const flush = () => {
    if (saveTimer !== null) {
      window.clearTimeout(saveTimer);
      saveTimer = null;
    }
    void flushSave().catch(() => undefined);
  };
  window.addEventListener("pagehide", flush);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") flush();
  });
}

// ------------------------------------------------------------------ 文件出入口
// 浏览器没有文件路径：导出走下载，导入走 <input type=file>。

export function downloadText(filename: string, text: string): void {
  const blob = new Blob([text], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function pickTextFile(): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json,application/json";
    input.addEventListener("change", () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }
      void file.text().then(resolve, () => resolve(null));
    });
    input.addEventListener("cancel", () => resolve(null));
    input.click();
  });
}

// ------------------------------------------------------------------ 命令映射

export async function webCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await ensureBooted();
  const a = args ?? {};
  const now = Date.now();

  switch (cmd) {
    case "get_snapshot":
      return callRaw({ cmd: "snapshot", now }).snapshot as T;

    case "get_content":
      return callRaw({ cmd: "content" }).content as T;

    case "app_info": {
      const info = (await Promise.resolve(callRaw({ cmd: "app_info" }).info)) as Record<
        string,
        unknown
      >;
      return { ...info, bootWarning } as T;
    }

    case "dispatch": {
      const result = callRaw({ cmd: "dispatch", now, action: a.action });
      scheduleSave();
      return { snapshot: result.snapshot, effects: result.effects, saveError: null } as T;
    }

    case "sync": {
      const result = callRaw({ cmd: "sync", now });
      if (result.changed) scheduleSave();
      return { snapshot: result.snapshot, effects: result.effects, saveError: null } as T;
    }

    case "advance_time": {
      const result = callRaw({ cmd: "advance_time", now, hours: a.hours });
      scheduleSave();
      return { snapshot: result.snapshot, effects: result.effects, saveError: null } as T;
    }

    case "save_now":
      await flushSave();
      return undefined as T;

    case "game_mode":
      return undefined as T;

    case "export_save_text":
      return callRaw({ cmd: "save", now }).save as T;

    case "import_save_text": {
      const result = callRaw({ cmd: "import_save", now, text: a.text });
      await flushSave();
      return { snapshot: result.snapshot, content: result.content, warning: null } as T;
    }

    case "export_content_text":
      return callRaw({ cmd: "export_content" }).content as T;

    case "import_content_text": {
      const result = callRaw({ cmd: "import_content", now, text: a.text });
      scheduleSave();
      return { snapshot: result.snapshot, content: result.content, warning: null } as T;
    }

    case "apply_content": {
      const result = callRaw({ cmd: "apply_content", now, pack: a.pack });
      scheduleSave();
      return { snapshot: result.snapshot, content: result.content, warning: null } as T;
    }

    case "preview_content": {
      const result = callRaw({ cmd: "preview_content", pack: a.pack });
      return { issues: result.issues, crops: result.crops } as T;
    }

    case "generate_content_seeds":
      return callRaw({ cmd: "generate_seeds", count: a.count, seed: a.seed }).crops as T;

    case "review_content_seeds":
      return callRaw({ cmd: "review_seeds", count: a.count, seed: a.seed }).reviews as T;

    case "preview_ranch_breeding": {
      const result = callRaw({ cmd: "preview_breeding", now, a: a.a, b: a.b });
      return result.preview as T;
    }

    case "export_save":
    case "import_save":
    case "export_content":
    case "import_content":
      throw new Error("网页版没有文件路径：请用「导出/导入存档文本」");

    default:
      throw new Error(`网页版还不支持这个命令：${cmd}`);
  }
}
