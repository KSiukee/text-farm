/**
 * 玩家自定义资源：头像 / 背景 / 字体的本地存储。
 *
 * 桌面与安卓落在应用数据目录的 custom/ 下（走 Tauri 命令）。
 * 浏览器没有文件系统，改存 IndexedDB —— 同一个库、同一个 `kv` 表，
 * 键是 `asset:<kind>:<name>`，玩家资料是 `player-assets`。
 */
import { invoke } from "@tauri-apps/api/core";
import { isWeb } from "./web-backend";
import { idbDel, idbGet, idbSet } from "./web-storage";

export type CustomAssetKind = "avatars" | "backgrounds" | "fonts";

export interface PlayerAssets {
  avatar: string;
}

const PROFILE_KEY = "player-assets";

function assetKey(kind: CustomAssetKind, name: string): string {
  return `asset:${kind}:${name}`;
}

export async function customAssetsDir(): Promise<string> {
  if (isWeb) return "";
  return invoke<string>("custom_assets_dir");
}

export async function saveCustomAsset(
  kind: CustomAssetKind,
  name: string,
  dataUrl: string,
): Promise<string> {
  if (isWeb) {
    await idbSet(assetKey(kind, name), dataUrl);
    return `browser:${kind}/${name}`;
  }
  return invoke<string>("save_custom_asset", { kind, name, dataUrl });
}

export async function readCustomAsset(
  kind: CustomAssetKind,
  name: string,
): Promise<string | null> {
  if (isWeb) return idbGet(assetKey(kind, name));
  return invoke<string | null>("read_custom_asset", { kind, name });
}

export async function deleteCustomAsset(
  kind: CustomAssetKind,
  name: string,
): Promise<void> {
  if (isWeb) {
    await idbDel(assetKey(kind, name));
    return;
  }
  return invoke<void>("delete_custom_asset", { kind, name });
}

export async function loadPlayerAssets(): Promise<PlayerAssets> {
  if (isWeb) {
    const raw = await idbGet(PROFILE_KEY);
    if (!raw) return { avatar: "" };
    try {
      const parsed = JSON.parse(raw) as Partial<PlayerAssets>;
      return { avatar: typeof parsed.avatar === "string" ? parsed.avatar : "" };
    } catch {
      return { avatar: "" };
    }
  }
  return invoke<PlayerAssets>("load_player_assets");
}

export async function savePlayerAssets(assets: PlayerAssets): Promise<void> {
  if (isWeb) {
    await idbSet(PROFILE_KEY, JSON.stringify(assets));
    return;
  }
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
