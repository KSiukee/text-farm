import { describe, expect, it } from "vitest";
import { formatGrowMinutes, formatRemain, plotLine } from "./format";
import type { CropView, PlotView } from "./types";

const emptyPlot: PlotView = {
  index: 1,
  label: "土坑1",
  unlocked: true,
  kind: "field",
  unlockLevel: 1,
  unlockCost: 0,
  tier: 0,
  tierName: "普通",
  effects: { expPct: 0, coinPct: 0, yieldPct: 0, luckPct: 0 },
  upgradeCost: 1_000,
  upgradeLevel: 5,
  maxTier: false,
  canUpgrade: false,
  crop: null,
};

const growing: CropView = {
  cropId: "xiaobaicai",
  name: "小青菜",
  icon: "🥬",
  status: "growing",
  readyAt: 65_000,
  season: 1,
  seasons: 2,
  remaining: 2,
  watered: false,
  canWater: true,
  canHarvest: false,
  pest: null,
  canClearPest: false,
  disaster: null,
  canMitigate: false,
  offSeason: false,
  greenhouse: false,
  withered: false,
  regionLabel: "通用",
};

const ready: CropView = {
  ...growing,
  status: "ready",
  readyAt: 0,
  remaining: 2,
  canWater: false,
  canHarvest: true,
};

describe("format", () => {
  it("formats countdowns", () => {
    expect(formatRemain(0)).toBe("0:00");
    expect(formatRemain(65_000)).toBe("1:05");
    expect(formatRemain(3_661_000)).toBe("1:01:01");
  });

  it("formats grow minutes", () => {
    expect(formatGrowMinutes(3)).toBe("3分");
    expect(formatGrowMinutes(120)).toBe("2时");
    expect(formatGrowMinutes(90)).toBe("1时30分");
  });

  it("keeps the reference screenshot grammar", () => {
    expect(plotLine(emptyPlot, 0)).toBe("土坑1[普通]:空地 [播种]");
    expect(
      plotLine({ ...emptyPlot, unlocked: false, unlockLevel: 3 }, 0),
    ).toBe("[土坑1]3级可开垦.开垦");
    expect(plotLine({ ...emptyPlot, crop: growing }, 1_000)).toBe(
      "土坑1[普通]:小青菜 (生长中 1:04) (1/2季) [浇水]",
    );
    expect(plotLine({ ...emptyPlot, crop: ready }, 0)).toBe(
      "土坑1[普通]:小青菜 (产2剩2) (1/2季) [收获] [收割机收获]",
    );
  });

  it("marks watered crops", () => {
    expect(
      plotLine({ ...emptyPlot, crop: { ...growing, watered: true } }, 1_000),
    ).toContain("(已浇水, 生长中 1:04)");
  });
});
