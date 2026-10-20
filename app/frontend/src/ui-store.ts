/**
 * 前端本机 UI 存储（IndexedDB）：背景图和自定义字体共用同一个库。
 * 只存二进制/字符串，不碰存档、不碰文件路径。
 */

export const UI_DB_NAME = "text-farm-ui";
export const UI_DB_VERSION = 1;
export const UI_STORE_NAME = "kv";

let dbPromise: Promise<IDBDatabase> | null = null;

export function uiDatabase(): Promise<IDBDatabase> {
  dbPromise ??= openDatabase().catch((error: unknown) => {
    dbPromise = null;
    throw error;
  });
  return dbPromise;
}

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === "undefined") {
      reject(new Error("这台机器不支持 IndexedDB。"));
      return;
    }
    const request = indexedDB.open(UI_DB_NAME, UI_DB_VERSION);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(UI_STORE_NAME)) {
        database.createObjectStore(UI_STORE_NAME);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(new Error("本地 UI 存储打不开。"));
    request.onblocked = () => reject(new Error("本地 UI 存储被占用，关掉旧窗口再试。"));
  });
}
