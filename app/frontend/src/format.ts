import type { CropView, PlotView } from "./types";

export function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

export function formatRemain(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${h}:${pad2(m)}:${pad2(s)}` : `${m}:${pad2(s)}`;
}

export function formatStamp(ms: number): string {
  const d = new Date(ms);
  return `${pad2(d.getMonth() + 1)}.${pad2(d.getDate())} ${pad2(d.getHours())}:${pad2(
    d.getMinutes(),
  )}`;
}

export function formatGrowMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes}分`;
  if (minutes % 60 === 0) return `${minutes / 60}时`;
  return `${Math.floor(minutes / 60)}时${minutes % 60}分`;
}

function seasonText(crop: CropView): string {
  return crop.seasons > 1 ? ` (${crop.season}/${crop.seasons}季)` : "";
}

/** 参考图的逐字格式；测试锁这一行。 */
export function plotLine(plot: PlotView, now: number): string {
  if (!plot.unlocked) {
    return `[${plot.label}]${plot.unlockLevel}级可开垦.开垦`;
  }
  if (!plot.crop) {
    return `${plot.label}[普通]:空地 [播种]`;
  }
  const crop = plot.crop;
  const season = seasonText(crop);
  if (crop.status === "growing") {
    const watered = crop.watered ? "已浇水, " : "";
    return `${plot.label}[普通]:${crop.name} (${watered}生长中 ${formatRemain(
      crop.readyAt - now,
    )})${season} [浇水]`;
  }
  return `${plot.label}[普通]:${crop.name} (产${crop.remaining}剩${crop.remaining})${season} [收获] [收割机收获]`;
}
