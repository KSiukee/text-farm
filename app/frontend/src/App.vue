<script setup lang="ts">
import { open as openFileDialog, save as saveFileDialog } from "@tauri-apps/plugin-dialog";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  advanceTime,
  applyContent,
  dispatch,
  errorText,
  exportContent,
  exportSave,
  getAppInfo,
  getContent,
  getSnapshot,
  importContent,
  importSave,
  previewContent,
  reviewSeeds,
  saveNow,
  sync as syncGame,
} from "./api";
import { formatGrowMinutes, formatRemain, formatStamp } from "./format";
import type {
  Action,
  AppInfo,
  AutoReportView,
  BaitQuality,
  BaitRecipe,
  BaitType,
  ContentPack,
  CropView,
  CropDef,
  DisciplineKind,
  EquipmentKind,
  EquipmentRecipe,
  FacilityView,
  FishCatchView,
  FishCodexView,
  FishRecipe,
  FishingSpot,
  ItemStack,
  NpcView,
  PetBreed,
  PlotView,
  RanchAnimalKind,
  RanchRecipe,
  Region,
  RodKind,
  Season,
  Snapshot,
  SpecimenView,
} from "./types";
import FishingGame from "./FishingGame.vue";
import {
  applyTheme,
  DEFAULT_PREFS,
  PALETTES,
  readThemePrefs,
  THEME_MODES,
  writeThemePrefs,
  type PaletteId,
  type ThemeMode,
} from "./theme";

const snapshot = ref<Snapshot | null>(null);
const content = ref<ContentPack | null>(null);
const info = ref<AppInfo | null>(null);
const error = ref("");
const notice = ref("");
const saveMessage = ref("");
const loading = ref(false);
const acting = ref(false);
const tick = ref(Date.now());
const editor = ref<ContentPack | null>(null);
const editorIssues = ref<string[]>([]);
const editorMessage = ref("");
const seedCount = ref(24);
const seedValue = ref(20261018);
const seedReviewNotes = ref<string[]>([]);
const activeTab = ref("我的农场");
const debugTime = ref(window.localStorage.getItem("farm.debugTime") === "1");
const initialTheme = readThemePrefs(window.localStorage);
const themeMode = ref<ThemeMode>(initialTheme.mode);
const paletteId = ref<PaletteId>(initialTheme.palette);
let themeMedia: MediaQueryList | null = null;

let clockOffset = 0;
let timer: number | undefined;
let refreshTimer: number | undefined;
let syncTimer: number | undefined;

const topIcons = [
  { icon: "🧑‍🌾", name: "农场" },
  { icon: "🐄", name: "畜牧场" },
  { icon: "🏡", name: "牧村" },
  { icon: "🏭", name: "制造屋" },
  { icon: "⛏️", name: "矿场" },
  { icon: "🎣", name: "钓鱼" },
];

const tabs = ["我的农场", "邻居农场", "宠物", "标本室", "神秘商店", "图鉴", "神殿", "产业园", "种子精华", "玩家"];

const levelLine = computed(() => {
  const s = snapshot.value;
  return s ? `农场等级:${s.level}(${s.exp}/${s.expNeed})` : "农场等级:--";
});

const currency = computed(() => content.value?.nouns.currency ?? "金币");

const produce = computed(() =>
  (snapshot.value?.inventory ?? []).filter((item) => item.kind === "produce"),
);

const ranch = computed(() => snapshot.value?.ranch ?? null);
const fishing = computed(() => snapshot.value?.fishing ?? null);
const disciplines = computed(() => snapshot.value?.skills?.disciplines ?? []);
const buffList = computed(() => snapshot.value?.skills?.buffs ?? []);
const buffItems = computed(() => snapshot.value?.skills?.buffItems ?? []);
const ownedBuffItems = computed(() =>
  (snapshot.value?.inventory ?? []).flatMap((item) => {
    const def = buffItems.value.find(
      (buff) => item.key === buff.key || item.key.startsWith(`${buff.key}.`),
    );
    return def && item.qty > 0 ? [{ ...def, key: item.key, qty: item.qty }] : [];
  }),
);
const fishingSpot = ref<FishingSpot>("creek");
const catchResult = ref<FishCatchView | null>(null);
const selectedFish = ref<FishCodexView | null>(null);
const autoReportVisible = ref(false);
const autoReportShownAt = ref(
  Number(window.localStorage.getItem("farm.autoReportShownAt") || 0),
);
const autoSpot = ref<FishingSpot>("creek");
const autoRod = ref<RodKind>("wood");
const autoBaitQuality = ref<BaitQuality>("normal");
const activeBaitQuality = ref<BaitQuality>("normal");
const activeBaitType = ref<BaitType>("dough");
const autoBaitType = ref<BaitType>("dough");
let catchTimer: number | undefined;

function dismissCatch() {
  catchResult.value = null;
  if (catchTimer !== undefined) window.clearTimeout(catchTimer);
  catchTimer = undefined;
}

function dismissAutoReport() {
  autoReportVisible.value = false;
}

/// 自动钓鱼报告只在启动和进入钓鱼页时露一次；新报告才再弹，避免每条鱼都打断。
function maybeShowAutoReport() {
  const report = fishing.value?.lastAutoReport;
  if (!report || report.totalCaught <= 0) return;
  if (report.at <= autoReportShownAt.value) return;
  autoReportShownAt.value = report.at;
  window.localStorage.setItem("farm.autoReportShownAt", String(report.at));
  autoReportVisible.value = true;
}

watch(
  () => snapshot.value?.fishing.lastCatch?.at ?? 0,
  () => {
    const current = snapshot.value?.fishing.lastCatch;
    if (!current) return;
    if (Math.abs(Date.now() - current.at) > 20_000) return;
    catchResult.value = current;
    if (catchTimer !== undefined) window.clearTimeout(catchTimer);
    catchTimer = window.setTimeout(() => {
      catchResult.value = null;
    }, 4_200);
  },
);

watch(
  () => fishing.value?.activeBait,
  (quality) => {
    if (quality) activeBaitQuality.value = quality;
  },
  { immediate: true },
);

watch(
  () => fishing.value?.activeBaitType,
  (baitType) => {
    if (baitType) activeBaitType.value = baitType;
  },
  { immediate: true },
);

watch(
  () =>
    `${fishing.value?.auto.spot ?? ""}|${fishing.value?.auto.rod ?? ""}|${
      fishing.value?.auto.baitQuality ?? ""
    }|${fishing.value?.auto.baitType ?? ""}`,
  () => {
    const current = fishing.value?.auto;
    if (!current) return;
    autoSpot.value = current.spot;
    autoRod.value = current.rod;
    autoBaitQuality.value = current.baitQuality;
    autoBaitType.value = current.baitType;
  },
  { immediate: true },
);
const workshop = computed(() => snapshot.value?.workshop ?? null);
const emptyEquipmentSlot = computed(
  () =>
    workshop.value?.slotViews.find((slot) => slot.index > 0 && !slot.equipment)?.index ??
    null,
);
const ranchProducts = computed(() =>
  (snapshot.value?.inventory ?? []).filter((item) => item.kind === "ranch" && item.sellable),
);
const workshopItems = computed(() =>
  (snapshot.value?.inventory ?? []).filter(
    (item) => item.kind === "ranch" || item.kind === "craft",
  ),
);


const seedStacks = computed(() =>
  (snapshot.value?.inventory ?? []).filter((item) => item.kind === "seed"),
);

const fishItems = computed(() =>
  (snapshot.value?.inventory ?? []).filter((item) => item.kind === "fish"),
);
const currentSpot = computed(
  () => fishing.value?.spots.find((spot) => spot.spot === fishingSpot.value) ?? null,
);
const autoSpotInfo = computed(
  () => fishing.value?.spots.find((spot) => spot.spot === autoSpot.value) ?? null,
);
const autoBaitInfo = computed(
  () => fishing.value?.baitCounts.find((bait) => bait.quality === autoBaitQuality.value) ?? null,
);
const activeBaitInfo = computed(
  () => fishing.value?.baitCounts.find((bait) => bait.quality === activeBaitQuality.value) ?? null,
);
const kingCaught = computed(
  () => fishing.value?.kingTrophies.filter((king) => king.caught).length ?? 0,
);
const fishingExpPct = computed(() => {
  const current = fishing.value;
  if (!current || !current.expNeed) return 100;
  return Math.min(100, Math.round((current.exp / current.expNeed) * 100));
});
const codexPct = computed(() => {
  const current = fishing.value;
  if (!current || !current.codexTotal) return 0;
  return Math.round((current.codexCaught / current.codexTotal) * 100);
});
const activeBaitTypeLabel = computed(
  () =>
    fishing.value?.baitTypes.find((type) => type.baitType === activeBaitType.value)?.label ??
    "面团",
);
const pets = computed(() => snapshot.value?.pets ?? []);
const activePets = computed(() => pets.value.filter((item) => !item.boarded));
const boardedPets = computed(() => pets.value.filter((item) => item.boarded));
const selectedPetId = ref("");
const selectedPet = computed(
  () =>
    pets.value.find((item) => item.id === selectedPetId.value) ??
    activePets.value[0] ??
    pets.value[0] ??
    null,
);
const pet = computed(() => selectedPet.value ?? snapshot.value?.pet ?? null);
const petCare = computed(() => snapshot.value?.petCare ?? null);
const boardingTotal = computed(() => boardedPets.value.length * 3_000);
const nextBoardingMonth = computed(() => {
  const date = new Date();
  date.setMonth(date.getMonth() + 1);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
});
const showAdopt = ref(false);
const adoptBoard = ref(false);
const petName = ref("豆豆");
const petBreed = ref<PetBreed>("corgi");
const BREEDS: { id: PetBreed; label: string; species: "dog" | "cat"; image: string }[] = [
  { id: "corgi", label: "柯基", species: "dog", image: "/pets/dog.png" },
  { id: "german_shepherd", label: "德国牧羊犬", species: "dog", image: "/pets/german_shepherd.png" },
  { id: "golden_retriever", label: "金毛", species: "dog", image: "/pets/golden_retriever.png" },
  { id: "border_collie", label: "边牧", species: "dog", image: "/pets/border_collie.png" },
  { id: "shiba", label: "柴犬", species: "dog", image: "/pets/shiba.png" },
  { id: "rural_dog", label: "田园犬", species: "dog", image: "/pets/rural_dog.png" },
  { id: "bichon", label: "比熊", species: "dog", image: "/pets/bichon.png" },
  { id: "domestic_cat", label: "田园猫", species: "cat", image: "/pets/cat.png" },
  { id: "ragdoll", label: "布偶猫", species: "cat", image: "/pets/ragdoll.png" },
  { id: "golden_shaded", label: "金渐层", species: "cat", image: "/pets/golden_shaded.png" },
  { id: "silver_shaded", label: "银渐层", species: "cat", image: "/pets/silver_shaded.png" },
  { id: "blue_cat", label: "蓝猫", species: "cat", image: "/pets/blue_cat.png" },
  { id: "coffee_cat", label: "咖啡猫", species: "cat", image: "/pets/coffee_cat.png" },
];
const petSpecies = computed(
  () => BREEDS.find((breed) => breed.id === petBreed.value)?.species ?? "dog",
);
const visibleBreeds = computed(() => BREEDS.filter((breed) => breed.species === petSpecies.value));
const selectedBreed = computed(() => BREEDS.find((breed) => breed.id === petBreed.value) ?? BREEDS[0]);
const feedCropId = ref("");
const renameName = ref("");

const anyReady = computed(
  () => snapshot.value?.plots.some((plot) => Boolean(plot.crop?.canHarvest)) ?? false,
);

const hasHarvester = computed(
  () =>
    snapshot.value?.facilities.find((facility) => facility.kind === "harvester")?.owned ??
    false,
);

const logLines = computed(() =>
  [...(snapshot.value?.log ?? [])]
    .reverse()
    .slice(0, 10)
    .map((entry) => ({
      ...entry,
      stamp: formatStamp(entry.at),
    })),
);

const achievements = computed(() => snapshot.value?.achievements ?? []);
const codex = computed(() => snapshot.value?.codex ?? []);
const daily = computed(() => snapshot.value?.daily ?? null);
const levelRewards = computed(() => snapshot.value?.levelRewards ?? []);
const levelRewardClaimable = computed(() => snapshot.value?.levelRewardsClaimable ?? 0);
const npcs = computed(() => snapshot.value?.npcs ?? []);
const specimens = computed(() => snapshot.value?.specimens ?? []);
const seasonClass = computed(() => `season-${snapshot.value?.season ?? "spring"}`);
const SEASON_NAMES: Record<Season, string> = {
  spring: "春",
  summer: "夏",
  autumn: "秋",
  winter: "冬",
};
const SEASONS: Season[] = ["spring", "summer", "autumn", "winter"];
const npcMessages = computed(() =>
  [...(snapshot.value?.npcMessages ?? [])].reverse(),
);

function selectTab(tab: string) {
  if (
    tab === "我的农场" ||
    tab === "邻居农场" ||
    tab === "宠物" ||
    tab === "标本室" ||
    tab === "畜牧场" ||
    tab === "制造屋" ||
    tab === "钓鱼" ||
    tab === "玩家"
  ) {
    activeTab.value = tab;
    if (tab === "钓鱼") maybeShowAutoReport();
  } else {
    placeholder(tab);
  }
}

function openTop(name: string) {
  if (name === "畜牧场" || name === "制造屋" || name === "钓鱼") {
    selectTab(name);
  } else {
    placeholder(name);
  }
}

function isInSeason(crop: CropDef): boolean {
  const season = snapshot.value?.season ?? "spring";
  return crop.sowSeasons.length === 0 || crop.sowSeasons.includes(season);
}

function affinityLabel(region: CropDef["region"]): string {
  return region === "north" ? "北方" : region === "south" ? "南方" : "通用";
}

function resistanceText(crop: CropDef): string {
  const r = crop.resistances;
  const parts = [];
  if (r.drought) parts.push(`旱${r.drought}%`);
  if (r.flood) parts.push(`涝${r.flood}%`);
  if (r.cold) parts.push(`寒${r.cold}%`);
  if (r.pest) parts.push(`虫${r.pest}%`);
  return parts.length ? `抗灾 ${parts.join(" ")}` : "无抗性";
}

function toggleSeason(crop: CropDef, season: Season) {
  const index = crop.sowSeasons.indexOf(season);
  if (index >= 0) {
    crop.sowSeasons.splice(index, 1);
  } else {
    crop.sowSeasons.push(season);
  }
}

function setRegion(region: Region) {
  void act({ type: "setRegion", region });
}

function craftSpecimen(specimen: SpecimenView) {
  void act({ type: "craftSpecimen", cropId: specimen.cropId });
}

