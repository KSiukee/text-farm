/**
 * 自定义字体文件存储（#79）：只存本机 IndexedDB，不写存档、不碰路径。
 * 上限 30MB；只接受 .ttf/.otf/.woff/.woff2。
 */
import { uiDatabase, UI_STORE_NAME } from "./ui-store";

const FONT_KEY = "font-file";
const EXTENSIONS = [".ttf", ".otf", ".woff", ".woff2"];

export const MAX_FONT_FILE_BYTES = 30 * 1024 * 1024;
export const FONT_FILE_ACCEPT = ".ttf,.otf,.woff,.woff2";

export interface StoredFont {
  name: string;
  data: ArrayBuffer;
}

export class FontFileError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FontFileError";
  }
}

function displayName(file: File): string {
  const base = file.name.replace(/\.[^.]+$/, "").replace(/\s+/g, " ").trim();
  return (base || "自定义字体").slice(0, 40);
}

async function database(): Promise<IDBDatabase> {
  try {
    return await uiDatabase();
  } catch {
    throw new FontFileError("本地字体库打不开。");
  }
}

export async function saveCustomFont(file: File): Promise<StoredFont> {
  const extension = file.name.slice(file.name.lastIndexOf(".")).toLowerCase();
  if (!EXTENSIONS.includes(extension)) {
    throw new FontFileError("只支持 .ttf / .otf / .woff / .woff2 字体文件。");
  }
  if (!file.size) throw new FontFileError("这个字体文件是空的。");
  if (file.size > MAX_FONT_FILE_BYTES) {
    throw new FontFileError("字体文件超过 30MB，先换个小一点的（或转成 WOFF2）。");
  }
  const data = await file.arrayBuffer();
  if (!data.byteLength) throw new FontFileError("这个字体文件读不出内容。");
  const stored: StoredFont = { name: displayName(file), data };
  const db = await database();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(UI_STORE_NAME, "readwrite");
    tx.objectStore(UI_STORE_NAME).put(stored, FONT_KEY);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(new FontFileError("字体文件保存失败。"));
    tx.onabort = () => reject(new FontFileError("字体文件保存被中止。"));
  });
  return stored;
}

/** 读不到/库坏了都回 null：字体只是显示层，不该拦住启动。 */
export async function loadCustomFont(): Promise<StoredFont | null> {
  try {
    const db = await database();
    return await new Promise<StoredFont | null>((resolve, reject) => {
      const tx = db.transaction(UI_STORE_NAME, "readonly");
      const request = tx.objectStore(UI_STORE_NAME).get(FONT_KEY);
      request.onsuccess = () => {
        const value = request.result as StoredFont | undefined;
        if (
          value &&
          typeof value.name === "string" &&
          value.data instanceof ArrayBuffer &&
          value.data.byteLength > 0
        ) {
          resolve(value);
        } else {
          resolve(null);
        }
      };
      request.onerror = () => reject(new FontFileError("字体文件读取失败。"));
    });
  } catch {
    return null;
  }
}

export async function deleteCustomFont(): Promise<void> {
  const db = await database();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(UI_STORE_NAME, "readwrite");
    tx.objectStore(UI_STORE_NAME).delete(FONT_KEY);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(new FontFileError("字体文件删除失败。"));
    tx.onabort = () => reject(new FontFileError("字体文件删除被中止。"));
  });
}

/** 把二进制包成 FontFace 并挂到 document.fonts；失败抛 FontFileError。 */
export async function createCustomFontFace(
  data: ArrayBuffer,
  family: string,
): Promise<FontFace> {
  try {
    const face = new FontFace(family, data, { display: "swap" });
    await face.load();
    document.fonts.add(face);
    return face;
  } catch {
    throw new FontFileError("字体加载失败，可能文件损坏或格式不支持。");
  }
}
