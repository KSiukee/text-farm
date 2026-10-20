/**
 * 背景图存储（#76）：浏览器里压到最长边 1920，再存 IndexedDB。
 * 不写存档、不碰文件路径；App 只拿 data URL。
 */
import { uiDatabase, UI_STORE_NAME } from "./ui-store";

const IMAGE_KEY = "background-image";

export const MAX_BACKGROUND_EDGE = 1920;
export const MAX_BACKGROUND_FILE_BYTES = 20 * 1024 * 1024;

export class BackgroundImageError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BackgroundImageError";
  }
}

function loadImage(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const image = new Image();
    image.onload = () => {
      URL.revokeObjectURL(url);
      resolve(image);
    };
    image.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new BackgroundImageError("这张图片读不出来，换一张试试。"));
    };
    image.src = url;
  });
}

function encode(canvas: HTMLCanvasElement, type: string, quality: number): string {
  try {
    const dataUrl = canvas.toDataURL(type, quality);
    return dataUrl.startsWith(`data:${type}`) ? dataUrl : "";
  } catch {
    return "";
  }
}

/**
 * 读文件 → 缩到最长边 1920 → WebP（不支持就 JPEG）→ data URL。
 * 超过 20MB 的文件先拒绝，避免一次解码把界面卡死。
 */
export async function compressImage(
  file: File,
  maxEdge: number = MAX_BACKGROUND_EDGE,
): Promise<string> {
  if (!file.type.startsWith("image/")) {
    throw new BackgroundImageError("请选一张图片（PNG/JPG/WebP 都行）。");
  }
  if (file.size > MAX_BACKGROUND_FILE_BYTES) {
    throw new BackgroundImageError("图片超过 20MB，先裁小一点再选。");
  }

  const image = await loadImage(file);
  const sourceWidth = image.naturalWidth;
  const sourceHeight = image.naturalHeight;
  if (!sourceWidth || !sourceHeight) {
    throw new BackgroundImageError("这张图片没有有效的尺寸。");
  }

  const scale = Math.min(1, maxEdge / Math.max(sourceWidth, sourceHeight));
  let width = Math.max(1, Math.round(sourceWidth * scale));
  let height = Math.max(1, Math.round(sourceHeight * scale));

  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (!context) throw new BackgroundImageError("这台机器画不了图（canvas 不可用）。");

  const draw = () => {
    canvas.width = width;
    canvas.height = height;
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = "high";
    context.drawImage(image, 0, 0, width, height);
  };

  draw();
  let dataUrl = encode(canvas, "image/webp", 0.82);
  if (!dataUrl) dataUrl = encode(canvas, "image/jpeg", 0.82);
  if (!dataUrl) throw new BackgroundImageError("图片编码失败，换一张试试。");

  // data URL 太大就降一档；再大就缩到 1280。
  if (dataUrl.length > 2_500_000) dataUrl = encode(canvas, "image/jpeg", 0.72) || dataUrl;
  if (dataUrl.length > 3_500_000) {
    const shrink = Math.min(1, 1280 / Math.max(width, height));
    width = Math.max(1, Math.round(width * shrink));
    height = Math.max(1, Math.round(height * shrink));
    draw();
    dataUrl = encode(canvas, "image/webp", 0.78) || encode(canvas, "image/jpeg", 0.7) || dataUrl;
  }
  return dataUrl;
}

async function database(): Promise<IDBDatabase> {
  try {
    return await uiDatabase();
  } catch {
    throw new BackgroundImageError("本地图片库打不开。");
  }
}

export async function saveBackgroundImage(dataUrl: string): Promise<void> {
  const db = await database();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(UI_STORE_NAME, "readwrite");
    tx.objectStore(UI_STORE_NAME).put(dataUrl, IMAGE_KEY);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(new BackgroundImageError("背景图保存失败。"));
    tx.onabort = () => reject(new BackgroundImageError("背景图保存被中止。"));
  });
}

/** 读不到/库坏了都回 null：背景图只是装饰，不该拦住启动。 */
export async function loadBackgroundImage(): Promise<string | null> {
  try {
    const db = await database();
    return await new Promise<string | null>((resolve, reject) => {
      const tx = db.transaction(UI_STORE_NAME, "readonly");
      const request = tx.objectStore(UI_STORE_NAME).get(IMAGE_KEY);
      request.onsuccess = () =>
        resolve(typeof request.result === "string" ? request.result : null);
      request.onerror = () => reject(new BackgroundImageError("背景图读取失败。"));
    });
  } catch {
    return null;
  }
}

export async function deleteBackgroundImage(): Promise<void> {
  const db = await database();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(UI_STORE_NAME, "readwrite");
    tx.objectStore(UI_STORE_NAME).delete(IMAGE_KEY);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(new BackgroundImageError("背景图删除失败。"));
    tx.onabort = () => reject(new BackgroundImageError("背景图删除被中止。"));
  });
}
