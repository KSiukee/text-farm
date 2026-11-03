import { invoke } from "@tauri-apps/api/core";
import { isWeb, webCall } from "./web-backend";
import type {
  Action,
  AppInfo,
  ContentPack,
  ContentPreview,
  CropDef,
  DispatchReply,
  ImportReply,
  RanchBreedPreview,
  SeedReview,
  Snapshot,
} from "./types";

/// 同一套前端跑两个壳：桌面/安卓走 Tauri 命令，浏览器走 wasm。
function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isWeb) return webCall<T>(cmd, args);
  return invoke<T>(cmd, args);
}

export const getSnapshot = () => call<Snapshot>("get_snapshot");
export const getContent = () => call<ContentPack>("get_content");
export const getAppInfo = () => call<AppInfo>("app_info");
export const previewRanchBreeding = (a: number, b: number) =>
  call<RanchBreedPreview>("preview_ranch_breeding", { a, b });
export const dispatch = (action: Action) =>
  call<DispatchReply>("dispatch", { action });
export const sync = () => call<DispatchReply>("sync");
export const advanceTime = (hours: number) =>
  call<DispatchReply>("advance_time", { hours });
export const saveNow = () => call<void>("save_now");
/// 手机壳：进钓鱼小游戏时藏起系统栏（**不改方向**）。桌面/浏览器是空实现或忽略。
export const setImmersive = (enable: boolean) => call<void>("set_immersive", { enable });
export const exportSave = (path: string) => call<void>("export_save", { path });
export const importSave = (path: string) =>
  call<ImportReply>("import_save", { path });
// Android 上 dialog 选出来的是 content:// URI，std::fs 写不进去：
// 所以多一对「文本版」命令，文件读写交给 fs 插件（它支持 content URI）。
export const exportSaveText = () => call<string>("export_save_text");
export const importSaveText = (text: string) =>
  call<ImportReply>("import_save_text", { text });
export const exportContent = (path: string) =>
  call<void>("export_content", { path });
export const importContent = (path: string) =>
  call<ImportReply>("import_content", { path });
export const exportContentText = () => call<string>("export_content_text");
export const importContentText = (text: string) =>
  call<ImportReply>("import_content_text", { text });
export const applyContent = (pack: ContentPack) =>
  call<ImportReply>("apply_content", { pack });
export const previewContent = (pack: ContentPack) =>
  call<ContentPreview>("preview_content", { pack });
export const generateSeeds = (count: number, seed: number) =>
  call<CropDef[]>("generate_content_seeds", { count, seed });
export const reviewSeeds = (count: number, seed: number) =>
  call<SeedReview[]>("review_content_seeds", { count, seed });

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
