/// 资源路径统一出口。
///
/// 页面在 GitHub Pages 的项目站上会挂在 `/<仓库名>/` 下，写死的 `/pets/...`
/// 会跑到域名根上去（404）。所有绝对资源路径都过这里，自动带上构建时的 base。
export function asset(path: string): string {
  if (!path.startsWith("/")) return path;
  const base = import.meta.env.BASE_URL;
  const trimmed = base.endsWith("/") ? base.slice(0, -1) : base;
  return `${trimmed}${path}`;
}
