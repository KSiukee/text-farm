/// 浏览器端的键值存储：存档、玩家资源（头像/背景/字体）都走这一个 IndexedDB。
///
/// 桌面/安卓壳里这些东西是文件，浏览器里没有文件系统，就只有这一种"本机持久化"。
/// 注意：origin 变了（换域名/换端口）就等于换了一个库，这也是网页版会"丢档"的根因之一。

const DB_NAME = "text-farm-web";
const STORE = "kv";

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore(STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("打不开 IndexedDB"));
  });
}

export async function idbGet(key: string): Promise<string | null> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readonly");
    const req = tx.objectStore(STORE).get(key);
    req.onsuccess = () => resolve(typeof req.result === "string" ? req.result : null);
    req.onerror = () => reject(req.error ?? new Error("读 IndexedDB 失败"));
  });
}

export async function idbSet(key: string, value: string): Promise<void> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).put(value, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error ?? new Error("写 IndexedDB 失败"));
  });
}

export async function idbDel(key: string): Promise<void> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).delete(key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error ?? new Error("删 IndexedDB 失败"));
  });
}
