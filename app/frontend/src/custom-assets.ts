/**
 * 玩家自定义资源：头像 / 背景 / 字体的本地文件夹读写。
 * 文件落在应用数据目录的 custom/ 下，不写存档。
 */
import { invoke } from "@tauri-apps/api/core";

export type CustomAssetKind = "avatars" | "backgrounds" | "fonts";

export interface PlayerAssets {
  avatar: string;
}

export async function customAssetsDir(): Promise<string> {
  return invoke<string>("custom_assets_dir");
}

export async function saveCustomAsset(
  kind: CustomAssetKind,
  name: string,
  dataUrl: string,
): Promise<string> {
  return invoke<string>("save_custom_asset", { kind, name, dataUrl });
}

export async function readCustomAsset(
  kind: CustomAssetKind,
  name: string,
): Promise<string | null> {
  return invoke<string | null>("read_custom_asset", { kind, name });
}

export async function deleteCustomAsset(
  kind: CustomAssetKind,
  name: string,
): Promise<void> {
  return invoke<void>("delete_custom_asset", { kind, name });
}

export async function loadPlayerAssets(): Promise<PlayerAssets> {
  return invoke<PlayerAssets>("load_player_assets");
}

export async function savePlayerAssets(assets: PlayerAssets): Promise<void> {
  return invoke<void>("save_player_assets", { assets });
}

/** ArrayBuffer → data URL，给字体这类二进制资源用。 */
export function bufferToDataUrl(buffer: ArrayBuffer, mime: string): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let index = 0; index < bytes.length; index += 1) {
    binary += String.fromCharCode(bytes[index]);
  }
  return `data:${mime};base64,${btoa(binary)}`;
}

/** data URL → ArrayBuffer；失败返回 null。 */
export function dataUrlToBuffer(dataUrl: string): ArrayBuffer | null {
  try {
    const [, payload] = dataUrl.split(",");
    if (!payload) return null;
    const binary = atob(payload);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    return bytes.buffer;
  } catch {
    return null;
  }
}
