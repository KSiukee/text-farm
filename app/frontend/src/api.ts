import { invoke } from "@tauri-apps/api/core";
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

export const getSnapshot = () => invoke<Snapshot>("get_snapshot");
export const getContent = () => invoke<ContentPack>("get_content");
export const getAppInfo = () => invoke<AppInfo>("app_info");
export const previewRanchBreeding = (a: number, b: number) =>
  invoke<RanchBreedPreview>("preview_ranch_breeding", { a, b });
export const dispatch = (action: Action) =>
  invoke<DispatchReply>("dispatch", { action });
export const sync = () => invoke<DispatchReply>("sync");
export const advanceTime = (hours: number) =>
  invoke<DispatchReply>("advance_time", { hours });
export const saveNow = () => invoke<void>("save_now");
export const exportSave = (path: string) => invoke<void>("export_save", { path });
export const importSave = (path: string) =>
  invoke<ImportReply>("import_save", { path });
// Android 上 dialog 选出来的是 content:// URI，std::fs 写不进去：
// 所以多一对「文本版」命令，文件读写交给 fs 插件（它支持 content URI）。
export const exportSaveText = () => invoke<string>("export_save_text");
export const importSaveText = (text: string) =>
  invoke<ImportReply>("import_save_text", { text });
export const exportContent = (path: string) =>
  invoke<void>("export_content", { path });
export const importContent = (path: string) =>
  invoke<ImportReply>("import_content", { path });
export const exportContentText = () => invoke<string>("export_content_text");
export const importContentText = (text: string) =>
  invoke<ImportReply>("import_content_text", { text });
export const applyContent = (pack: ContentPack) =>
  invoke<ImportReply>("apply_content", { pack });
export const previewContent = (pack: ContentPack) =>
  invoke<ContentPreview>("preview_content", { pack });
export const generateSeeds = (count: number, seed: number) =>
  invoke<CropDef[]>("generate_content_seeds", { count, seed });
export const reviewSeeds = (count: number, seed: number) =>
  invoke<SeedReview[]>("review_content_seeds", { count, seed });

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