function formatCooldown(ms: number): string {
  const minutes = Math.ceil(ms / 60000);
  if (minutes <= 0) return "好了";
  if (minutes < 60) return `${minutes} 分钟`;
  return `${Math.floor(minutes / 60)} 小时 ${minutes % 60} 分`;
}

function adoptPet() {
  void act({
    type: "adoptPet",
    name: petName.value,
    breed: petBreed.value,
    board: adoptBoard.value || activePets.value.length >= 3,
  });
  showAdopt.value = false;
}

function feedPet() {
  const target = selectedPet.value;
  const cropId = feedCropId.value || produce.value[0]?.key.replace("crop:", "");
  if (target && cropId) {
    void act({ type: "feedPet", petId: target.id, cropId });
  }
}

function petAction(action: "petCompanion" | "playWithPet" | "healPet" | "digWithPet") {
  if (selectedPet.value) {
    void act({ type: action, petId: selectedPet.value.id });
  }
}

function usePetSkill() {
  if (selectedPet.value) {
    void act({ type: "usePetSkill", petId: selectedPet.value.id });
  }
}

function renamePet() {
  if (selectedPet.value && renameName.value.trim()) {
    void act({ type: "renamePet", petId: selectedPet.value.id, name: renameName.value.trim() });
  }
}

function setPetBoarded(petId: string, boarded: boolean) {
  void act({ type: "setPetBoarded", petId, boarded });
}

function releasePet(petId: string, name: string) {
  if (!window.confirm(`确定放生「${name}」吗？这一步不可撤销。`)) return;
  void act({ type: "releasePet", petId });
}

function buyPetSupply(kind: "dog_bed" | "cat_bed" | "bowl" | "dog_toy" | "cat_toy" | "medicine") {
  void act({ type: "buyPetSupply", kind });
}

function repairPetSupply(kind: "dog_bed" | "cat_bed" | "dog_toy" | "cat_toy") {
  void act({ type: "repairPetSupply", kind });
}

function trainPet(kind: "guard" | "dig" | "mischief" | "companion") {
  if (selectedPet.value) {
    void act({ type: "trainPet", petId: selectedPet.value.id, kind });
  }
}

function usePetMedicine() {
  if (selectedPet.value) {
    void act({ type: "usePetMedicine", petId: selectedPet.value.id });
  }
}

function settleBoardingDebt() {
  void act({ type: "settleBoardingDebt" });
}

function helpNpc(npc: NpcView, plotIndex: number) {
  void act({ type: "helpNpc", npcId: npc.id, plot: plotIndex });
}

function stealNpc(npc: NpcView, plotIndex: number) {
  void act({ type: "stealNpc", npcId: npc.id, plot: plotIndex });
}

function helpNpcRanch(npc: NpcView, index: number) {
  void act({ type: "helpNpcRanch", npcId: npc.id, index });
}

function stealNpcRanch(npc: NpcView, index: number) {
  void act({ type: "stealNpcRanch", npcId: npc.id, index });
}

function greetNpc(npc: NpcView) {
  void act({ type: "npcGreeting", npcId: npc.id });
}

function cropById(id: string): CropDef | undefined {
  return content.value?.crops.find((crop) => crop.id === id);
}

function cropName(plot: PlotView): string {
  return plot.crop?.name ?? "";
}

function cropStatusText(plot: PlotView): string {
  const crop = plot.crop;
  if (!crop) return "";
  if (crop.withered) return "枯萎，无法成熟";
  if (crop.disaster) {
    const disaster = crop.disaster;
    const growth =
      disaster.growthSlow >= 100 ? "·生长暂停" : disaster.growthSlow > 0 ? "·生长减半" : "";
    const remain =
      disaster.kind === "hail" ? "" : ` ${formatRemain(disaster.endsAt - tick.value)}`;
    return `${disaster.label}${growth}${remain}`;
  }
  if (crop.pest) {
    return crop.pest.kind === "bug" ? "有虫, 生长变慢" : "有草, 生长变慢";
  }
  if (crop.status === "growing") {
    const watered = crop.watered ? "已浇水, " : "";
    return `${watered}生长中 ${formatRemain(crop.readyAt - tick.value)}`;
  }
  return `产${crop.remaining}剩${crop.remaining}`;
}

function cropSeasonText(plot: PlotView): string {
  const crop = plot.crop;
  return crop && crop.seasons > 1 ? ` (${crop.season}/${crop.seasons}季)` : "";
}

function canWater(plot: PlotView): boolean {
  return Boolean(plot.crop?.canWater);
}

function canHarvest(plot: PlotView): boolean {
  return Boolean(plot.crop?.canHarvest);
}

function canClearPest(plot: PlotView): boolean {
  return Boolean(plot.crop?.canClearPest);
}

function clearPest(plot: PlotView) {
  void act({ type: "clearPest", plot: plot.index });
}

function mitigate(plot: PlotView) {
  void act({ type: "mitigateDisaster", plot: plot.index });
}

function clearCrop(plot: PlotView) {
  void act({ type: "clear", plot: plot.index });
}

function claimSignIn() {
  void act({ type: "claimSignIn" });
}

function claimDaily(id: number) {
  void act({ type: "claimDaily", id });
}

function claimLevelReward(level: number) {
  void act({ type: "claimLevelReward", level });
}

function bestSeedId(): string | null {
  const level = snapshot.value?.level ?? 1;
  let best: { id: string; unlock: number; grow: number } | null = null;
  for (const stack of seedStacks.value) {
    if (stack.qty <= 0) continue;
    const id = stack.key.replace(/^seed:/, "");
    const def = cropById(id);
    if (!def || def.unlockLevel > level) continue;
    if (
      !best ||
      def.unlockLevel > best.unlock ||
      (def.unlockLevel === best.unlock && def.growMinutes < best.grow)
    ) {
      best = { id, unlock: def.unlockLevel, grow: def.growMinutes };
    }
  }
  return best?.id ?? null;
}

async function act(action: Action) {
  if (acting.value) return;
  acting.value = true;
  notice.value = "";
  try {
    const reply = await dispatch(action);
    snapshot.value = reply.snapshot;
    const achievement = reply.effects.find((effect) => effect.kind === "achievement");
    if (achievement && achievement.kind === "achievement") {
      notice.value = `成就达成：${achievement.name}`;
    }
    const levelUp = reply.effects.find((effect) => effect.kind === "levelUp");
    if (!achievement && levelUp && levelUp.kind === "levelUp") {
      notice.value = `升级！农场等级 ${levelUp.level}`;
    }
    if (reply.saveError) {
      saveMessage.value = `已生效，但保存失败：${reply.saveError}`;
    }
  } catch (e) {
    notice.value = errorText(e);
  } finally {
    acting.value = false;
  }
}

function plant(plot: PlotView) {
  const cropId = bestSeedId();
  if (!cropId) {
    notice.value = "没有当前等级能种的种子，先在下面种子铺买。";
    return;
  }
  void act({ type: "plant", plot: plot.index, cropId });
}

function water(plot: PlotView) {
  void act({ type: "water", plot: plot.index });
}

function harvest(plot: PlotView) {
  void act({ type: "harvest", plot: plot.index });
}

function unlock(plot: PlotView) {
  void act({ type: "unlockPlot", plot: plot.index });
}

function upgrade(plot: PlotView) {
  void act({ type: "upgradePlot", plot: plot.index });
}

function landEffectsText(plot: PlotView): string {
  const effects = plot.effects;
  return `经验+${effects.expPct}% 金币+${effects.coinPct}% 收成+${effects.yieldPct}% 幸运+${effects.luckPct}%`;
}

function harvestAll() {
  void act({ type: "harvestAll" });
}

function buy(crop: CropDef) {
  void act({ type: "buySeed", cropId: crop.id, qty: 1 });
}

function buyFacility(facility: FacilityView) {
  void act({ type: "buyFacility", kind: facility.kind });
}

function repairWindmills() {
  void act({ type: "repairWindmills" });
}

function dismissGuide() {
  void act({ type: "dismissGuide" });
}

function sell(item: ItemStack, all: boolean) {
  void act({ type: "sell", key: item.key, qty: all ? item.qty : 1 });
}

function buyRanchAnimal(pen: number, kind: RanchAnimalKind) {
  void act({ type: "buyRanchAnimal", pen, kind });
}

function feedRanchAnimal(pen: number) {
  void act({ type: "feedRanchAnimal", pen });
}

function collectRanchProduct(pen: number) {
  void act({ type: "collectRanchProduct", pen });
}

function healRanchAnimal(pen: number) {
  void act({ type: "healRanchAnimal", pen });
}

function expandRanch() {
  void act({ type: "expandRanch" });
}

function buyRanchFeed() {
  void act({ type: "buyRanchFeed" });
}

function feedRanchPremium(pen: number) {
  void act({ type: "feedRanchPremium", pen });
}

function breedRanch() {
  void act({ type: "breedRanch" });
}

function buildWorkshop() {
  void act({ type: "buildWorkshop" });
}

function buyBait(qty: number) {
  void act({
    type: "buyBait",
    qty,
    quality: activeBaitQuality.value,
    baitType: activeBaitType.value,
  });
}

function setBaitType() {
  void act({ type: "setBaitType", baitType: activeBaitType.value });
}

function upgradeFishingSkill(skill: string) {
  void act({ type: "upgradeFishingSkill", skill });
}
function upgradeSkill(discipline: DisciplineKind, skill: string) {
  void act({ type: "upgradeSkill", discipline, skill });
}
function useBuffItem(key: string) {
  void act({ type: "useBuffItem", key });
}
function expPct(view: { exp: number; expNeed: number }) {
  return view.expNeed ? `${Math.min(100, Math.round((view.exp / view.expNeed) * 100))}%` : "100%";
}

function buyRod(rod: RodKind) {
  void act({ type: "buyRod", rod });
}

function castLine() {
  void act({ type: "castLine", spot: fishingSpot.value, quality: activeBaitQuality.value });
}

function resolveFishing(success: boolean) {
  void act({ type: "resolveFishing", success });
}

function setAutoFishing(enabled: boolean) {
  void act({
    type: "setAutoFishing",
    spot: autoSpot.value,
    rod: autoRod.value,
    baitQuality: autoBaitQuality.value,
    baitType: autoBaitType.value,
    enabled,
  });
}

function toggleFishLock(item: ItemStack) {
  const fishId = item.key.startsWith("fish:") ? item.key.slice(5) : item.key;
  void act({ type: "toggleFishLock", fishId });
}

function sellAllFish() {
  void act({ type: "sellAllFish" });
}

function maintainRod(rod: RodKind) {
  void act({ type: "maintainRod", rod });
}

function onFishingShortcut(event: KeyboardEvent) {
  if (event.key.toLowerCase() !== "f") return;
  if (activeTab.value !== "钓鱼") return;
  if (snapshot.value?.fishing?.session) return;
  const target = event.target as HTMLElement | null;
  if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
  event.preventDefault();
  if (
    !acting.value &&
    currentSpot.value?.unlocked &&
    (activeBaitInfo.value?.count ?? 0) > 0
  ) {
    castLine();
  }
}

function expandWorkshop() {
  void act({ type: "expandWorkshop" });
}

function installEquipment(slot: number, kind: EquipmentKind) {
  void act({ type: "installEquipment", slot, kind });
}

function startRanchCraft(recipe: RanchRecipe) {
  void act({ type: "startRanchCraft", recipe });
}

function startCropCraft(cropId: string) {
  void act({ type: "startCropCraft", cropId });
}

function startEquipmentCraft(recipe: EquipmentRecipe) {
  void act({ type: "startEquipmentCraft", recipe });
}

function startBaitCraft(recipe: BaitRecipe) {
  void act({ type: "startBaitCraft", recipe });
}

function startFishCraft(recipe: FishRecipe) {
  void act({ type: "startFishCraft", recipe });
}

function collectCraft(slot: number) {
  void act({ type: "collectCraft", slot });
}

function setPetLocation(petId: string, location: "farm" | "ranch") {
  void act({ type: "setPetLocation", petId, location });
}

function sendPetToRanch(petId: string) {
  setPetLocation(petId, "ranch");
}

function callPetHome(petId: string) {
  setPetLocation(petId, "farm");
}

function placeholder(name: string) {
  notice.value = `${name}：首版只做农场，这块排在 M4。`;
}

function fileStamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(
    d.getMinutes(),
  )}`;
}

async function exportSaveFile() {
  try {
    const path = await saveFileDialog({
      defaultPath: `文字农场-存档-${fileStamp()}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await exportSave(path);
    notice.value = `存档已导出：${path}`;
  } catch (e) {
    notice.value = `导出存档失败：${errorText(e)}`;
  }
}

async function importSaveFile() {
  try {
    const picked = await openFileDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (typeof picked !== "string") return;
    const reply = await importSave(picked);
    snapshot.value = reply.snapshot;
    content.value = reply.content;
    notice.value = reply.warning ? `存档已导入：${reply.warning}` : "存档已导入。";
  } catch (e) {
    notice.value = `导入存档失败：${errorText(e)}`;
  }
}

