// 文字农场 · 网页版 Service Worker
//
// 目标只有一个：**加不到主屏幕、断网也能开**，顺带让第二次打开快一点。
// 不做预缓存清单——hashed 资源见过就缓存，导航请求走「先网络、失败回缓存」，
// 这样发新版本时用户下次联网打开就是新的（旧的 hash 文件自然不会再被引用）。

const CACHE = "text-farm-v1";

self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key)));
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") return;

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  // 导航：先网络（拿最新 HTML），断网时回缓存，最后兜底到站点根
  if (request.mode === "navigate") {
    event.respondWith(
      (async () => {
        try {
          const response = await fetch(request);
          const cache = await caches.open(CACHE);
          cache.put(request, response.clone());
          return response;
        } catch {
          const cache = await caches.open(CACHE);
          const cached = await cache.match(request);
          if (cached) return cached;
          const scope = self.registration.scope;
          const root = await cache.match(scope) || (await cache.match(scope + "index.html"));
          if (root) return root;
          return new Response("离线中，先联网打开一次再试。", {
            status: 503,
            headers: { "Content-Type": "text/plain; charset=utf-8" },
          });
        }
      })(),
    );
    return;
  }

  // 静态资源：缓存优先（文件名带 hash，内容不会变）
  event.respondWith(
    (async () => {
      const cached = await caches.match(request);
      if (cached) return cached;
      const response = await fetch(request);
      if (response.ok) {
        const cache = await caches.open(CACHE);
        cache.put(request, response.clone());
      }
      return response;
    })(),
  );
});