async function exportContentFile() {
  try {
    const path = await saveFileDialog({
      defaultPath: `文字农场-内容包-${fileStamp()}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await exportContent(path);
    notice.value = `内容包已导出：${path}`;
  } catch (e) {
    notice.value = `导出内容包失败：${errorText(e)}`;
  }
}

async function importContentFile() {
  try {
    const picked = await openFileDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (typeof picked !== "string") return;
    const reply = await importContent(picked);
    snapshot.value = reply.snapshot;
    content.value = reply.content;
    resetEditor();
    notice.value = "内容包已导入。";
  } catch (e) {
    notice.value = `导入内容包失败：${errorText(e)}`;
  }
}

function resetEditor() {
  editor.value = content.value
    ? (JSON.parse(JSON.stringify(content.value)) as ContentPack)
    : null;
  editorIssues.value = [];
  editorMessage.value = "";
}

function addCrop() {
  if (!editor.value) return;
  editor.value.crops.push({
    id: `custom_${Date.now()}`,
    name: "新作物",
    icon: "🌱",
    unlockLevel: 1,
    growMinutes: 10,
    regrowMinutes: 0,
    seasons: 1,
    yieldPerSeason: 1,
    seedPrice: 5,
    sellPrice: 10,
    exp: 2,
    note: "",
    sowSeasons: ["spring", "summer", "autumn", "winter"],
    region: "all",
    resistances: { drought: 0, flood: 0, cold: 0, pest: 0 },
  });
}

function removeCrop(index: number) {
  if (!editor.value) return;
  const crop = editor.value.crops[index];
  if (!window.confirm(`删除「${crop?.name ?? "这个作物"}」？正在种或仓库里有货时会被拒绝。`)) {
    return;
  }
  editor.value.crops.splice(index, 1);
}

async function previewEditor() {
  if (!editor.value) return;
  try {
    const preview = await previewContent(editor.value);
    editorIssues.value = preview.issues;
    editorMessage.value = preview.issues.length
      ? "校验没通过，按上面的错误改。"
      : `校验通过：${preview.crops.length} 种作物；平均收益见控制台。`;
    if (!preview.issues.length) {
      console.table(preview.crops);
    }
  } catch (e) {
    editorMessage.value = `校验失败：${errorText(e)}`;
  }
}

async function saveEditor() {
  if (!editor.value) return;
  try {
    const preview = await previewContent(editor.value);
    editorIssues.value = preview.issues;
    if (preview.issues.length) {
      editorMessage.value = "有错误，未保存。";
      return;
    }
    const reply = await applyContent(editor.value);
    snapshot.value = reply.snapshot;
    content.value = reply.content;
    resetEditor();
    notice.value = "内容包已保存。";
  } catch (e) {
    editorMessage.value = `保存失败：${errorText(e)}`;
  }
}

async function generateSeedBatch() {
  if (!editor.value) return;
  try {
    const reviews = await reviewSeeds(seedCount.value, seedValue.value);
    const accepted = reviews.filter((review) => review.status !== "rejected");
    const rejected = reviews.filter((review) => review.status === "rejected");
    const existing = new Set(editor.value.crops.map((crop) => crop.id));
    const fresh = accepted
      .map((review) => review.crop)
      .filter((crop) => !existing.has(crop.id));
    editor.value.crops.push(...fresh);
    const revised = reviews.filter((review) => review.status === "revised").length;
    editorMessage.value = `审核 ${reviews.length} 个：通过 ${accepted.length - revised}、修正 ${revised}、作废 ${rejected.length}；已追加 ${fresh.length} 个。`;
    seedReviewNotes.value = rejected.slice(0, 5).map(
      (review) => `作废「${review.crop.name}」：${review.reasons.join("；")}`,
    );
  } catch (e) {
    editorMessage.value = `生成失败：${errorText(e)}`;
  }
}

function setExpRate(rate: number) {
  void act({ type: "updateSettings", expRate: rate });
}

function showGuide() {
  void act({ type: "showGuide" });
}

function toggleDebugTime() {
  window.localStorage.setItem("farm.debugTime", debugTime.value ? "1" : "0");
}

function systemPrefersDark(): boolean {
  if (themeMedia) return themeMedia.matches;
  return typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)").matches
    : false;
}

/// 主题只在 <html> 上落 data-theme / data-palette + accent 变量；Rust 与存档都不参与。
function syncTheme() {
  applyTheme(
    document.documentElement,
    { mode: themeMode.value, palette: paletteId.value },
    systemPrefersDark(),
  );
}

function saveTheme() {
  writeThemePrefs(window.localStorage, { mode: themeMode.value, palette: paletteId.value });
  syncTheme();
}

function setThemeMode(mode: ThemeMode) {
  themeMode.value = mode;
  saveTheme();
}

function setPalette(id: PaletteId) {
  paletteId.value = id;
  saveTheme();
}

function resetAppearance() {
  themeMode.value = DEFAULT_PREFS.mode;
  paletteId.value = DEFAULT_PREFS.palette;
  saveTheme();
}

function onSystemThemeChange() {
  // 只有「跟随系统」会真的换色；浅色/深色重新应用一次也无副作用。
  syncTheme();
}

async function advanceFarm(hours: number) {
  if (acting.value) return;
  acting.value = true;
  try {
    const reply = await advanceTime(hours);
    snapshot.value = reply.snapshot;
    notice.value = `时间加速：农场快进了 ${hours} 小时。`;
    if (reply.saveError) {
      saveMessage.value = `已快进，但保存失败：${reply.saveError}`;
    }
  } catch (e) {
    notice.value = `快进失败：${errorText(e)}`;
  } finally {
    acting.value = false;
  }
}

async function refreshSnapshot() {
  if (acting.value) return;
  try {
    const next = await getSnapshot();
    snapshot.value = next;
    clockOffset = next.now - Date.now();
  } catch {
    // 读不到就等下一轮；不要为一次瞬时失败打断界面。
  }
}

async function syncFarm() {
  if (acting.value) return;
  try {
    const reply = await syncGame();
    snapshot.value = reply.snapshot;
    if (reply.saveError) {
      saveMessage.value = `已生效，但保存失败：${reply.saveError}`;
    }
  } catch {
    // 同步失败不打断界面。
  }
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [nextSnapshot, nextInfo, nextContent] = await Promise.all([
      getSnapshot(),
      getAppInfo(),
      getContent(),
    ]);
    snapshot.value = nextSnapshot;
    info.value = nextInfo;
    content.value = nextContent;
    resetEditor();
    clockOffset = nextSnapshot.now - Date.now();
    tick.value = nextSnapshot.now;
    maybeShowAutoReport();
  } catch (e) {
    error.value = `读取失败：${errorText(e)}`;
  } finally {
    loading.value = false;
  }
}

async function doSave() {
  saveMessage.value = "";
  try {
    await saveNow();
    saveMessage.value = `已保存 · ${new Date().toLocaleTimeString()}`;
  } catch (e) {
    saveMessage.value = `保存失败：${errorText(e)}`;
  }
}

onMounted(() => {
  window.addEventListener("keydown", onFishingShortcut);
  themeMedia =
    typeof window.matchMedia === "function"
      ? window.matchMedia("(prefers-color-scheme: dark)")
      : null;
  syncTheme();
  themeMedia?.addEventListener("change", onSystemThemeChange);
  void load();
  timer = window.setInterval(() => {
    tick.value = Date.now() + clockOffset;
  }, 1000);
  refreshTimer = window.setInterval(() => {
    void refreshSnapshot();
  }, 5000);
  syncTimer = window.setInterval(() => {
    void syncFarm();
  }, 30000);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onFishingShortcut);
  themeMedia?.removeEventListener("change", onSystemThemeChange);
  themeMedia = null;
  if (timer !== undefined) window.clearInterval(timer);
  if (refreshTimer !== undefined) window.clearInterval(refreshTimer);
  if (syncTimer !== undefined) window.clearInterval(syncTimer);
});
</script>

<template>
  <div class="page" :class="seasonClass">
    <header class="icons">
      <button
        v-for="item in topIcons"
        :key="item.name"
        class="icon-item"
        type="button"
        @click="openTop(item.name)"
      >
        <span class="icon-emoji">{{ item.icon }}</span>
        <span>{{ item.name }}</span>
      </button>
    </header>

    <nav v-if="activeTab !== '畜牧场' && activeTab !== '制造屋'" class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab"
        type="button"
        class="tab"
        :class="{ active: tab === activeTab }"
        @click="selectTab(tab)"
      >
        {{ tab }}
      </button>
    </nav>
    <nav v-else-if="activeTab === '畜牧场'" class="tabs ranch-tabs">
      <button class="tab active" type="button" disabled>畜牧场</button>
      <button class="tab" type="button" @click="selectTab('我的农场')">返回农场</button>
      <button class="tab" type="button" @click="selectTab('制造屋')">制造屋</button>
      <button class="tab" type="button" @click="selectTab('宠物')">宠物</button>
    </nav>
    <nav v-else class="tabs ranch-tabs">
      <button class="tab active" type="button" disabled>制造屋</button>
      <button class="tab" type="button" @click="selectTab('我的农场')">返回农场</button>
      <button class="tab" type="button" @click="selectTab('畜牧场')">畜牧场</button>
      <button class="tab" type="button" @click="selectTab('宠物')">宠物</button>
    </nav>

    <p v-if="error" class="banner error">{{ error }}</p>
    <p v-if="notice" class="banner notice">
      {{ notice }}
      <button class="link" type="button" @click="notice = ''">×</button>
    </p>

    <main>
      <div class="season-banner">
        <strong>{{ snapshot?.seasonEmoji }} {{ snapshot?.seasonLabel }}</strong>
        <span class="muted">{{ snapshot?.regionLabel }}</span>
        <span class="muted">当季作物正常生长；反季种植会减速、减产，温室只庇护自己的独立地块。</span>
      </div>
      <div v-show="activeTab === '我的农场'">
      <section v-if="snapshot && !snapshot.onboarded" class="guide">
        <div>
          <strong>第一次来？</strong>
          ① 在下面种子铺买种子　② 点空地 [播种]　③ 成熟后 [收获]　④ 仓库 [全卖]
        </div>
        <button class="link" type="button" :disabled="acting" @click="dismissGuide">
          知道了
        </button>
      </section>

      <div v-if="activePets[0]" class="farm-pet-strip" @click="selectTab('宠物')">
        <img :src="activePets[0].image" :alt="activePets[0].species" />
        <span>
          {{ activePets[0].speciesEmoji }} {{ activePets[0].name }} 在{{ activePets[0].location === 'ranch' ? '牧场' : '农场' }}陪着你
          <template v-if="activePets.length > 1">（还有 {{ activePets.length - 1 }} 只）</template>
          · {{ activePets[0].moodLabel }}
        </span>
        <button class="link" type="button">[查看宠物]</button>
      </div>
      <div v-else class="farm-pet-strip farm-pet-empty">
        <span>{{ pets.length ? "宠物都在寄养中。" : "还没有宠物。" }}</span>
        <button class="link" type="button" @click="selectTab('宠物')">去「宠物」页领养狗狗/猫猫</button>
      </div>

      <div class="level">{{ levelLine }}</div>
      <div class="sub">
        图鉴:{{ snapshot?.codexDiscovered ?? 0 }}/{{ snapshot?.contentCrops ?? 0 }}　成就:{{
          snapshot?.achievementsEarned ?? 0
        }}/24.查看　·　{{ snapshot?.farmName ?? "我的农场" }}
      </div>
      <div class="sub">
        一键:开启({{ hasHarvester ? "收割机" : "无" }})
        <button
          v-if="anyReady && hasHarvester"
          class="link"
          type="button"
          :disabled="acting"
          @click="harvestAll"
        >
          [一键收获]
        </button>
        　灌溉机:{{ snapshot?.facilities.find((f) => f.kind === "sprinkler")?.owned ? "已建设" : "无" }}.使用　·　{{
          currency
        }}
        {{ snapshot?.coins ?? 0 }}
      </div>
      <div class="rule"></div>

      <section class="plots">
        <div v-for="plot in snapshot?.plots ?? []" :key="plot.index" class="plot-line">
          <template v-if="!plot.unlocked">
            <span class="muted">[{{ plot.label }}]{{ plot.unlockLevel }}级可开垦.</span>
            <button
              class="link"
              type="button"
              :disabled="acting || (snapshot?.level ?? 1) < plot.unlockLevel"
              @click="unlock(plot)"
            >
              [开垦 {{ plot.unlockCost }}{{ currency }}]
            </button>
          </template>

          <template v-else-if="!plot.crop">
            <span>{{ plot.label }}[{{ plot.tierName }}]:空地</span>
            <button class="link" type="button" :disabled="acting" @click="plant(plot)">
              [播种]
            </button>
          </template>

          <template v-else>
            <span>{{ plot.label }}[{{ plot.tierName }}]:{{ cropName(plot) }}</span>
            <span v-if="plot.crop?.offSeason" class="off-season-tag">反季</span>
            <span v-if="plot.crop?.greenhouse" class="greenhouse-tag">温室</span>
            <span> ({{ cropStatusText(plot) }}){{ cropSeasonText(plot) }}</span>
            <button
              v-if="canClearPest(plot)"
              class="link pest-link"
              type="button"
              :disabled="acting"
              @click="clearPest(plot)"
            >
              [{{ plot.crop?.pest?.kind === "bug" ? "杀虫" : "除草" }}]
            </button>
            <button
              v-if="plot.crop?.canMitigate"
              class="link disaster-link"
              type="button"
              :disabled="acting"
              @click="mitigate(plot)"
            >
              [{{ plot.crop?.disaster?.mitigation }}]
            </button>
            <button
              v-if="canWater(plot)"
              class="link"
              type="button"
              :disabled="acting"
              @click="water(plot)"
            >
              [浇水]
            </button>
            <button
              v-if="canHarvest(plot)"
              class="link"
              type="button"
              :disabled="acting"
              @click="harvest(plot)"
            >
              [收获]
            </button>
            <button
              v-if="canHarvest(plot) && hasHarvester"
              class="link"
              type="button"
              :disabled="acting"
              @click="harvest(plot)"
            >
              [收割机收获]
            </button>
          </template>
          <button
            v-if="plot.crop?.withered"
            class="link disaster-link"
            type="button"
            :disabled="acting"
            @click="clearCrop(plot)"
          >
            [清除枯萎]
          </button>
          <span
            v-if="plot.unlocked && plot.tier > 0"
            class="land-effects"
            :title="landEffectsText(plot)"
          >
            （收成+{{ plot.effects.yieldPct }}%）
          </span>
          <button
            v-if="plot.unlocked && plot.canUpgrade"
            class="link upgrade-link"
            type="button"
            :disabled="acting"
            @click="upgrade(plot)"
          >
            [升级 {{ plot.upgradeCost }}{{ currency }}]
          </button>
          <span v-else-if="plot.unlocked && !plot.maxTier" class="muted">
            [{{ plot.upgradeLevel }}级可升级]
          </span>
        </div>
      </section>

      <section class="panel">
        <div class="panel-title">仓库</div>
        <div v-if="seedStacks.length" class="row">
          <span class="muted">种子：</span>
          <span v-for="item in seedStacks" :key="item.key" class="item">
            {{ item.name }}×{{ item.qty }}
          </span>
        </div>
        <div v-for="item in produce" :key="item.key" class="row">
          <span>{{ item.name }}×{{ item.qty }}</span>
          <button class="link" type="button" :disabled="acting" @click="sell(item, false)">
            [卖1]
          </button>
          <button class="link" type="button" :disabled="acting" @click="sell(item, true)">
            [全卖]
          </button>
        </div>
        <div v-if="!seedStacks.length && !produce.length" class="muted">仓库还是空的。</div>
      </section>

      <section class="panel">
        <div class="panel-title">农场设施</div>
        <div v-for="facility in snapshot?.facilities ?? []" :key="facility.kind" class="row">
          <span class="fac-name">
            {{ facility.name }}
            <span v-if="facility.maxCount > 1">（{{ facility.count }}/{{ facility.maxCount }}）</span>
          </span>
          <span class="muted">
            <template v-if="facility.count === 0">
              {{ facility.unlockLevel }}级 · {{ facility.nextCost }}{{ currency }}
            </template>
            <template v-else>
              已建设
              <span v-if="facility.damaged > 0" class="warn">
                损坏 {{ facility.damaged }} 台
              </span>
            </template>
          </span>
          <button
            v-if="facility.count < facility.maxCount"
            class="link"
            type="button"
            :disabled="acting || (snapshot?.level ?? 1) < facility.unlockLevel"
            @click="buyFacility(facility)"
          >
            {{ facility.count === 0 ? "[建设]" : `[再建 ${facility.nextCost}]` }}
          </button>
          <button
            v-if="facility.kind === 'windmill' && facility.damaged > 0"
            class="link"
            type="button"
            :disabled="acting"
            @click="repairWindmills"
          >
            [维修]
          </button>
        </div>
        <div class="muted">
          灌溉机：播种即浇水；风车：每台 +1% 生长（上限 15 台，台风/气候灾害可能损坏，维修每台 3,000）；收割机：一键收获；温室：每座 3 块独立地，最多 3 座。
        </div>
      </section>

      <section class="panel">
        <div class="panel-title">设置</div>
        <div class="row">
          <span>经验倍率</span>
          <button
            v-for="rate in [1, 2, 5]"
            :key="rate"
            class="rate-btn"
            :class="{ active: snapshot?.expRate === rate }"
            type="button"
            :disabled="acting"
            @click="setExpRate(rate)"
          >
            {{ rate }}×
          </button>
        </div>
        <div class="row">
          <span>气候区</span>
          <button
            class="rate-btn"
            :class="{ active: snapshot?.region === 'north' }"
            type="button"
            :disabled="acting"
            @click="setRegion('north')"
          >
            北方温带
          </button>
          <button
            class="rate-btn"
            :class="{ active: snapshot?.region === 'south' }"
            type="button"
            :disabled="acting"
            @click="setRegion('south')"
          >
            南方亚热带
          </button>
          <span class="muted">南方反季减速/减产更轻</span>
        </div>
        <div class="row">
          <button class="link" type="button" @click="showGuide">重新显示新手引导</button>
        </div>
        <div class="row">
          <label class="debug-toggle">
            <input v-model="debugTime" type="checkbox" @change="toggleDebugTime" />
            调试：时间加速
          </label>
        </div>
        <div v-if="debugTime" class="row">
          <span class="muted">快进：</span>
          <button class="rate-btn" type="button" :disabled="acting" @click="advanceFarm(1)">
            +1 小时
          </button>
          <button class="rate-btn" type="button" :disabled="acting" @click="advanceFarm(24)">
            +1 天
          </button>
          <button class="rate-btn" type="button" :disabled="acting" @click="advanceFarm(168)">
            +7 天
          </button>
          <span class="muted">会改当前存档（自动备份），用来观察生长和 NPC。</span>
        </div>
      </section>

      <section class="panel">
        <div class="panel-title">外观</div>
        <div class="row">
          <span>明暗</span>
          <button
            v-for="mode in THEME_MODES"
            :key="mode.id"
            class="rate-btn"
            :class="{ active: themeMode === mode.id }"
            type="button"
            @click="setThemeMode(mode.id)"
          >
            {{ mode.name }}
          </button>
        </div>
        <div class="row">
          <span>配色</span>
          <button
            v-for="palette in PALETTES"
            :key="palette.id"
            class="swatch"
            :class="{ active: paletteId === palette.id }"
            type="button"
            :style="`--swatch-color: ${palette.accent}`"
            @click="setPalette(palette.id)"
          >
            <i class="swatch-dot" aria-hidden="true"></i>
            {{ palette.name }}
          </button>
          <button class="link" type="button" @click="resetAppearance">恢复默认</button>
        </div>
        <div class="row muted appearance-hint">
          只存在这台电脑上（localStorage），不写存档；默认跟随系统，改完立即生效。
        </div>
      </section>

      <section v-if="daily" class="panel">
        <div class="panel-title">今日任务 <span class="muted">{{ daily.day }}</span></div>
        <div class="row">
          <span>每日签到</span>
          <button
            v-if="!daily.signIn"
            class="link"
            type="button"
            :disabled="acting"
            @click="claimSignIn"
          >
            [签到]
          </button>
          <span v-else class="ok">已签到</span>
          <span class="muted">金币+20 · 小青菜种子+2</span>
        </div>
        <div v-for="task in daily.tasks" :key="task.id" class="row">
          <span>{{ task.name }}</span>
          <span class="muted">{{ task.progress }}/{{ task.target }}</span>
          <button
            v-if="task.claimable"
            class="link"
            type="button"
            :disabled="acting"
            @click="claimDaily(task.id)"
          >
            [领奖]
          </button>
          <span v-else-if="task.claimed" class="ok">已领</span>
          <span class="muted">金币+{{ task.rewardCoins }} · 经验+{{ task.rewardExp }}</span>
        </div>
      </section>

      <details v-if="levelRewards.length" class="panel">
        <summary class="panel-title">
          等级奖励
          <span v-if="levelRewardClaimable > 0" class="ok">{{ levelRewardClaimable }} 份可领</span>
          <span v-else class="muted">已领完</span>
        </summary>
        <div class="ach-grid">
          <div
            v-for="reward in levelRewards"
            :key="reward.level"
            class="ach"
            :class="{ earned: reward.claimed, claimable: reward.claimable }"
          >
            <span class="ach-name">{{ reward.claimed ? "★" : "☆" }} Lv{{ reward.level }}</span>
            <span class="muted reward-text">{{ reward.text }}</span>
            <span class="reward-action">
              <button
                v-if="reward.claimable"
                class="link"
                type="button"
                :disabled="acting"
                @click="claimLevelReward(reward.level)"
              >
                [领奖]
              </button>
              <span v-else-if="reward.claimed" class="ok">已领</span>
              <span v-else class="muted">{{ reward.level }} 级可领</span>
            </span>
          </div>
        </div>
      </details>

      <details class="panel">
        <summary class="panel-title">
          成就 {{ snapshot?.achievementsEarned ?? 0 }}/24
        </summary>
        <div class="ach-grid">
          <div
            v-for="achievement in achievements"
            :key="achievement.id"
            class="ach"
            :class="{ earned: achievement.earned }"
          >
            <span class="ach-name">
              {{ achievement.earned ? "★" : "☆" }} {{ achievement.name }}
            </span>
            <span class="muted">
              {{ achievement.desc }} · {{ achievement.progress }}/{{ achievement.target }}
            </span>
          </div>
        </div>
      </details>

      <details class="panel">
        <summary class="panel-title">
          图鉴 {{ snapshot?.codexDiscovered ?? 0 }}/{{ snapshot?.contentCrops ?? 0 }}
        </summary>
        <div class="codex-grid">
          <div
            v-for="entry in codex"
            :key="entry.cropId"
            class="codex-item"
            :class="{ locked: !entry.discovered }"
          >
            <span>{{ entry.icon }} {{ entry.discovered ? entry.name : "???" }}</span>
            <span v-if="entry.discovered" class="muted">
              累计 {{ entry.total }} · 最高 {{ entry.best }}
            </span>
          </div>
        </div>
      </details>

      <section class="panel">
        <div class="panel-title">种子铺 <span class="muted">（买完回到地块点播种）</span></div>
        <div class="shop-grid">
          <div
            v-for="crop in content?.crops ?? []"
            :key="crop.id"
            class="shop-item"
            :class="{ locked: crop.unlockLevel > (snapshot?.level ?? 1) }"
          >
            <span class="shop-name">
              {{ crop.icon }} {{ crop.name }}
              <span class="season-tag" :class="isInSeason(crop) ? 'in' : 'off'">
                {{ isInSeason(crop) ? "当季" : "反季" }}
              </span>
              <span class="season-tag region-tag">{{ affinityLabel(crop.region) }}</span>
            </span>
            <span class="shop-meta">
              {{ crop.unlockLevel }}级 · {{ formatGrowMinutes(crop.growMinutes) }}
              <template v-if="crop.seasons > 1"> ×{{ crop.seasons }}季</template>
              · 种子 {{ crop.seedPrice }} · {{ resistanceText(crop) }}
            </span>
            <button
              v-if="crop.unlockLevel <= (snapshot?.level ?? 1)"
              class="link"
              type="button"
              :disabled="acting"
              @click="buy(crop)"
            >
              [买1]
            </button>
            <span v-else class="muted">[{{ crop.unlockLevel }}级]</span>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-title">存档与内容包</div>
        <div class="file-actions">
          <button type="button" @click="exportSaveFile">导出存档</button>
          <button type="button" @click="importSaveFile">导入存档</button>
          <button type="button" @click="exportContentFile">导出内容包</button>
          <button type="button" @click="importContentFile">导入内容包</button>
        </div>
        <div class="muted">内容包不能删除正在种或仓库里还有的作物。</div>
      </section>

      <details v-if="editor" class="panel">
        <summary class="panel-title">内容编辑器（名字 / 数值 / 名词）</summary>
        <div class="editor-nouns">
          <label>农场名 <input v-model="editor.nouns.farm" /></label>
          <label>主人名 <input v-model="editor.nouns.owner" /></label>
          <label>地块名 <input v-model="editor.nouns.plot" /></label>
          <label>货币名 <input v-model="editor.nouns.currency" /></label>
        </div>
        <div class="editor-generator">
          <strong>种子生成器</strong>
          <label>数量<input v-model.number="seedCount" type="number" min="1" max="200" /></label>
          <label>种子码<input v-model.number="seedValue" type="number" min="1" /></label>
          <button type="button" @click="generateSeedBatch">生成并审核</button>
          <span class="muted">会追加到草稿；可先改再保存</span>
        </div>
        <p v-if="seedReviewNotes.length" class="warn">
          {{ seedReviewNotes.join("；") }}
        </p>
        <div class="editor-list">
          <div v-for="(crop, index) in editor.crops" :key="crop.id" class="editor-row">
            <code class="crop-id">{{ crop.id }}</code>
            <input v-model="crop.name" class="name-input" placeholder="名字" />
            <input v-model="crop.icon" class="icon-input" maxlength="2" title="图标" />
            <label>级<input v-model.number="crop.unlockLevel" type="number" min="1" max="30" /></label>
            <label>成熟<input v-model.number="crop.growMinutes" type="number" min="1" /></label>
            <label>再熟<input v-model.number="crop.regrowMinutes" type="number" min="0" /></label>
            <label>季<input v-model.number="crop.seasons" type="number" min="1" max="4" /></label>
            <label>产量<input v-model.number="crop.yieldPerSeason" type="number" min="1" /></label>
            <label>种子<input v-model.number="crop.seedPrice" type="number" min="0" /></label>
            <label>售价<input v-model.number="crop.sellPrice" type="number" min="0" /></label>
            <label>经验<input v-model.number="crop.exp" type="number" min="0" /></label>
            <span class="season-checks">
              <label v-for="season in SEASONS" :key="season">
                <input
                  type="checkbox"
                  :checked="crop.sowSeasons.includes(season)"
                  @change="toggleSeason(crop, season)"
                />
                {{ SEASON_NAMES[season] }}
              </label>
            </span>
            <select v-model="crop.region" class="region-select">
              <option value="all">通用</option>
              <option value="north">北方</option>
              <option value="south">南方</option>
            </select>
            <span class="resistance-fields">
              <label>旱<input v-model.number="crop.resistances.drought" type="number" min="0" max="80" /></label>
              <label>涝<input v-model.number="crop.resistances.flood" type="number" min="0" max="80" /></label>
              <label>寒<input v-model.number="crop.resistances.cold" type="number" min="0" max="80" /></label>
              <label>虫<input v-model.number="crop.resistances.pest" type="number" min="0" max="80" /></label>
            </span>
            <button class="link" type="button" @click="removeCrop(index)">[删]</button>
          </div>
        </div>
        <div class="file-actions">
          <button type="button" @click="addCrop">+ 新作物</button>
          <button type="button" @click="previewEditor">校验</button>
          <button type="button" @click="saveEditor">保存内容包</button>
          <button type="button" @click="resetEditor">还原</button>
        </div>
        <p v-if="editorIssues.length" class="warn">{{ editorIssues.join("；") }}</p>
        <p v-if="editorMessage" class="muted">{{ editorMessage }}</p>
      </details>

      <section class="boot">
        <div class="boot-head">
          <strong>A2 存档/内容包已接上</strong>
          <span class="muted">Rust → Tauri → Vue</span>
          <span v-if="loading" class="muted">读取中…</span>
          <span v-if="acting" class="muted">结算中…</span>
        </div>
        <div v-if="info" class="boot-grid">
          <span>内核 v{{ info.coreVersion }}</span>
          <span>壳 v{{ info.version }}</span>
          <span>存档 schema v{{ info.schema }}</span>
        </div>
        <div class="boot-actions">
          <button type="button" @click="load">重读快照</button>
          <button type="button" @click="doSave">立刻保存</button>
          <span v-if="saveMessage" class="muted">{{ saveMessage }}</span>
        </div>
        <div v-if="info" class="path">{{ info.savePath }}</div>
        <p v-if="info?.bootWarning" class="warn">{{ info.bootWarning }}</p>
      </section>
      </div>

      <div v-show="activeTab === '邻居农场'">
        <section class="panel">
          <div class="panel-title">
            邻居农场
            <span class="muted">今日可偷 {{ snapshot?.stealsLeft ?? 0 }}/5 · 可帮忙 {{ snapshot?.helpsLeft ?? 0 }}/10</span>
          </div>
          <div v-for="npc in npcs" :key="npc.id" class="npc-card">
            <div class="npc-head">
              <strong>{{ npc.name }}</strong>
              <span class="muted">{{ npc.level }}级 · {{ npc.relation }}（{{ npc.friendship }}）</span>
              <button class="link" type="button" :disabled="acting" @click="greetNpc(npc)">
                [留句话]
              </button>
            </div>
            <div v-for="plot in npc.plots" :key="plot.index" class="npc-plot">
              <span>{{ plot.label }}[普通]:{{ plot.cropName }}</span>
              <span v-if="plot.status === 'growing'" class="muted">
                (生长中 {{ formatRemain(plot.readyAt - tick) }})
              </span>
              <span v-else-if="plot.status === 'ready'" class="ok">(可偷)</span>
              <button
                v-if="plot.canHelp"
                class="link"
                type="button"
                :disabled="acting"
                @click="helpNpc(npc, plot.index)"
              >
                [帮忙浇水]
              </button>
              <button
                v-if="plot.canSteal"
                class="link"
                type="button"
                :disabled="acting"
                @click="stealNpc(npc, plot.index)"
              >
                [偷1]
              </button>
              <span v-if="plot.helped" class="muted">已帮忙</span>
            </div>
          </div>
        </section>

        <section class="panel">
          <div class="panel-title">邻居留言</div>
          <div v-for="(message, index) in npcMessages" :key="index" class="log-line">
            ({{ formatStamp(message.at) }})
            {{ message.fromPlayer ? "你" : message.npcName }}：{{ message.text }}
          </div>
          <div v-if="!npcMessages.length" class="muted">还没有留言。</div>
        </section>
      </div>

      <div v-show="activeTab === '畜牧场'">
        <div v-if="activePets[0]" class="farm-pet-strip" @click="selectTab('宠物')">
          <img :src="activePets[0].image" :alt="activePets[0].species" />
          <span>
            {{ activePets[0].speciesEmoji }} {{ activePets[0].name }} 在{{ activePets[0].location === 'ranch' ? '牧场' : '农场' }}陪着你
            <template v-if="activePets.length > 1">（还有 {{ activePets.length - 1 }} 只）</template>
            · {{ activePets[0].moodLabel }}
          </span>
          <button class="link" type="button">[查看宠物]</button>
        </div>
        <div v-else class="farm-pet-strip farm-pet-empty">
          <span>{{ pets.length ? "宠物都在寄养中。" : "还没有宠物。" }}</span>
          <button class="link" type="button" @click="selectTab('宠物')">去「宠物」页领养狗狗/猫猫</button>
        </div>
        <div class="level">{{ levelLine }}</div>
        <div class="sub">
          图鉴:{{ snapshot?.codexDiscovered ?? 0 }}/{{ snapshot?.contentCrops ?? 0 }}　成就:{{
            snapshot?.achievementsEarned ?? 0
          }}/24.查看　·　{{ snapshot?.farmName ?? "我的农场" }}
        </div>
        <div class="sub">
          {{ currency }} {{ snapshot?.coins ?? 0 }}
          <span class="muted">· 牧场产物卖出后实时到账</span>
        </div>
        <div class="rule"></div>
        <section class="panel">
          <div class="panel-title">
            畜牧场
            <span class="muted">鸡/鸭/牛/羊；宠物派来牧场可降低生病概率</span>
          </div>
          <div class="ranch-feed-bar">
            <span>专用饲料 ×{{ ranch?.feedCount ?? 0 }}</span>
            <button class="link" type="button" :disabled="acting" @click="buyRanchFeed">[买专用饲料 300]</button>
            <button v-if="ranch?.canBreed" class="link" type="button" :disabled="acting" @click="breedRanch">[繁殖]</button>
            <span class="muted">专用饲料提升品质；繁殖消耗 1 份饲料，需要空栏</span>
          </div>
          <div class="ranch-pens">
            <div v-for="pen in ranch?.pens ?? []" :key="pen.index" class="ranch-pen">
              <div class="ranch-pen-head">畜栏 {{ pen.index }}</div>
              <template v-if="pen.animal">
                <div class="ranch-animal">
                  <span class="ranch-emoji">{{ pen.animal.emoji }}</span>
                  <div>
                    <strong>{{ pen.animal.name }}</strong>
                    <div class="muted">{{ pen.animal.label }} · {{ pen.animal.qualityLabel }} · 第{{ pen.animal.generation + 1 }}代 · 饱食 {{ pen.animal.satiety }}/100 · 健康 {{ pen.animal.health }}/100</div>
                    <div v-if="pen.animal.sick" class="warn">生病了，需要兽医</div>
                    <div v-else class="muted">
                      {{ pen.animal.canCollect ? `${pen.animal.product}可以收了` : `${pen.animal.product}还要 ${formatRemain(pen.animal.remainingMs)}` }}
                    </div>
                  </div>
                </div>
                <div class="ranch-actions">
                  <button class="link" type="button" :disabled="acting || !pen.animal.canFeed" @click="feedRanchAnimal(pen.index)">
                    [喂食]
                  </button>
                  <button
                    class="link"
                    type="button"
                    :disabled="acting || !pen.animal.canFeed || (ranch?.feedCount ?? 0) === 0"
                    @click="feedRanchPremium(pen.index)"
                  >
                    [喂专用饲料]
                  </button>
                  <button class="link" type="button" :disabled="acting || !pen.animal.canCollect" @click="collectRanchProduct(pen.index)">
                    [收{{ pen.animal.product }}]
                  </button>
                  <button class="link" type="button" :disabled="acting || !pen.animal.canHeal" @click="healRanchAnimal(pen.index)">
                    [看兽医 300]
                  </button>
                </div>
              </template>
              <template v-else>
                <div class="muted">空畜栏，买一只：</div>
                <div class="ranch-buy">
                  <button class="link" type="button" :disabled="acting" @click="buyRanchAnimal(pen.index, 'chicken')">🐔 母鸡 800</button>
                  <button class="link" type="button" :disabled="acting" @click="buyRanchAnimal(pen.index, 'duck')">🦆 鸭子 1200</button>
                  <button class="link" type="button" :disabled="acting" @click="buyRanchAnimal(pen.index, 'cow')">🐄 奶牛 5000</button>
                  <button class="link" type="button" :disabled="acting" @click="buyRanchAnimal(pen.index, 'sheep')">🐑 绵羊 3500</button>
                </div>
              </template>
            </div>
          </div>
          <div class="ranch-workshop">
            <div class="ranch-workshop-head">
              <strong>🏭 制造屋</strong>
              <span class="muted">牧场原料可以拿到制造屋，做成奶酪/黄油/蛋糕/毛线/咸鸭蛋</span>
            </div>
            <div class="row">
              <button class="link" type="button" @click="selectTab('制造屋')">
                [去制造屋{{ workshop?.built ? "" : "（还没建）" }}]
              </button>
              <span v-if="workshop?.slotViews[0]?.job" class="muted">
                手工台正在做 {{ workshop.slotViews[0].job.outputName }}
              </span>
            </div>
          </div>
          <div v-if="ranch?.canExpand" class="ranch-expand">
            <button class="link" type="button" :disabled="acting" @click="expandRanch">
              [扩建畜栏 {{ ranch.expandCost }}]
            </button>
            <span class="muted">当前 {{ ranch.pens.length }} / {{ ranch.maxPens }} 栏</span>
          </div>
          <div v-if="ranchProducts.length" class="ranch-products">
            <strong>牧场产物</strong>
            <span v-for="item in ranchProducts" :key="item.key" class="ranch-product">
              {{ item.name }}×{{ item.qty }}
              <span class="muted">每个 {{ item.unitPrice }}</span>
              <button class="link" type="button" :disabled="acting" @click="sell(item, false)">[卖1]</button>
              <button class="link" type="button" :disabled="acting" @click="sell(item, true)">[全卖]</button>
            </span>
          </div>
          <div class="ranch-neighbors">
            <div class="ranch-neighbors-head">
              <strong>邻居牧场</strong>
              <span class="muted">
                帮忙 {{ snapshot?.ranchHelpsLeft ?? 0 }}/10 · 偷产物 {{ snapshot?.ranchStealsLeft ?? 0 }}/5
                <template v-if="snapshot">（和农场次数独立）</template>
              </span>
            </div>
            <div v-for="npc in npcs" :key="npc.id" class="ranch-neighbor">
              <span class="ranch-neighbor-name">{{ npc.name }} · {{ npc.relation }}</span>
              <span v-for="animal in npc.ranch" :key="animal.index" class="ranch-neighbor-animal">
                {{ animal.emoji }} {{ animal.label }}
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !animal.canHelp || (snapshot?.ranchHelpsLeft ?? 0) === 0"
                  @click="helpNpcRanch(npc, animal.index)"
                >
                  [帮忙]
                </button>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !animal.canSteal || (snapshot?.ranchStealsLeft ?? 0) === 0"
                  @click="stealNpcRanch(npc, animal.index)"
                >
                  [偷{{ animal.product }}]
                </button>
              </span>
            </div>
          </div>
          <div class="ranch-guard">
            <strong>牧场守卫：</strong>
            <span v-if="!activePets.length" class="muted">没有当值宠物</span>
            <span v-for="pet in activePets" :key="pet.id" class="ranch-guard-pet">
              {{ pet.name }}
              <button v-if="pet.location !== 'ranch'" class="link" type="button" :disabled="acting" @click="sendPetToRanch(pet.id)">[派去牧场]</button>
              <button v-else class="link" type="button" :disabled="acting" @click="callPetHome(pet.id)">[叫回农场]</button>
            </span>
          </div>
        </section>
      </div>

      <div v-show="activeTab === '制造屋'">
        <div class="level">{{ levelLine }}</div>
        <div class="sub">
          {{ currency }} {{ snapshot?.coins ?? 0 }}
          <span class="muted">· 制造屋只做加工，不影响动物状态</span>
        </div>
        <div class="rule"></div>
        <section v-if="workshop" class="panel">
          <div class="panel-title">
            制造屋
            <span class="muted">加工坊放设备，各槽各跑各的；配方按设备分组</span>
          </div>
          <template v-if="!workshop.built">
            <p class="muted">建一座制造屋，就能把作物和牧场原料做成更值钱的东西。</p>
            <div class="row">
              <button
                class="link"
                type="button"
                :disabled="acting || !workshop.canBuild"
                @click="buildWorkshop"
              >
                [建制造屋 {{ workshop.cost }}]
              </button>
              <span class="muted">需要农场 {{ workshop.level }} 级、金币够</span>
            </div>
          </template>
          <template v-else>
            <div class="workshop-slots">
              <div v-for="slot in workshop.slotViews" :key="slot.index" class="workshop-slot">
                <div class="workshop-slot-head">
                  {{ slot.index }} 号槽 · {{ slot.equipmentEmoji }} {{ slot.equipmentLabel }}
                </div>
                <div v-if="slot.job" class="ranch-craft-active">
                  <span>
                    正在做 {{ slot.job.outputName }}
                    <span v-if="!slot.job.isCrop" class="muted">（{{ slot.job.qualityLabel }}）</span>
                  </span>
                  <button
                    class="link"
                    type="button"
                    :disabled="acting || !slot.job.canCollect"
                    @click="collectCraft(slot.index)"
                  >
                    {{ slot.job.canCollect ? "[取货]" : `还要 ${formatRemain(slot.job.remainingMs)}` }}
                  </button>
                </div>
                <div v-else class="muted">
                  {{ slot.index === 0 ? "手工台空闲" : slot.equipment ? "设备空闲" : "空槽，等装设备" }}
                </div>
              </div>
            </div>

            <div class="ranch-recipe-group">设备</div>
            <div class="equipment-options">
              <span
                v-for="option in workshop.equipmentOptions"
                :key="option.kind"
                class="equipment-option"
              >
                {{ option.emoji }} {{ option.label }} {{ option.cost }}
                <button
                  v-if="!option.installed"
                  class="link"
                  type="button"
                  :disabled="acting || !option.canInstall || emptyEquipmentSlot === null"
                  @click="installEquipment(emptyEquipmentSlot ?? 1, option.kind)"
                >
                  [装到 {{ emptyEquipmentSlot ?? "-" }} 号槽]
                </button>
                <span v-else class="muted">已装</span>
                <span v-if="option.missing && !option.installed" class="muted">{{ option.missing }}</span>
              </span>
            </div>
            <div v-if="workshop.canExpand" class="ranch-expand">
              <button class="link" type="button" :disabled="acting" @click="expandWorkshop">
                [扩加工槽 {{ workshop.expandCost }}]
              </button>
              <span class="muted">
                当前 {{ workshop.slots }} / {{ workshop.maxSlots }} 槽，需要农场
                {{ workshop.expandLevel }} 级
              </span>
            </div>

            <div class="ranch-recipe-group">手工台 · 牧场配方</div>
            <div class="ranch-recipes">
              <div v-for="recipe in workshop.manualRecipes" :key="recipe.recipe" class="ranch-recipe">
                <span class="ranch-recipe-text">
                  {{ recipe.label }}：{{ recipe.inputText }} → {{ recipe.outputName }}
                  <span class="muted">（{{ recipe.outputPrice }} 金币 · 约 {{ formatRemain(recipe.durationMs) }}）</span>
                </span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !recipe.canStart"
                  @click="startRanchCraft(recipe.recipe)"
                >
                  [开工]
                </button>
                <span v-if="recipe.missing" class="muted">{{ recipe.missing }}</span>
              </div>
            </div>
            <div class="ranch-recipe-group">
              手工台 · 作物礼盒
              <span class="muted">5 份同种作物 → 1 个礼盒，约为原料的 1.4 倍</span>
            </div>
            <div v-if="workshop.cropRecipes.length" class="ranch-recipes">
              <div v-for="crop in workshop.cropRecipes" :key="crop.cropId" class="ranch-recipe">
                <span class="ranch-recipe-text">
                  {{ crop.name }}×{{ crop.need }}（有 {{ crop.qty }}）→ {{ crop.outputName }}
                  <span class="muted">（{{ crop.outputPrice }} 金币 · 约 {{ formatRemain(crop.durationMs) }}）</span>
                </span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !crop.canStart"
                  @click="startCropCraft(crop.cropId)"
                >
                  [做礼盒]
                </button>
                <span v-if="crop.missing" class="muted">{{ crop.missing }}</span>
              </div>
            </div>
            <div v-else class="muted">仓库里没有作物；去农场收一点再来。</div>

            <div class="ranch-recipe-group">
              手工台 · 做饵
              <span class="muted">用作物/牧场产物做，产出普通品质，直接进鱼饵篓</span>
            </div>
            <div class="ranch-recipes">
              <div v-for="recipe in workshop.baitRecipes" :key="recipe.recipe" class="ranch-recipe">
                <span class="ranch-recipe-text">
                  {{ recipe.label }}：{{ recipe.inputText }} → {{ recipe.outputText }}
                  <span class="muted">（约 {{ formatRemain(recipe.durationMs) }}）</span>
                </span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !recipe.canStart"
                  @click="startBaitCraft(recipe.recipe)"
                >
                  [做饵]
                </button>
                <span v-if="recipe.missing" class="muted">{{ recipe.missing }}</span>
              </div>
            </div>

            <div class="ranch-recipe-group">
              手工台 · 鱼加工
              <span class="muted">吃任意未锁定鱼，按鱼价从低到高；鱼汤可当加成食物</span>
            </div>
            <div class="ranch-recipes">
              <div v-for="recipe in workshop.fishRecipes" :key="recipe.recipe" class="ranch-recipe">
                <span class="ranch-recipe-text">
                  {{ recipe.label }}：{{ recipe.inputText }} → {{ recipe.outputText }}
                  <span class="muted">（约 {{ formatRemain(recipe.durationMs) }}）</span>
                </span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !recipe.canStart"
                  @click="startFishCraft(recipe.recipe)"
                >
                  [加工]
                </button>
                <span v-if="recipe.missing" class="muted">{{ recipe.missing }}</span>
              </div>
            </div>

            <div class="ranch-recipe-group">设备配方</div>
            <div class="ranch-recipes">
              <div
                v-for="recipe in workshop.equipmentRecipes"
                :key="recipe.recipe"
                class="ranch-recipe"
              >
                <span class="ranch-recipe-text">
                  {{ recipe.equipmentLabel }} · {{ recipe.label }}：{{ recipe.inputText }} →
                  {{ recipe.outputName }}
                  <span class="muted">（{{ recipe.outputPrice }} 金币 · 约 {{ formatRemain(recipe.durationMs) }}）</span>
                </span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !recipe.canStart"
                  @click="startEquipmentCraft(recipe.recipe)"
                >
                  [开工]
                </button>
                <span v-if="recipe.missing" class="muted">{{ recipe.missing }}</span>
              </div>
            </div>
          </template>
        </section>
        <section class="panel">
          <div class="panel-title">
            原料与成品
            <span class="muted">加工品按品质分堆；品质跟着投入原料里最低的那一档</span>
          </div>
          <div v-if="workshopItems.length" class="ranch-products">
            <span v-for="item in workshopItems" :key="item.key" class="ranch-product">
              {{ item.name }}×{{ item.qty }}
              <span v-if="item.unitPrice" class="muted">每个 {{ item.unitPrice }}</span>
              <span v-else class="muted">加工用</span>
              <template v-if="item.sellable">
                <button class="link" type="button" :disabled="acting" @click="sell(item, false)">[卖1]</button>
                <button class="link" type="button" :disabled="acting" @click="sell(item, true)">[全卖]</button>
              </template>
            </span>
          </div>
          <div v-else class="muted">仓库里还没有牧场原料或作物，先去畜牧场/农场弄一点。</div>
          <div class="row">
            <button class="link" type="button" @click="selectTab('畜牧场')">[去畜牧场]</button>
            <span class="muted">可卖的东西都能在这里直接卖掉</span>
          </div>
        </section>
      </div>

      <div v-show="activeTab === '钓鱼'">
        <div class="level">{{ levelLine }}</div>
        <div class="sub">
          {{ currency }} {{ snapshot?.coins ?? 0 }}
          <span class="muted">
            · 鱼饵 {{ activeBaitInfo?.label ?? "普通" }} ×{{
              activeBaitInfo?.count ?? fishing?.bait ?? 0
            }} · {{ fishing?.rodLabel ?? "木竿" }}
          </span>
        </div>
        <div class="rule"></div>
        <section v-if="fishing" class="panel fishing-page">
          <div class="panel-title">
            钓鱼
            <span class="muted">先选钓点/鱼饵再抛竿；专精、钓具、自动和图鉴奖励都收进折叠</span>
          </div>

          <div class="fish-hero">
            <div class="fish-hero-main">
              <div class="fish-level-line">
                <b>钓鱼等级 Lv{{ fishing.level }}</b>
                <span class="muted">{{ fishing.expNeed ? `${fishing.exp}/${fishing.expNeed}` : "满级" }}</span>
                <span class="fish-points">技能点 {{ fishing.skillPoints }}</span>
              </div>
              <div class="fish-exp"><span :style="{ width: `${fishingExpPct}%` }"></span></div>
              <div class="fish-hero-meta">
                {{ currentSpot?.emoji }} {{ currentSpot?.label }} · {{ fishing.rodLabel }} ·
                {{ activeBaitTypeLabel }}{{ activeBaitInfo?.label }} ×{{ activeBaitInfo?.count ?? 0 }}
              </div>
            </div>
            <div class="fish-hero-side">
              <span class="fish-auto-pill" :class="{ on: fishing.auto.enabled }">
                自动 {{ fishing.auto.enabled ? "开着" : "关着" }}
              </span>
              <span class="muted">每 {{ fishing.autoIntervalLabel }}</span>
            </div>
          </div>

          <details class="fish-card fish-card--skill fish-fold">
            <summary>
              <span class="fish-fold-title">专精</span>
              <span class="muted">省饵 / 收线 / 巨物 / 自动化 · 技能点 {{ fishing.skillPoints }}</span>
            </summary>
            <div class="fishing-level">
            <div class="row">
              <span>钓鱼等级 <b>Lv{{ fishing.level }}</b>（{{ fishing.expNeed ? fishing.exp + "/" + fishing.expNeed : "满级" }}）</span>
              <span class="muted">技能点 {{ fishing.skillPoints }}</span>
            </div>
            <div class="fishing-skills">
              <div v-for="skill in fishing.skills" :key="skill.id" class="fishing-skill">
                <span class="skill-name">{{ skill.name }} Lv{{ skill.level }}/{{ skill.maxLevel }}</span>
                <span class="muted">{{ skill.desc }}</span>
                <button
                  v-if="skill.canUpgrade"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="upgradeFishingSkill(skill.id)"
                >
                  [升级 {{ skill.cost }} 点]
                </button>
                <span v-else-if="skill.level >= skill.maxLevel" class="ok">已满</span>
                <span v-else class="muted">需要 {{ skill.cost }} 点</span>
              </div>
            </div>
          </div>
          </details>

          <div class="fish-card fish-card--play">
          <div class="fishing-spots">
            <button
              v-for="spot in fishing.spots"
              :key="spot.spot"
              class="tab"
              type="button"
              :class="{ active: fishingSpot === spot.spot }"
              :disabled="!spot.unlocked"
              @click="fishingSpot = spot.spot"
            >
              {{ spot.emoji }} {{ spot.label }}
              <span v-if="!spot.unlocked" class="muted">{{ spot.level }}级</span>
              <span v-else class="muted">{{ spot.fishCount }}种</span>
            </button>
          </div>
          <div class="row">
            <span>
              鱼竿：{{ fishing.rodLabel }}（框 {{ fishing.framePct }}%）
              <span class="muted">磨损 {{ fishing.rodWear }}/{{ fishing.rodMaxWear }}</span>
            </span>
            <button
              v-if="fishing.needsMaintenance"
              class="link"
              type="button"
              :disabled="acting || (snapshot?.coins ?? 0) < fishing.maintenanceCost"
              @click="maintainRod(fishing.rod)"
            >
              [保养 {{ fishing.maintenanceCost }}]
            </button>
            <span>鱼饵</span>
            <select class="bait-select" v-model="activeBaitType" :disabled="!!fishing.session" @change="setBaitType">
              <option v-for="type in fishing.baitTypes" :key="type.baitType" :value="type.baitType">
                {{ type.label }}
              </option>
            </select>
            <select class="bait-select" v-model="activeBaitQuality" :disabled="!!fishing.session">
              <option v-for="bait in fishing.baitCounts" :key="bait.quality" :value="bait.quality">
                {{ bait.label }} · {{ bait.price }}金币/份
              </option>
            </select>
            <span>×{{ activeBaitInfo?.count ?? 0 }}</span>
            <button class="link" type="button" :disabled="acting" @click="buyBait(10)">
              [买10份 {{ (activeBaitInfo?.price ?? fishing.baitPrice) * 10 }}]
            </button>
            <span class="muted">自动：每 {{ fishing.autoIntervalLabel }} 一条</span>
          </div>

          <FishingGame
            v-if="fishing.session"
            :key="fishing.session.id"
            :session="fishing.session"
            @resolve="resolveFishing"
          />
          <div v-else class="row">
            <button
              class="link"
              type="button"
              :disabled="acting || !currentSpot?.unlocked || (activeBaitInfo?.count ?? 0) === 0"
              @click="castLine"
            >
              [抛竿 (F)]
            </button>
            <span v-if="(activeBaitInfo?.count ?? 0) === 0" class="muted">这款鱼饵没了，先买一点</span>
            <span v-else-if="currentSpot && !currentSpot.unlocked" class="muted">
              需要钓鱼等级 {{ currentSpot.level }} 级
            </span>
            <span v-else class="muted">F 抛竿 → 等咬钩 → 再按 F 开始；WASD 移框，进度满自动收鱼</span>
          </div>
          </div>

          <details class="fish-card fish-card--gear fish-fold">
            <summary>
              <span class="fish-fold-title">钓具</span>
              <span class="muted">
                {{ fishing.rodLabel }} · 磨损 {{ fishing.rodWear }}/{{ fishing.rodMaxWear }}
                <template v-if="fishing.needsMaintenance"> · 可保养 {{ fishing.maintenanceCost }}</template>
              </span>
            </summary>
            <div class="ranch-recipe-group">鱼竿</div>
          <div class="ranch-recipes">
            <div v-for="rod in fishing.rods" :key="rod.kind" class="ranch-recipe">
              <span class="ranch-recipe-text">
                {{ rod.label }}：框 {{ rod.framePct }}% · 自动 {{ rod.autoMinutes }} 分钟 · 磨损
                {{ rod.wear }}/{{ rod.maxWear }}
              </span>
              <span v-if="rod.current" class="muted">使用中</span>
              <span v-else-if="rod.owned" class="muted">已拥有</span>
              <span v-else-if="fishing.level < rod.level" class="muted">
                需要钓鱼等级 {{ rod.level }} 级
              </span>
              <button
                v-else
                class="link"
                type="button"
                :disabled="acting || !rod.canBuy"
                @click="buyRod(rod.kind)"
              >
                [换竿 {{ rod.cost }}]
              </button>
              <button
                v-if="rod.needsMaintenance"
                class="link"
                type="button"
                :disabled="acting || (snapshot?.coins ?? 0) < rod.maintenanceCost"
                @click="maintainRod(rod.kind)"
              >
                [保养 {{ rod.maintenanceCost }}]
              </button>
              <span v-if="!rod.owned && (snapshot?.level ?? 0) < rod.level" class="muted">
                需要 {{ rod.level }} 级
              </span>
            </div>
          </div>
          </details>

          <details class="fish-card fish-card--auto fish-fold">
            <summary>
              <span class="fish-fold-title">自动钓鱼</span>
              <b :class="{ on: fishing.auto.enabled }">{{ fishing.auto.enabled ? "开着" : "关着" }}</b>
              <span class="muted">
                {{ autoSpotInfo?.label }} · {{ fishing.auto.rodLabel }} ·
                {{ fishing.auto.baitTypeLabel }}{{ fishing.auto.baitQualityLabel }} · 每
                {{ fishing.auto.intervalLabel }} 一条
              </span>
              <button
                class="link fish-auto-toggle"
                type="button"
                :disabled="acting || (!fishing.auto.enabled && !autoSpotInfo?.unlocked)"
                @click.stop.prevent="setAutoFishing(!fishing.auto.enabled)"
              >
                {{ fishing.auto.enabled ? "[收自动]" : "[开自动]" }}
              </button>
            </summary>
          <div class="auto-settings">
            <label>
              钓点
              <select v-model="autoSpot" :disabled="fishing.auto.enabled">
                <option
                  v-for="spot in fishing.spots"
                  :key="spot.spot"
                  :value="spot.spot"
                  :disabled="!spot.unlocked"
                >
                  {{ spot.emoji }} {{ spot.label }}{{ spot.unlocked ? "" : `（${spot.level}级）` }}
                </option>
              </select>
            </label>
            <label>
              鱼竿
              <select v-model="autoRod" :disabled="fishing.auto.enabled">
                <option
                  v-for="rod in fishing.rods"
                  :key="rod.kind"
                  :value="rod.kind"
                  :disabled="!rod.owned"
                >
                  {{ rod.label }}（框 {{ rod.framePct }}% · {{ rod.autoMinutes }}分）
                </option>
              </select>
            </label>
            <label>
              饵料
              <select v-model="autoBaitType" :disabled="fishing.auto.enabled">
                <option v-for="type in fishing.baitTypes" :key="type.baitType" :value="type.baitType">
                  {{ type.label }}
                </option>
              </select>
            </label>
            <label>
              鱼饵
              <select v-model="autoBaitQuality" :disabled="fishing.auto.enabled">
                <option v-for="bait in fishing.baitCounts" :key="bait.quality" :value="bait.quality">
                  {{ bait.label }} · {{ bait.price }}金币/份
                </option>
              </select>
            </label>
          </div>
          <div class="row">
            <span v-if="fishing.auto.enabled">
              开着 · {{ fishing.auto.spotLabel }} · {{ fishing.auto.rodLabel }} ·
              {{ fishing.auto.baitTypeLabel }}{{ fishing.auto.baitQualityLabel }}鱼饵 · 每
              {{ fishing.auto.intervalLabel }} 一条
              <span class="muted">
                · 下一条约 {{ formatRemain(Math.max(0, fishing.auto.nextAt - (snapshot?.now ?? 0))) }}
              </span>
            </span>
            <span v-else class="muted">
              关着；开了以后离线也收鱼，最多 8 小时。鱼饵不足会自动买 10 份（花金币）。
            </span>
          </div>
          <div v-if="autoBaitInfo" class="muted">
            当前{{ autoBaitInfo.label }}鱼饵：品质 +{{ autoBaitInfo.qualityBonus }}，重量
            +{{ (autoBaitInfo.weightBonus / 10).toFixed(1) }}kg；还剩 {{ autoBaitInfo.count }} 份。
          </div>
          <div class="muted">
            自动只钓已点亮图鉴的鱼（鱼王除外），不会自动解锁新鱼；新号直接开自动会先点亮小溪初始鱼。
          </div>
          </details>

          <div class="fish-card fish-card--codex">
          <div class="fish-codex-head">
            <div class="fish-codex-title">
              鱼类图鉴 <b>{{ fishing.codexCaught }} / {{ fishing.codexTotal }}</b>
              <span class="muted">
                · 金色 {{ fishing.goldenCaught }}/{{ fishing.goldenTotal }} · 鱼王
                {{ kingCaught }}/{{ fishing.kingTrophies.length }}
                <span v-if="fishing.fullCodexClaimed">· 全图鉴奖励已领</span>
              </span>
            </div>
            <div class="fish-codex-bar"><span :style="{ width: `${codexPct}%` }"></span></div>
            <details class="fish-fold fish-fold--inline">
              <summary>分钓点进度与鱼王 trophy</summary>
          <div class="codex-progress">
            <div v-for="spot in fishing.spotProgress" :key="spot.spot" class="codex-progress-row">
              <b>{{ spot.label }}</b>
              <span>{{ spot.caught }}/{{ spot.total }}</span>
              <span class="muted">奖励 {{ spot.rewardCoins }} 金币 + {{ spot.rewardBaitLabel }}</span>
              <span v-if="spot.claimed" class="codex-tag">已领</span>
              <span v-else-if="spot.complete" class="codex-tag">下次钓获自动发</span>
              <span v-else class="muted">未集齐</span>
            </div>
          </div>
          <div class="codex-kings">
            <div v-for="king in fishing.kingTrophies" :key="king.fishId" class="king-trophy" :class="{ caught: king.caught }">
              <b>{{ king.caught ? king.name : "???" }}</b>
              <span class="muted">{{ king.spotLabel }}</span>
              <span v-if="king.caught" class="muted">
                ×{{ king.count }} · {{ (king.bestWeightTenths / 10).toFixed(1) }}kg
              </span>
            </div>
          </div>
          </details>
          </div>
          <div class="fish-codex">
            <div
              v-for="item in fishing.codex"
              :key="item.id"
              class="fish-codex-item"
              :class="{ caught: item.caught, king: item.rarity === 'king', golden: item.golden }"
              @click="selectedFish = item"
            >
              <img :src="`/fish/${item.id}.png`" :alt="item.name" />
              <span>{{ item.caught ? item.name : "???" }}</span>
              <span class="muted">{{ item.rarityLabel }} · {{ item.price }}</span>
              <span v-if="item.caught" class="muted">
                最大 {{ (item.bestWeightTenths / 10).toFixed(1) }}kg · {{ item.bestQualityLabel }}
              </span>
              <span v-if="item.caught" class="muted">×{{ item.count }}</span>
              <span v-if="item.golden" class="golden-badge">金色 ×{{ item.goldenCount }}</span>
            </div>
          </div>
          </div>

          <div class="fish-card fish-card--keep">
            <div class="fish-card-head">
              <b>鱼护</b>
              <span class="muted">共 {{ fishItems.length }} 种 · 锁定的不卖</span>
            </div>
          <div v-if="fishItems.length" class="ranch-products">
            <div class="row">
              <button
                class="link"
                type="button"
                :disabled="acting || !fishItems.some((item) => !item.locked)"
                @click="sellAllFish"
              >
                [一键全卖（跳过锁定）]
              </button>
            </div>
            <span v-for="item in fishItems" :key="item.key" class="ranch-product">
              {{ item.locked ? "🔒" : "" }}{{ item.name }}×{{ item.qty }}
              <span class="muted">每个 {{ item.unitPrice }}</span>
              <span v-if="item.detail" class="muted">{{ item.detail }}</span>
              <button class="link" type="button" :disabled="acting" @click="toggleFishLock(item)">
                {{ item.locked ? "[解锁]" : "[锁定]" }}
              </button>
              <button
                class="link"
                type="button"
                :disabled="acting || item.locked"
                @click="sell(item, false)"
              >
                [卖1]
              </button>
              <button
                class="link"
                type="button"
                :disabled="acting || item.locked"
                @click="sell(item, true)"
              >
                [全卖]
              </button>
            </span>
          </div>
          <div v-else class="muted">还没钓到鱼；去抛一竿吧。</div>
          </div>
        </section>
      </div>

      <div v-show="activeTab === '玩家'">
        <section class="panel">
          <div class="panel-title">
            玩家信息
            <span class="muted">四专业共用一套等级与专精；宠物技能在宠物页，不混在这里</span>
          </div>
          <div class="player-grid">
            <div v-for="d in disciplines" :key="d.kind" class="player-card">
              <div class="row">
                <b>{{ d.label }} Lv{{ d.level }}</b>
                <span class="muted">
                  {{ d.expNeed ? d.exp + "/" + d.expNeed : "满级" }} · 技能点 {{ d.points }}
                </span>
              </div>
              <div class="player-exp">
                <span :style="{ width: expPct(d) }"></span>
              </div>
              <div v-if="d.skills.length" class="fishing-skills">
                <div v-for="skill in d.skills" :key="skill.id" class="fishing-skill">
                  <span class="skill-name">{{ skill.name }} Lv{{ skill.level }}/{{ skill.maxLevel }}</span>
                  <span class="muted">{{ skill.desc }}</span>
                  <button
                    v-if="skill.canUpgrade"
                    class="link"
                    type="button"
                    :disabled="acting"
                    @click="upgradeSkill(d.kind, skill.id)"
                  >
                    [升级 {{ skill.cost }} 点]
                  </button>
                  <span v-else-if="skill.level >= skill.maxLevel" class="ok">已满</span>
                  <span v-else class="muted">需要 {{ skill.cost }} 点</span>
                </div>
              </div>
              <div v-else class="muted">这个专业还没有开放专精。</div>
            </div>
          </div>
        </section>
        <section class="panel">
          <div class="panel-title">临时加成</div>
          <div v-if="buffList.length" class="player-buffs">
            <div v-for="b in buffList" :key="b.id" class="row">
              <b>{{ b.name }}</b>
              <span class="muted">{{ b.desc }} · 剩 {{ formatRemain(b.remainingMs) }}</span>
            </div>
          </div>
          <div v-else class="muted">暂时没有临时加成。</div>
          <div class="rule"></div>
          <div class="row">
            <b>加成食物</b>
            <span class="muted">在仓库里，点「享用」就生效；同一种只刷新时间，不同种同项相加</span>
          </div>
          <div v-if="ownedBuffItems.length">
            <div v-for="item in ownedBuffItems" :key="item.key" class="row">
              <span>{{ item.name }}×{{ item.qty }}</span>
              <span class="muted">{{ item.desc }}</span>
              <button class="link" type="button" :disabled="acting" @click="useBuffItem(item.key)">
                [享用]
              </button>
            </div>
          </div>
          <div v-else class="muted">
            仓库里还没有加成食物（草莓汁/葡萄汁/西瓜汁/甘蔗汁/奶酪/蛋糕）。
          </div>
        </section>
      </div>

      <div v-show="activeTab === '宠物'">
        <section class="panel">
          <div class="panel-title">
            宠物
            <span class="muted">狗狗/猫猫先只显示静态像素侧视图</span>
          </div>

          <div class="boarding-panel">
            <div class="boarding-head">
              <strong>🏡 宠物院子</strong>
              <span class="muted">
                寄养 {{ boardedPets.length }} 只 · 月费合计 {{ boardingTotal }} · 下次扣费 {{ nextBoardingMonth }}-01
              </span>
            </div>
            <div v-if="!boardedPets.length" class="muted">目前没有寄养宠物。</div>
            <div v-else class="boarding-list">
              <div v-for="item in boardedPets" :key="item.id" class="boarding-card">
                <img :src="item.image" :alt="item.name" />
                <div>
                  <strong>{{ item.name }}</strong>
                  <span class="muted">{{ item.breedLabel }} · Lv{{ item.level }} · 每月 {{ item.boardingFee }}</span>
                </div>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || activePets.length >= 3"
                  @click="setPetBoarded(item.id, false)"
                >
                  [接回]
                </button>
                <button class="link" type="button" :disabled="acting" @click="releasePet(item.id, item.name)">
                  [放生]
                </button>
              </div>
            </div>
            <div v-if="snapshot?.boardingDebt" class="warn boarding-debt">
              欠费 {{ snapshot.boardingDebt }} 金币
              <button class="link" type="button" :disabled="acting" @click="settleBoardingDebt">
                [手动补缴]
              </button>
            </div>
          </div>

          <div v-if="!pets.length || showAdopt" class="pet-adopt">
            <img class="pet-portrait-img" :src="selectedBreed.image" :alt="selectedBreed.label" />
            <div class="pet-adopt-form">
              <p>领养一只小伙伴。它会陪你种地、挖稀有种子，还会帮你看着农场。</p>
              <p v-if="activePets.length >= 3" class="warn">
                已同时养 3 只，继续领养只能放进宠物院子寄养（每月 3,000/只）。
              </p>
              <div class="row">
                <input v-model="petName" maxlength="12" placeholder="给它起个名字" />
                <button
                  type="button"
                  :class="{ active: petSpecies === 'dog' }"
                  @click="petBreed = 'corgi'"
                >
                  🐶 狗狗
                </button>
                <button
                  type="button"
                  :class="{ active: petSpecies === 'cat' }"
                  @click="petBreed = 'domestic_cat'"
                >
                  🐱 猫猫
                </button>
              </div>
              <div class="breed-grid">
                <button
                  v-for="breed in visibleBreeds"
                  :key="breed.id"
                  type="button"
                  class="breed-card"
                  :class="{ active: petBreed === breed.id }"
                  @click="petBreed = breed.id"
                >
                  <img :src="breed.image" :alt="breed.label" />
                  <span>{{ breed.label }}</span>
                </button>
              </div>
              <div class="row">
                <span class="muted">已选：{{ selectedBreed.label }}</span>
                <label v-if="activePets.length >= 3" class="muted">
                  <input v-model="adoptBoard" type="checkbox" checked disabled />
                  只能寄养领养
                </label>
                <label v-else class="muted">
                  <input v-model="adoptBoard" type="checkbox" />
                  放到宠物院子寄养
                </label>
                <button type="button" :disabled="acting || !petName.trim()" @click="adoptPet">
                  [{{ adoptBoard || activePets.length >= 3 ? "寄养领养" : "领养" }} {{ selectedBreed.label }}]
                </button>
                <button v-if="pets.length" class="link" type="button" @click="showAdopt = false">
                  [取消]
                </button>
              </div>
              <p class="muted">不会死亡；生病可治疗。先只开放农场，牧场/鱼塘/钓鱼地留给以后。</p>
            </div>
          </div>

          <div v-else>
            <div v-if="snapshot?.boardingDebt" class="warn">
              宠物院子欠费 {{ snapshot.boardingDebt }} 金币，下月会自动补扣。
            </div>
            <div class="pet-selector">
              <button
                v-for="item in pets"
                :key="item.id"
                type="button"
                class="pet-select-card"
                :class="{ active: pet?.id === item.id }"
                @click="selectedPetId = item.id"
              >
                <img :src="item.image" :alt="item.name" />
                <span>
                  {{ item.name }}
                  <small v-if="item.boarded">寄养</small>
                </span>
              </button>
              <button class="pet-select-add" type="button" @click="showAdopt = true">
                + 领养
              </button>
            </div>

            <div v-if="pet" class="pet-layout">
            <div class="pet-portrait">
              <img class="pet-portrait-img" :src="pet.image" :alt="pet.species" />
              <div class="pet-name">{{ pet.speciesEmoji }} {{ pet.name }}</div>
              <div class="muted">
                {{ pet.breedLabel }} · {{ pet.temperamentLabel }} · {{ pet.stageLabel }} · Lv{{ pet.level }} ·
                第 {{ pet.ageDays }} 天 · {{ pet.moodLabel }}
              </div>
              <div class="pet-personality">
                个性：{{ pet.personality }} · 小癖好：{{ pet.quirk }}
                <span class="muted">{{ pet.quirkLine }}</span>
              </div>
              <div class="pet-skill">
                <strong>品种技能 · {{ pet.skill }}</strong>
                <span class="muted">{{ pet.skillDescription }}</span>
              </div>
              <div class="pet-activity">
                正在：{{ pet.activityLabel }}
                <span class="muted">{{ pet.activityLine }}</span>
                <span v-if="pet.activityStreak >= 2" class="muted">连续 {{ pet.activityStreak }} 天</span>
              </div>
              <div class="pet-behavior">“{{ pet.behavior }}”</div>
              <div v-if="pet.location === 'ranch'" class="pet-job">
                牧场岗位 Lv{{ pet.jobLevel }}
                <span class="muted">（{{ pet.jobExp }}/{{ pet.jobNeed }}）· 每天结算一次看护收益</span>
              </div>
              <div v-else class="pet-job muted">牧场岗位 Lv{{ pet.jobLevel }} · 派去牧场后每天有收益</div>
            </div>
            <div class="pet-info">
              <div v-if="pet.boarded" class="warn">
                {{ pet.name }}在宠物院子寄养中，每月自动扣 {{ pet.boardingFee }} 金币。
                <button class="link" type="button" :disabled="acting || activePets.length >= 3" @click="setPetBoarded(pet.id, false)">
                  [接回]
                </button>
                <button class="link" type="button" :disabled="acting" @click="releasePet(pet.id, pet.name)">
                  [放生]
                </button>
              </div>
              <div v-if="pet.sickness" class="warn">
                {{ pet.name }}生病了：{{ pet.sickness }}
                <button class="link" type="button" :disabled="acting" @click="petAction('healPet')">
                  [看兽医 500]
                </button>
                <button
                  v-if="petCare && petCare.medicine > 0"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="usePetMedicine"
                >
                  [用药品 ×{{ petCare.medicine }}]
                </button>
              </div>
              <div class="pet-bars">
                <div class="pet-bar">
                  <span>饱食 {{ pet.satiety }}</span>
                  <i><b :style="{ width: `${pet.satiety}%` }"></b></i>
                </div>
                <div class="pet-bar">
                  <span>精力 {{ pet.energy }}</span>
                  <i><b :style="{ width: `${pet.energy}%` }"></b></i>
                </div>
                <div class="pet-bar">
                  <span>心情 {{ pet.mood }}</span>
                  <i><b :style="{ width: `${pet.mood}%` }"></b></i>
                </div>
                <div class="pet-bar">
                  <span>亲密 {{ pet.bond }}</span>
                  <i><b :style="{ width: `${pet.bond}%` }"></b></i>
                </div>
                <div class="pet-bar">
                  <span>健康 {{ pet.health }}</span>
                  <i><b :style="{ width: `${pet.health}%` }"></b></i>
                </div>
              </div>
              <div class="muted">经验 {{ pet.exp }} / {{ pet.expNeed }} · 地点 {{ pet.locationLabel }}</div>
              <div class="pet-skills">
                <div class="pet-bar">
                  <span>挖掘 Lv{{ pet.digSkill }}</span>
                  <i><b :style="{ width: `${Math.min(100, (pet.digExp / Math.max(1, pet.digNeed)) * 100)}%` }"></b></i>
                  <small>{{ pet.digExp }}/{{ pet.digNeed }}</small>
                </div>
                <div class="pet-bar">
                  <span>守卫 Lv{{ pet.guardSkill }}</span>
                  <i><b :style="{ width: `${Math.min(100, (pet.guardExp / Math.max(1, pet.guardNeed)) * 100)}%` }"></b></i>
                  <small>{{ pet.guardExp }}/{{ pet.guardNeed }}</small>
                </div>
              </div>

              <div v-if="!pet.boarded" class="row">
                <select v-model="feedCropId">
                  <option v-for="item in produce" :key="item.key" :value="item.key.replace('crop:', '')">
                    {{ item.name }} ×{{ item.qty }}
                  </option>
                </select>
                <button class="link" type="button" :disabled="acting || !produce.length" @click="feedPet">
                  [喂食]
                </button>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !pet.canTouch"
                  @click="petAction('petCompanion')"
                >
                  [摸摸]
                  <span v-if="!pet.canTouch" class="muted">{{ formatCooldown(pet.touchRemainingMs) }}</span>
                </button>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !pet.canPlay"
                  @click="petAction('playWithPet')"
                >
                  [玩耍]
                  <span v-if="!pet.canPlay && pet.playRemainingMs > 0" class="muted">
                    {{ formatCooldown(pet.playRemainingMs) }}
                  </span>
                </button>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !pet.canDig"
                  @click="petAction('digWithPet')"
                >
                  [挖种子]
                  <span v-if="!pet.canDig && pet.digRemainingMs > 0" class="muted">
                    {{ formatCooldown(pet.digRemainingMs) }}
                  </span>
                </button>
                <span v-if="pet.digRemainingMs === 0 && pet.location === 'farm'" class="muted">
                  挖种子已就绪
                </span>
              </div>
              <div v-if="!pet.boarded" class="row">
                <button
                  class="link"
                  type="button"
                  :disabled="acting || !pet.canUseSkill"
                  @click="usePetSkill"
                >
                  [主动 · {{ pet.activeSkill }}]
                  <span v-if="!pet.canUseSkill && pet.skillRemainingMs > 0" class="muted">
                    {{ formatCooldown(pet.skillRemainingMs) }}
                  </span>
                </button>
                <span class="muted">{{ pet.activeDescription }}</span>
              </div>

              <div v-if="!pet.boarded" class="row">
                <span class="muted">地点：</span>
                <button class="rate-btn" :class="{ active: pet.location === 'farm' }" type="button" :disabled="acting" @click="setPetLocation(pet.id, 'farm')">农场</button>
                <button class="rate-btn" :class="{ active: pet.location === 'ranch' }" type="button" :disabled="acting" @click="setPetLocation(pet.id, 'ranch')">牧场</button>
                <button class="rate-btn" type="button" disabled>鱼塘（以后）</button>
                <button class="rate-btn" type="button" disabled>钓鱼地（以后）</button>
              </div>

              <div class="row">
                <input v-model="renameName" maxlength="12" placeholder="改个名字" />
                <button class="link" type="button" :disabled="acting || !renameName.trim()" @click="renamePet">
                  [改名]
                </button>
              </div>
              <div v-if="!pet.boarded" class="row">
                <button class="link" type="button" :disabled="acting" @click="setPetBoarded(pet.id, true)">
                  [送去寄养 {{ pet.boardingFee }}/月]
                </button>
                <button class="link" type="button" :disabled="acting" @click="releasePet(pet.id, pet.name)">
                  [放生]
                </button>
              </div>
              <p class="muted">
                挖到的种子直接进仓库；守卫会减少偷菜损失，狗狗有低概率把偷菜贼吓跑。
              </p>
            </div>
          </div>
          </div>

          <div v-if="petCare" class="pet-care">
            <div class="pet-care-title">宠物用品</div>
            <div class="pet-care-grid">
              <div class="pet-care-card">
                <strong>🛏️ 狗窝</strong>
                <span>{{ petCare.dogBed ? (petCare.dogBedBroken ? "已损坏" : "已购买") : `${petCare.dogBedCost} 金币` }}</span>
                <span class="muted">狗狗离线精力 +50%</span>
                <button v-if="!petCare.dogBed" class="link" type="button" :disabled="acting" @click="buyPetSupply('dog_bed')">
                  [购买]
                </button>
                <button
                  v-else-if="petCare.dogBedBroken"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="repairPetSupply('dog_bed')"
                >
                  [修补 {{ petCare.dogBedRepair }}]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>🛏️ 猫窝</strong>
                <span>{{ petCare.catBed ? (petCare.catBedBroken ? "已损坏" : "已购买") : `${petCare.catBedCost} 金币` }}</span>
                <span class="muted">猫猫离线精力 +50%</span>
                <button v-if="!petCare.catBed" class="link" type="button" :disabled="acting" @click="buyPetSupply('cat_bed')">
                  [购买]
                </button>
                <button
                  v-else-if="petCare.catBedBroken"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="repairPetSupply('cat_bed')"
                >
                  [修补 {{ petCare.catBedRepair }}]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>🥣 食盆</strong>
                <span>{{ petCare.bowls }}/3</span>
                <span class="muted">当值宠物比食盆多会抢食打架</span>
                <button v-if="petCare.bowls < 3" class="link" type="button" :disabled="acting" @click="buyPetSupply('bowl')">
                  [购买 {{ petCare.bowlCost }}/个]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>🧸 狗狗玩具</strong>
                <span>{{ petCare.dogToy ? (petCare.dogToyBroken ? "已损坏" : "已购买") : `${petCare.dogToyCost} 金币` }}</span>
                <span class="muted">狗狗玩耍心情 +25%</span>
                <button v-if="!petCare.dogToy" class="link" type="button" :disabled="acting" @click="buyPetSupply('dog_toy')">
                  [购买]
                </button>
                <button
                  v-else-if="petCare.dogToyBroken"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="repairPetSupply('dog_toy')"
                >
                  [修补 {{ petCare.dogToyRepair }}]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>🧶 猫猫玩具</strong>
                <span>{{ petCare.catToy ? (petCare.catToyBroken ? "已损坏" : "已购买") : `${petCare.catToyCost} 金币` }}</span>
                <span class="muted">猫猫玩耍心情 +25%</span>
                <button v-if="!petCare.catToy" class="link" type="button" :disabled="acting" @click="buyPetSupply('cat_toy')">
                  [购买]
                </button>
                <button
                  v-else-if="petCare.catToyBroken"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="repairPetSupply('cat_toy')"
                >
                  [修补 {{ petCare.catToyRepair }}]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>💊 药品</strong>
                <span>已有 {{ petCare.medicine }} 个</span>
                <span class="muted">生病时直接用药，不用花 500 看兽医</span>
                <button class="link" type="button" :disabled="acting" @click="buyPetSupply('medicine')">
                  [购买 {{ petCare.medicineCost }}]
                </button>
              </div>
            </div>
            <p class="muted">狗窝/猫窝/玩具按物种生效；食盆全档共用，3 只宠物需要 3 个。拆家会咬坏窝/玩具，修补费为原价 40%。</p>
          </div>

          <div v-if="pet && pet.trainingCenter" class="pet-training">
            <div class="pet-care-title">
              训练中心
              <span class="muted">训练点 {{ pet.trainingSpent }}/{{ pet.trainingCap }}，总计上限 6，不能全点满</span>
            </div>
            <div class="pet-training-grid">
              <div class="pet-care-card">
                <strong>🛡️ 守护 Lv{{ pet.guardTrain }}/5</strong>
                <span class="muted">每级吓跑概率 +3%</span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || pet.trainingSpent >= pet.trainingCap || pet.guardTrain >= 5"
                  @click="trainPet('guard')"
                >
                  [训练]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>👃 挖掘嗅探 Lv{{ pet.digTrain }}/5</strong>
                <span class="muted">每级挖种加成 +3%，更容易双倍</span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || pet.trainingSpent >= pet.trainingCap || pet.digTrain >= 5"
                  @click="trainPet('dig')"
                >
                  [训练]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>🧹 闯祸拆家 Lv{{ pet.mischiefTrain }}/5</strong>
                <span class="muted">每级拆家概率 -1%，最低 0</span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || pet.trainingSpent >= pet.trainingCap || pet.mischiefTrain >= 5"
                  @click="trainPet('mischief')"
                >
                  [训练]
                </button>
              </div>
              <div class="pet-care-card">
                <strong>🤝 陪伴 Lv{{ pet.companionTrain }}/5</strong>
                <span class="muted">每级摸摸/玩耍心情收益 +5%</span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || pet.trainingSpent >= pet.trainingCap || pet.companionTrain >= 5"
                  @click="trainPet('companion')"
                >
                  [训练]
                </button>
              </div>
            </div>
            <p class="muted">训练点按宠物等级增长（每 5 级 1 点，最多 6 点）；每项最多 5 级，所以无法六边形全满。</p>
          </div>
          <div v-else-if="pet" class="pet-training">
            <div class="pet-care-title">训练中心</div>
            <p class="muted">农场 12 级可建设训练中心（20,000 金币），之后才能训练守护/挖掘/拆家/陪伴。</p>
          </div>
        </section>
      </div>

      <div v-show="activeTab === '标本室'">
        <section class="panel">
          <div class="panel-title">
            标本室
            <span class="muted">消耗作物和金币，随机换稀有种子</span>
          </div>
          <div class="specimen-grid">
            <div v-for="specimen in specimens" :key="specimen.cropId" class="specimen-card">
              <div class="specimen-head">
                <strong>{{ specimen.icon }} {{ specimen.name }}标本</strong>
                <span class="muted">{{ specimen.level }}级作物</span>
              </div>
              <div>
                材料：{{ specimen.name }}×{{ specimen.needQty }}（有 {{ specimen.haveQty }}）
                + {{ specimen.coins }}金币
              </div>
              <div class="muted">
                奖励：{{ specimen.rewardHint }} + {{ Math.floor(specimen.coins / 2) }}金币
              </div>
              <button
                v-if="specimen.canCraft"
                class="link"
                type="button"
                :disabled="acting"
                @click="craftSpecimen(specimen)"
              >
                [制作标本]
              </button>
              <span v-else class="muted">[材料或金币不足]</span>
            </div>
          </div>
        </section>
      </div>

      <div class="rule"></div>
      <h2 class="section-title">
        牧场广播
        <span class="muted">最新 10 条</span>
        <button class="link" type="button" @click="placeholder('我要广播')">我要广播&gt;&gt;</button>
      </h2>
      <div class="log">
        <div v-for="entry in logLines" :key="`${entry.at}-${entry.text}`" class="log-line">
          ({{ entry.stamp }}) {{ entry.text }}
        </div>
        <div v-if="!logLines.length" class="muted">还没有广播。</div>
      </div>
    </main>

    <footer>
      <div class="footer-links">包裹 | 仓库 | 商城 | 图鉴 | 成就 | 设置 | 说明</div>
      <div class="footer-links">我的农场 | 邻居留言 | 签到</div>
    </footer>

    <div v-if="selectedFish" class="fish-modal-overlay" @click.self="selectedFish = null">
      <div class="fish-modal">
        <img
          class="fish-modal-img"
          :class="{ unknown: !selectedFish.caught }"
          :src="`/fish/${selectedFish.id}.png`"
          :alt="selectedFish.name"
        />
        <div class="fish-modal-name">{{ selectedFish.caught ? selectedFish.name : "???" }}</div>
        <div class="muted">
          {{ selectedFish.rarityLabel }} · {{ selectedFish.spotLabel }} · 基础价
          {{ selectedFish.price }}
        </div>
        <div v-if="selectedFish.caught" class="muted">
          金色变异：{{ selectedFish.golden ? `已记录 ×${selectedFish.goldenCount}` : "未记录" }}
        </div>
        <div class="fish-source">主动钓鱼</div>
        <div class="fish-stats">
          <div>
            <b>{{ selectedFish.caught ? selectedFish.manualCount : "—" }}</b>
            <span>累计钓到</span>
          </div>
          <div>
            <b>
              {{
                selectedFish.caught
                  ? `${(selectedFish.manualBestWeightTenths / 10).toFixed(1)}kg`
                  : "—"
              }}
            </b>
            <span>最大重量</span>
          </div>
          <div>
            <b>{{ selectedFish.caught ? selectedFish.manualBestPrice : "—" }}</b>
            <span>最高售价</span>
          </div>
          <div>
            <b>{{ selectedFish.caught ? selectedFish.manualBestQualityLabel : "—" }}</b>
            <span>最高品质</span>
          </div>
        </div>

        <div class="fish-source">自动钓鱼</div>
        <div class="fish-stats">
          <div>
            <b>{{ selectedFish.autoCount }}</b>
            <span>累计钓到</span>
          </div>
          <div>
            <b>
              {{
                selectedFish.autoCount
                  ? `${(selectedFish.autoBestWeightTenths / 10).toFixed(1)}kg`
                  : "—"
              }}
            </b>
            <span>最大重量</span>
          </div>
          <div>
            <b>{{ selectedFish.autoCount ? selectedFish.autoBestPrice : "—" }}</b>
            <span>最高售价</span>
          </div>
          <div>
            <b>{{ selectedFish.autoCount ? selectedFish.autoBestQualityLabel : "—" }}</b>
            <span>最高品质</span>
          </div>
        </div>
        <button class="link" type="button" @click="selectedFish = null">[关闭]</button>
      </div>
    </div>

    <div
      v-if="autoReportVisible && fishing?.lastAutoReport"
      class="auto-report-overlay"
      @click.self="dismissAutoReport"
    >
      <div class="auto-report-card">
        <div class="catch-kicker">自动钓鱼报告</div>
        <div class="auto-report-title">收上来 {{ fishing.lastAutoReport.totalCaught }} 条鱼</div>
        <div class="fish-stats">
          <div><b>{{ fishing.lastAutoReport.baitBought }}</b><span>买了鱼饵</span></div>
          <div><b>{{ fishing.lastAutoReport.baitUsed }}</b><span>消耗鱼饵</span></div>
          <div><b>{{ fishing.lastAutoReport.coinsSpent }}</b><span>花费金币</span></div>
          <div><b>{{ fishing.lastAutoReport.totalCaught }}</b><span>渔获条数</span></div>
        </div>
        <div class="auto-report-catches">
          <span v-for="item in fishing.lastAutoReport.catches" :key="item.fishId">
            {{ item.name }}×{{ item.count }}
          </span>
        </div>
        <button class="link" type="button" @click="dismissAutoReport">[知道了]</button>
      </div>
    </div>

    <div v-if="catchResult" class="catch-overlay" @click.self="dismissCatch">
      <div class="catch-card" :class="{ king: catchResult.isKing, golden: catchResult.golden }">
        <div class="catch-glow"></div>
        <div class="catch-kicker">
          {{ catchResult.golden ? "金色变异" : catchResult.isKing ? "鱼王上钩" : "钓到了" }}
        </div>
        <img
          class="catch-fish"
          :src="`/fish/${catchResult.fishId}.png`"
          :alt="catchResult.name"
        />
        <div class="catch-name">{{ catchResult.name }}</div>
        <div class="catch-metrics">
          <span>重量 <b>{{ (catchResult.weightTenths / 10).toFixed(1) }} kg</b></span>
          <span>品质 <b :class="`q-${catchResult.quality}`">{{ catchResult.qualityLabel }}</b></span>
        </div>
        <div v-if="catchResult.isRecord" class="catch-badge">新纪录</div>
        <div v-if="catchResult.golden" class="catch-badge golden">金色变异</div>
        <div class="catch-price">
          {{ catchResult.price }} 金币
          <span class="muted">市场 {{ catchResult.marketPct }}%</span>
        </div>
        <button class="link" type="button" @click="dismissCatch">[收下]</button>
      </div>
    </div>
  </div>
</template>
