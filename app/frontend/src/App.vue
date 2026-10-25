<script setup lang="ts">
import { open as openFileDialog, save as saveFileDialog } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
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
  previewRanchBreeding,
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
  Effect,
  EquipmentKind,
  EquipmentRecipe,
  FacilityView,
  FishCatchView,
  FishCodexView,
  FishRecipe,
  FishingSpot,
  ItemStack,
  NpcTalkEffect,
  NpcView,
  PetBreed,
  PetFoodKind,
  PetLocation,
  PetTalent,
  PlotView,
  RanchAnimalKind,
  RanchBreedPreview,
  RanchRecipe,
  Region,
  RodKind,
  RoomLayout,
  Season,
  Snapshot,
  SpecimenView,
} from "./types";
import FishingGame from "./FishingGame.vue";
import RoomScene from "./RoomScene.vue";
import {
  applyBackgroundPrefs,
  BACKGROUND_FITS,  BACKGROUND_MODES,
  backgroundClasses,
  clampPercent,
  DEFAULT_BACKGROUND_PREFS,
  imageFitStyle,
  PAPER_PATTERNS,
  readBackgroundPrefs,
  writeBackgroundPrefs,
  type BackgroundFit,
  type BackgroundMode,
  type BackgroundPrefs,
  type PaperPattern,
} from "./background";
import {
  compressImage,
  deleteBackgroundImage,
  loadBackgroundImage,
  saveBackgroundImage,
} from "./background-store";
import {
  applyFontPrefs,
  CUSTOM_FONT_FAMILY,
  DEFAULT_FONT_PRESET,
  FONT_PRESETS,
  readFontPrefs,
  writeFontPrefs,
  type FontChoiceId,
  type FontPrefs,
  type FontPresetId,
} from "./font";
import {
  createCustomFontFace,
  deleteCustomFont,
  FONT_FILE_ACCEPT,
  loadCustomFont,
  saveCustomFont,
  type StoredFont,
} from "./font-store";
import {
  bufferToDataUrl,
  customAssetsDir,
  dataUrlToBuffer,
  loadPlayerAssets,
  readCustomAsset,
  saveCustomAsset,
  savePlayerAssets,
} from "./custom-assets";
import {
  applyStylePack,
  readStylePack,
  STYLE_PACKS,
  stylePackById,
  writeStylePack,
} from "./style-pack";
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
const seedPickerPlot = ref<number | null>(null);
const ranchDetail = ref(0);
const ranchBreedSel = ref<number[]>([]);
const ranchPreview = ref<RanchBreedPreview | null>(null);
const ranchRenameDrafts = ref<Record<number, string>>({});
const playerMode = ref(false);
const PLAYER_AVATAR_KEY = "farm.ui.playerAvatar";
const PLAYER_AVATAR_IMAGE_KEY = "farm.ui.playerAvatarImage";
const playerAvatar = ref(window.localStorage.getItem(PLAYER_AVATAR_KEY) || "🧑‍🌾");
const playerAvatarImage = ref(window.localStorage.getItem(PLAYER_AVATAR_IMAGE_KEY) || "");
const playerName = ref("");
const customDir = ref("");
const avatarChoices = ["🧑‍🌾", "👩‍🌾", "🐱", "🐶", "🐰", "🌻", "🍓", "🚜"];
const farmDays = computed(() => {
  const s = snapshot.value;
  if (!s || !s.createdAt || s.createdAt <= 0) return 1;
  return Math.max(1, Math.floor((s.now - s.createdAt) / 86_400_000) + 1);
});
watch(
  () => snapshot.value?.ownerName,
  (name) => {
    if (name && !playerName.value) playerName.value = name;
  },
  { immediate: true },
);
const debugTime = ref(window.localStorage.getItem("farm.debugTime") === "1");
const initialTheme = readThemePrefs(window.localStorage);
const themeMode = ref<ThemeMode>(initialTheme.mode);
const paletteId = ref<PaletteId>(initialTheme.palette);
const initialStylePack = readStylePack(window.localStorage);
const stylePackId = ref(initialStylePack.id);
const stylePack = computed(() => stylePackById(stylePackId.value));
const classicLayout = computed(() => stylePack.value.layout === "classic");
const initialBackground = readBackgroundPrefs(window.localStorage);
const bgMode = ref<BackgroundMode>(initialBackground.mode);
const bgPaper = ref<PaperPattern>(initialBackground.paper);
const bgFit = ref<BackgroundFit>(initialBackground.fit);
const bgImageAlpha = ref(initialBackground.imageAlpha);
const panelAlpha = ref(initialBackground.panelAlpha);
const backgroundImage = ref<string | null>(null);
const bgImageBusy = ref(false);
const bgImageError = ref("");
const initialFont = readFontPrefs(window.localStorage);
const fontId = ref<FontChoiceId>(initialFont.id);
const fontName = ref(initialFont.name);
const fontBusy = ref(false);
const fontError = ref("");
let customFontFace: FontFace | null = null;
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
  { icon: "🎣", name: "钓鱼" },
  { icon: "🛒", name: "神秘商店" },
];

const tabs = ["我的农场", "牧村", "邻居农场", "宠物", "鱼塘", "标本室", "图鉴"];

const levelLine = computed(() => {
  const s = snapshot.value;
  return s ? `农场等级:${s.level}(${s.exp}/${s.expNeed})` : "农场等级:--";
});

const currency = computed(() => content.value?.nouns.currency ?? "金币");

const produce = computed(() =>
  (snapshot.value?.inventory ?? []).filter((item) => item.kind === "produce"),
);

const ranch = computed(() => snapshot.value?.ranch ?? null);
const pond = computed(() => snapshot.value?.pond ?? null);
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
const workshopActiveJobs = computed(
  () => workshop.value?.slotViews.filter((slot) => slot.job).length ?? 0,
);
const workshopCollectable = computed(
  () => workshop.value?.slotViews.filter((slot) => slot.job?.canCollect).length ?? 0,
);
const workshopInstalled = computed(
  () =>
    workshop.value?.slotViews.filter((slot) => slot.index > 0 && slot.equipment).length ?? 0,
);
const onlyReadyRecipes = ref(true);
const recipeGroup = ref("ranch");
const showAllEquipment = ref(false);

interface WsRecipeRow {
  key: string;
  label: string;
  inputText: string;
  outputName: string;
  price: number;
  durationMs: number;
  canStart: boolean;
  missing: string;
  start: () => void;
}

interface WsRecipeGroup {
  key: string;
  label: string;
  rows: WsRecipeRow[];
  ready: number;
  total: number;
}

const wsRecipeGroups = computed<WsRecipeGroup[]>(() => {
  const ws = workshop.value;
  if (!ws) return [];
  const ranch: WsRecipeRow[] = ws.manualRecipes.map((recipe) => ({
    key: `ranch:${recipe.recipe}`,
    label: recipe.label,
    inputText: recipe.inputText,
    outputName: recipe.outputName,
    price: recipe.outputPrice,
    durationMs: recipe.durationMs,
    canStart: recipe.canStart,
    missing: recipe.missing,
    start: () => startRanchCraft(recipe.recipe),
  }));
  const crop: WsRecipeRow[] = ws.cropRecipes.map((recipe) => ({
    key: `crop:${recipe.cropId}`,
    label: "作物礼盒",
    inputText: `${recipe.name}×${recipe.need}（有 ${recipe.qty}）`,
    outputName: recipe.outputName,
    price: recipe.outputPrice,
    durationMs: recipe.durationMs,
    canStart: recipe.canStart,
    missing: recipe.missing,
    start: () => startCropCraft(recipe.cropId),
  }));
  const bait: WsRecipeRow[] = ws.baitRecipes.map((recipe) => ({
    key: `bait:${recipe.recipe}`,
    label: recipe.label,
    inputText: recipe.inputText,
    outputName: recipe.outputText,
    price: 0,
    durationMs: recipe.durationMs,
    canStart: recipe.canStart,
    missing: recipe.missing,
    start: () => startBaitCraft(recipe.recipe),
  }));
  const fish: WsRecipeRow[] = ws.fishRecipes.map((recipe) => ({
    key: `fish:${recipe.recipe}`,
    label: recipe.label,
    inputText: recipe.inputText,
    outputName: recipe.outputText,
    price: 0,
    durationMs: recipe.durationMs,
    canStart: recipe.canStart,
    missing: recipe.missing,
    start: () => startFishCraft(recipe.recipe),
  }));
  const equipment: WsRecipeRow[] = ws.equipmentRecipes.map((recipe) => ({
    key: `equip:${recipe.recipe}`,
    label: `${recipe.equipmentLabel} · ${recipe.label}`,
    inputText: recipe.inputText,
    outputName: recipe.outputName,
    price: recipe.outputPrice,
    durationMs: recipe.durationMs,
    canStart: recipe.canStart,
    missing: recipe.missing,
    start: () => startEquipmentCraft(recipe.recipe),
  }));
  return [
    ["ranch", "牧场配方", ranch],
    ["crop", "作物礼盒", crop],
    ["bait", "做饵", bait],
    ["fish", "鱼加工", fish],
    ["equipment", "设备配方", equipment],
  ].map(([key, label, rows]) => {
    const list = rows as WsRecipeRow[];
    return {
      key: key as string,
      label: label as string,
      rows: list,
      ready: list.filter((row) => row.canStart).length,
      total: list.length,
    };
  });
});
const recipeTotal = computed(() =>
  wsRecipeGroups.value.reduce((sum, group) => sum + group.total, 0),
);
const readyRecipeTotal = computed(() =>
  wsRecipeGroups.value.reduce((sum, group) => sum + group.ready, 0),
);
const visibleRecipeGroups = computed(() =>
  wsRecipeGroups.value.map((group) => ({
    ...group,
    rows: onlyReadyRecipes.value
      ? group.rows.filter((row) => row.canStart)
      : group.rows,
  })),
);
const allReadyRecipes = computed(() =>
  wsRecipeGroups.value
    .flatMap((group) => group.rows)
    .filter((row) => row.canStart)
    .sort((a, b) => b.price - a.price),
);
const topRecipe = computed(() => allReadyRecipes.value[0] ?? null);
const firstCollectable = computed(
  () => workshop.value?.slotViews.find((slot) => slot.job?.canCollect) ?? null,
);
const nextUnlock = computed(() => {
  const ws = workshop.value;
  if (!ws) return "";
  const equipment = ws.equipmentOptions.find((option) => !option.installed);
  if (equipment) {
    return `${equipment.emoji} ${equipment.label} ${equipment.cost}${equipment.missing ? `（${equipment.missing}）` : ""}`;
  }
  if (ws.canExpand) return `扩加工槽 ${ws.expandCost}`;
  return "设备都装齐了";
});
const visibleEquipmentOptions = computed(() => {
  const options = workshop.value?.equipmentOptions ?? [];
  if (showAllEquipment.value) return options;
  const installed = options.filter((option) => option.installed);
  const next = options.find((option) => !option.installed);
  return next ? [...installed, next] : installed;
});


const seedStacks = computed(() =>
  (snapshot.value?.inventory ?? []).filter((item) => item.kind === "seed"),
);

const plantableSeeds = computed(() => {
  const level = snapshot.value?.level ?? 1;
  return seedStacks.value
    .flatMap((stack) => {
      const rare = stack.key.startsWith("rare_seed:");
      const id = rare
        ? stack.key.slice("rare_seed:".length)
        : stack.key.replace(/^seed:/, "");
      const def = cropById(id);
      if (!def || (!rare && def.unlockLevel > level) || stack.qty <= 0) return [];
      return [
        {
          id,
          name: def.name,
          icon: def.icon,
          qty: stack.qty,
          grow: formatGrowMinutes(def.growMinutes),
          off: !isInSeason(def),
          rare,
        },
      ];
    })
    .sort(
      (a, b) =>
        Number(b.rare) - Number(a.rare) ||
        Number(a.off) - Number(b.off) ||
        a.name.localeCompare(b.name),
    );
});

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
const fishingPet = computed(
  () => activePets.value.find((item) => item.location === "fishing") ?? null,
);
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
const petFoods = computed(() => {
  const care = petCare.value;
  if (!care) return [];
  return [
    { kind: "cat_food" as PetFoodKind, label: "猫粮", count: care.catFood, cost: care.catFoodCost },
    { kind: "dog_food" as PetFoodKind, label: "狗粮", count: care.dogFood, cost: care.dogFoodCost },
    { kind: "dog_bone" as PetFoodKind, label: "狗骨头", count: care.dogBone, cost: care.dogBoneCost },
    { kind: "cat_can" as PetFoodKind, label: "猫罐头", count: care.catCan, cost: care.catCanCost },
  ];
});
const petAnim = ref<"" | "happy" | "play" | "eat">("");
const petEmote = ref("");
let petAnimTimer: number | undefined;

function playPetAnim(kind: "happy" | "play" | "eat", emote: string) {
  petAnim.value = kind;
  petEmote.value = emote;
  if (petAnimTimer !== undefined) window.clearTimeout(petAnimTimer);
  petAnimTimer = window.setTimeout(() => {
    petAnim.value = "";
    petEmote.value = "";
    petAnimTimer = undefined;
  }, 900);
}
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
const mystery = computed(() => snapshot.value?.mystery ?? null);
const rareSeedStacks = computed(() =>
  seedStacks.value.filter((stack) => stack.key.startsWith("rare_seed:")),
);
const farmCards = computed(() =>
  (snapshot.value?.inventory ?? []).filter((item) => item.key === "craft:farm_card"),
);
const mysteryColumns = computed(() => [
  {
    title: "物品",
    hint: "种子 / 一键卡 / 稀有家具",
    offers: (mystery.value?.offers ?? []).filter(
      (offer) =>
        offer.kind === "种子" || offer.kind === "一键卡" || offer.kind === "家具",
    ),
  },
  {
    title: "加成",
    hint: "农场 / 牧场 / 钓鱼 / 制造 / 金币",
    offers: (mystery.value?.offers ?? []).filter((offer) => offer.kind === "加成"),
  },
]);
const levelRewards = computed(() => snapshot.value?.levelRewards ?? []);
const levelRewardClaimable = computed(() => snapshot.value?.levelRewardsClaimable ?? 0);
const npcs = computed(() => snapshot.value?.npcs ?? []);
const specimens = computed(() => snapshot.value?.specimens ?? []);
const specimenFeatured = computed(
  () => specimens.value.find((item) => item.featured) ?? null,
);
const specimenMilestones = computed(() => snapshot.value?.specimenMilestones ?? []);
const specimenResult = ref<Extract<Effect, { kind: "specimen" }> | null>(null);
const npcTalk = ref<NpcTalkEffect | null>(null);
const specimenResultCrop = computed(
  () => specimens.value.find((item) => item.cropId === specimenResult.value?.cropId) ?? null,
);
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
  playerMode.value = false;
  if (tab === "图鉴") {
    void openCodex();
    return;
  }
  if (
    tab === "我的农场" ||
    tab === "牧村" ||
    tab === "邻居农场" ||
    tab === "宠物" ||
    tab === "标本室" ||
    tab === "畜牧场" ||
    tab === "制造屋" ||
    tab === "鱼塘" ||
    tab === "钓鱼" ||
    tab === "神秘商店"
  ) {
    activeTab.value = tab;
    if (tab === "钓鱼") maybeShowAutoReport();
  } else {
    placeholder(tab);
  }
}

async function openFarmSection(id: string) {
  playerMode.value = false;
  activeTab.value = "我的农场";
  await nextTick();
  const el = document.getElementById(id);
  if (!el) return;
  if (el instanceof HTMLDetailsElement) el.open = true;
  el.scrollIntoView({ behavior: "smooth", block: "center" });
}

function openCodex() {
  return openFarmSection("farm-codex");
}

function openPlayerSection(id: string) {
  playerMode.value = true;
  activeTab.value = "我的农场";
  void nextTick(() => {
    const el = document.getElementById(id);
    if (!el) return;
    if (el instanceof HTMLDetailsElement) el.open = true;
    el.scrollIntoView({ behavior: "smooth", block: "center" });
  });
}

function openPlayer() {
  playerMode.value = true;
  activeTab.value = "我的农场";
  window.scrollTo({ top: 0, behavior: "smooth" });
}

function closePlayer() {
  playerMode.value = false;
}

function savePlayerName() {
  const name = playerName.value.trim();
  if (!name) {
    notice.value = "昵称不能为空";
    return;
  }
  if ([...name].length > 12) {
    notice.value = "昵称最多 12 个字";
    return;
  }
  playerName.value = name;
  void act({ type: "updateProfile", ownerName: name });
}

function setAvatar(choice: string) {
  playerAvatar.value = choice;
  playerAvatarImage.value = "";
  try {
    window.localStorage.setItem(PLAYER_AVATAR_KEY, choice);
    window.localStorage.removeItem(PLAYER_AVATAR_IMAGE_KEY);
  } catch {
    // localStorage 只是旧档迁移用的，失败不影响文件保存。
  }
  void persistAvatar(`preset:${choice}`);
}

async function persistAvatar(avatar: string) {
  try {
    await savePlayerAssets({ avatar });
  } catch (error) {
    notice.value = `头像没存进资源文件夹：${errorText(error)}`;
  }
}

function compressAvatar(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    if (!file.type.startsWith("image/")) {
      reject(new Error("请选一张图片"));
      return;
    }
    if (file.size > 8 * 1024 * 1024) {
      reject(new Error("头像图片建议小于 8MB"));
      return;
    }
    const url = URL.createObjectURL(file);
    const image = new Image();
    image.onload = () => {
      URL.revokeObjectURL(url);
      const size = 256;
      const canvas = document.createElement("canvas");
      canvas.width = size;
      canvas.height = size;
      const context = canvas.getContext("2d");
      if (!context) {
        reject(new Error("这台机器画不了头像"));
        return;
      }
      const scale = Math.max(size / image.naturalWidth, size / image.naturalHeight);
      const width = image.naturalWidth * scale;
      const height = image.naturalHeight * scale;
      context.drawImage(image, (size - width) / 2, (size - height) / 2, width, height);
      resolve(canvas.toDataURL("image/png"));
    };
    image.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("这张图片读不出来"));
    };
    image.src = url;
  });
}

async function onAvatarFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  try {
    const data = await compressAvatar(file);
    await saveCustomAsset("avatars", "avatar.png", data);
    await persistAvatar("file:avatar.png");
    playerAvatarImage.value = data;
    notice.value = "头像已存进自定义资源文件夹";
  } catch (error) {
    notice.value = `头像没存上：${errorText(error)}`;
  }
}

async function restorePlayerAssets() {
  customDir.value = await customAssetsDir().catch(() => "");
  try {
    const assets = await loadPlayerAssets();
    if (assets.avatar.startsWith("preset:")) {
      playerAvatar.value = assets.avatar.slice("preset:".length);
      playerAvatarImage.value = "";
      return;
    }
    if (assets.avatar.startsWith("file:")) {
      const data = await readCustomAsset("avatars", assets.avatar.slice("file:".length));
      if (data) {
        playerAvatarImage.value = data;
        return;
      }
    }
  } catch {
    // 资源目录读不到就退回 localStorage 旧数据，不拦启动。
  }
  try {
    const legacyImage = window.localStorage.getItem(PLAYER_AVATAR_IMAGE_KEY) || "";
    if (legacyImage.startsWith("data:image/")) {
      playerAvatarImage.value = legacyImage;
      await saveCustomAsset("avatars", "avatar.png", legacyImage).catch(() => null);
      await persistAvatar("file:avatar.png");
      return;
    }
    const legacyChoice = window.localStorage.getItem(PLAYER_AVATAR_KEY) || "";
    if (legacyChoice) {
      playerAvatar.value = legacyChoice;
      await persistAvatar(`preset:${legacyChoice}`);
    }
  } catch {
    // 旧数据也只是兜底。
  }
}

function openTop(name: string) {
  if (name === "农场") {
    selectTab("我的农场");
    return;
  }
  if (name === "畜牧场" || name === "制造屋" || name === "钓鱼" || name === "牧村" || name === "神秘商店") {
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

function claimSpecimenMilestone(unique: number) {
  void act({ type: "claimSpecimenMilestone", unique });
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
  if (!target) return;
  const fish = (snapshot.value?.inventory ?? [])
    .filter((item) => item.kind === "fish" && item.qty > 0)
    .sort(
      (a, b) =>
        (a.unitPrice ?? 0) - (b.unitPrice ?? 0) || a.key.localeCompare(b.key),
    )[0];
  if (!fish) {
    notice.value = "鱼篓里没有鱼，先去钓鱼";
    return;
  }
  playPetAnim("eat", "🍖");
  void act({
    type: "feedPet",
    petId: target.id,
    fishId: fish.key.replace("fish:", ""),
  });
}

function buyPetFood(kind: PetFoodKind) {
  void act({ type: "buyPetFood", kind });
}

function feedPetFood(kind: PetFoodKind) {
  const target = selectedPet.value;
  if (!target) return;
  playPetAnim("eat", kind === "dog_bone" ? "🦴" : "🍖");
  void act({ type: "feedPetFood", petId: target.id, kind });
}

function petAction(action: "petCompanion" | "playWithPet" | "healPet" | "digWithPet") {
  if (selectedPet.value) {
    if (action === "petCompanion") {
      playPetAnim("happy", "❤️");
    } else if (action === "playWithPet") {
      playPetAnim("play", "🎾");
    } else if (action === "healPet") {
      playPetAnim("happy", "💊");
    } else {
      playPetAnim("happy", "🌱");
    }
    void act({ type: action, petId: selectedPet.value.id });
  }
}

function usePetSkill() {
  if (selectedPet.value) {
    playPetAnim("happy", "✨");
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

function choosePetTalent(talent: PetTalent) {
  if (selectedPet.value) {
    void act({ type: "choosePetTalent", petId: selectedPet.value.id, talent });
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

function chatWithNpc(npc: NpcView) {
  void act({ type: "chatWithNpc", npcId: npc.id });
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

function acceptFarmOrder(id: string) {
  void act({ type: "acceptFarmOrder", id });
}

function deliverFarmOrder(id: string) {
  void act({ type: "deliverFarmOrder", id });
}

function claimLevelReward(level: number) {
  void act({ type: "claimLevelReward", level });
}

async function act(action: Action): Promise<boolean> {
  if (acting.value) return false;
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
    const specimen = reply.effects.find((effect) => effect.kind === "specimen");
    if (specimen && specimen.kind === "specimen") {
      specimenResult.value = specimen;
    }
    const talk = reply.effects.find((effect) => effect.kind === "npcTalk");
    if (talk && talk.kind === "npcTalk") {
      npcTalk.value = talk;
    }
    if (reply.saveError) {
      saveMessage.value = `已生效，但保存失败：${reply.saveError}`;
    }
    return true;
  } catch (e) {
    notice.value = errorText(e);
    return false;
  } finally {
    acting.value = false;
  }
}

async function saveRoom(layout: RoomLayout) {
  const room = currentRoom.value;
  if (!room) return;
  await act({ type: "saveRoomLayout", room: room.roomId, layout });
}

const currentRoomId = ref("living");
const selectedAquariumId = ref("");
const selectedNpcRoomId = ref("");
const selectedNpcRoom = computed(
  () => snapshot.value?.npcRooms.find((room) => room.npcId === selectedNpcRoomId.value) ?? null,
);
const currentRoom = computed(() => {
  if (currentRoomId.value === "extra_1" && snapshot.value?.extraRoom) return snapshot.value.extraRoom;
  return snapshot.value?.room ?? null;
});
const decorShop = computed(() => snapshot.value?.decorShop ?? null);
const furnitureCatalog = computed(() => currentRoom.value?.catalog ?? []);
const furnitureOwnedCount = computed(
  () =>
    furnitureCatalog.value.filter(
      (def) => (currentRoom.value?.ownedFurniture?.[def.id] ?? 0) > 0,
    ).length,
);

function buyFurniture(def: string) {
  void act({ type: "buyFurniture", def });
}

function buyDecor(id: string) {
  void act({ type: "buyDecor", id });
}

function furnitureSourceLabel(source: string) {
  return source === "gift" ? "赠送" : source === "mystery" ? "神秘" : "商店";
}
const selectedAquarium = computed(
  () => currentRoom.value?.layout.furniture.find((item) => item.id === selectedAquariumId.value) ?? null,
);
const aquariumDef = computed(
  () => currentRoom.value?.catalog.find((def) => def.id === selectedAquarium.value?.def) ?? null,
);
const fishInventory = computed(
  () => snapshot.value?.inventory.filter((item) => item.kind === "fish" && item.qty > 0) ?? [],
);
const EXTRA_ROOM_COST = 20_000;

function fishName(id: string): string {
  return (
    snapshot.value?.inventory.find((item) => item.key === `fish:${id}`)?.name ??
    snapshot.value?.fishing.codex.find((fish) => fish.id === id)?.name ??
    id
  );
}

function moveFishToTank(fishId: string) {
  if (!selectedAquarium.value || !currentRoom.value) return;
  void act({
    type: "moveFishToAquarium",
    room: currentRoom.value.roomId,
    furnitureId: selectedAquarium.value.id,
    fishId,
  });
}

function removeFishFromTank(fishId: string) {
  if (!selectedAquarium.value || !currentRoom.value) return;
  void act({
    type: "removeFishFromAquarium",
    room: currentRoom.value.roomId,
    furnitureId: selectedAquarium.value.id,
    fishId,
  });
}

watch(currentRoomId, () => {
  selectedAquariumId.value = "";
});

function travelRoom() {
  if (currentRoomId.value === "living" && snapshot.value?.extraRoom) {
    currentRoomId.value = "extra_1";
  } else {
    currentRoomId.value = "living";
  }
}

async function unlockExtraRoom() {
  await act({ type: "unlockExtraRoom" });
  if (snapshot.value?.extraRoom) currentRoomId.value = "extra_1";
}

watch(
  () => snapshot.value?.extraRoom,
  (extra) => {
    if (!extra) currentRoomId.value = "living";
  },
);

function plant(plot: PlotView) {
  if (!plantableSeeds.value.length) {
    notice.value = "仓库里没有当前等级能种的种子，先去种子铺买。";
    return;
  }
  seedPickerPlot.value = seedPickerPlot.value === plot.index ? null : plot.index;
}

function plantWithSeed(plot: PlotView, seed: { id: string; rare: boolean }) {
  seedPickerPlot.value = null;
  void act(
    seed.rare
      ? { type: "plantRareSeed", plot: plot.index, cropId: seed.id }
      : { type: "plant", plot: plot.index, cropId: seed.id },
  );
}

function buyMystery(id: string) {
  void act({ type: "buyMysteryOffer", id });
}

function useFarmCard() {
  void act({ type: "useFarmCard" });
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

function cleanRanchPen(pen: number) {
  void act({ type: "cleanRanchPen", pen });
}

function toggleRanchDetail(index: number) {
  ranchDetail.value = ranchDetail.value === index ? 0 : index;
  if (ranchDetail.value === index) {
    const animal = ranch?.value?.pens.find((pen) => pen.index === index)?.animal;
    ranchRenameDrafts.value[index] = animal?.name ?? "";
  }
}

function renameRanchAnimal(pen: number) {
  const name = (ranchRenameDrafts.value[pen] ?? "").trim();
  if (!name) {
    notice.value = "名字不能为空";
    return;
  }
  void act({ type: "renameRanchAnimal", pen, name });
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

function acceptRanchOrder(id: string) {
  void act({ type: "acceptRanchOrder", id });
}

function deliverRanchOrder(id: string) {
  void act({ type: "deliverRanchOrder", id });
}

function enterRanchContest(pen: number) {
  void act({ type: "enterRanchContest", pen });
}

function buildPond() {
  void act({ type: "buildPond" });
}

function stockPond(fishId: string, qty: number) {
  void act({ type: "stockPond", fishId, qty });
}

function feedPond() {
  void act({ type: "feedPond" });
}

function cleanPond() {
  void act({ type: "cleanPond" });
}

function installPondAerator() {
  void act({ type: "installPondAerator" });
}

function plantPondWeeds() {
  void act({ type: "plantPondWeeds" });
}

function harvestPond(fishId: string) {
  void act({ type: "harvestPond", fishId });
}

function togglePondFry() {
  void act({ type: "togglePondFry" });
}

function deliverPondOrder() {
  void act({ type: "deliverPondOrder" });
}

const ranchBreedNames = computed(() =>
  ranchBreedSel.value
    .map((pen) => ranch.value?.pens.find((item) => item.index === pen)?.animal?.name ?? `${pen} 号`)
    .join(" × "),
);

function isRanchBreedSelected(pen: number) {
  return ranchBreedSel.value.includes(pen);
}

function toggleRanchBreed(pen: number) {
  const list = [...ranchBreedSel.value];
  const at = list.indexOf(pen);
  if (at >= 0) {
    list.splice(at, 1);
  } else if (list.length < 2) {
    list.push(pen);
  } else {
    list.splice(1, 1, pen);
  }
  ranchBreedSel.value = list;
  ranchPreview.value = null;
  if (list.length === 2) {
    void loadRanchPreview();
  }
}

async function loadRanchPreview() {
  const [a, b] = ranchBreedSel.value;
  if (!a || !b) return;
  try {
    ranchPreview.value = await previewRanchBreeding(a, b);
  } catch (error) {
    ranchPreview.value = null;
    notice.value = errorText(error);
  }
}

async function breedSelectedRanch() {
  const [a, b] = ranchBreedSel.value;
  if (!a || !b) return;
  const ok = await act({ type: "breedRanchPair", a, b });
  if (ok) {
    ranchBreedSel.value = [];
    ranchPreview.value = null;
  }
}

function clearRanchBreed() {
  ranchBreedSel.value = [];
  ranchPreview.value = null;
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

function switchRod(rod: RodKind) {
  if (fishing.value?.session) {
    notice.value = "等收线/咬钩时不能换竿，先收竿";
    return;
  }
  void act({ type: "switchRod", rod });
}

function castLine() {
  void act({ type: "castLine", spot: fishingSpot.value, quality: activeBaitQuality.value });
}

function resolveFishing(success: boolean) {
  void act({ type: "resolveFishing", success });
}

function acceptFishingCommission(id: string) {
  void act({ type: "acceptFishingCommission", id });
}

function showSpotTag(commission: { spotLabel: string; title: string }): boolean {
  return Boolean(commission.spotLabel) && !commission.title.includes(commission.spotLabel);
}

function claimFishingCommission(id: string) {
  void act({ type: "claimFishingCommission", id });
}

function refreshFishingCommissions() {
  void act({ type: "refreshFishingCommissions" });
}

function claimFishingTournament(day: string) {
  void act({ type: "claimFishingTournament", day });
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
  if (fishing.value?.session) {
    notice.value = "等收线/咬钩时不能保养，先收竿";
    return;
  }
  void act({ type: "maintainRod", rod });
}

function onFishingShortcut(event: KeyboardEvent) {
  if (event.key.toLowerCase() !== "f") return;
  if (activeTab.value !== "钓鱼") return;
  const target = event.target as HTMLElement | null;
  if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
  if (catchResult.value) {
    event.preventDefault();
    dismissCatch();
    // 卡片先关掉，再自动续钓；没饵/没解锁就只关卡片。
    window.setTimeout(() => {
      if (
        activeTab.value === "钓鱼" &&
        !snapshot.value?.fishing?.session &&
        !acting.value &&
        currentSpot.value?.unlocked &&
        (activeBaitInfo.value?.count ?? 0) > 0
      ) {
        castLine();
      }
    }, 120);
    return;
  }
  if (snapshot.value?.fishing?.session) return;
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

function upgradeEquipment(slot: number) {
  void act({ type: "upgradeEquipment", slot });
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

function setPetLocation(petId: string, location: PetLocation) {
  void act({ type: "setPetLocation", petId, location });
}

function sendPetToRanch(petId: string) {
  setPetLocation(petId, "ranch");
}

function callPetHome(petId: string) {
  setPetLocation(petId, "farm");
}

function placeholder(name: string) {
  notice.value = `「${name}」还没开放；当前可玩：农场、牧村、邻居农场、畜牧场、制造屋、钓鱼、宠物、标本室。`;
}

function showHelp() {
  notice.value = "文字农场：种地/养牧/钓鱼/制造；设置、外观和存档在右上角玩家页。";
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

/// 样式包只落 data-style/layout/density/decoration；颜色仍归 theme.ts。
function syncStylePack() {
  applyStylePack(document.documentElement, stylePackById(stylePackId.value));
}

function setStylePack(id: string) {
  const pack = stylePackById(id);
  stylePackId.value = pack.id;
  writeStylePack(window.localStorage, pack);
  applyStylePack(document.documentElement, pack);
}

/// 经典布局下 section 常开；点 summary 不许把它合上（里面的按钮照常可用）。
function guardClassicFold(event: MouseEvent) {
  if (classicLayout.value) event.preventDefault();
}

function currentBackgroundPrefs(): BackgroundPrefs {
  return {
    mode: bgMode.value,
    paper: bgPaper.value,
    fit: bgFit.value,
    imageAlpha: bgImageAlpha.value,
    panelAlpha: panelAlpha.value,
  };
}

const hasBackgroundImage = computed(
  () => bgMode.value === "image" && !!backgroundImage.value,
);
const backgroundClassList = computed(() => backgroundClasses(currentBackgroundPrefs()));
const bgImageStyle = computed(() => {
  if (!backgroundImage.value) return {};
  return {
    backgroundImage: `url(${backgroundImage.value})`,
    ...imageFitStyle(bgFit.value),
    backgroundPosition: "center",
  };
});
const bgReadabilityWarning = computed(
  () => bgMode.value === "image" && bgImageAlpha.value > 70,
);

function syncBackground() {
  applyBackgroundPrefs(document.documentElement, currentBackgroundPrefs());
}

function saveBackground() {
  writeBackgroundPrefs(window.localStorage, currentBackgroundPrefs());
  syncBackground();
}

function setBgMode(mode: BackgroundMode) {
  bgMode.value = mode;
  bgImageError.value = "";
  saveBackground();
}

function setBgPaper(pattern: PaperPattern) {
  bgPaper.value = pattern;
  saveBackground();
}

function setBgFit(fit: BackgroundFit) {
  bgFit.value = fit;
  saveBackground();
}

function onImageAlphaInput(event: Event) {
  bgImageAlpha.value = clampPercent(
    (event.target as HTMLInputElement).value,
    0,
    100,
    DEFAULT_BACKGROUND_PREFS.imageAlpha,
  );
  saveBackground();
}

function onPanelAlphaInput(event: Event) {
  panelAlpha.value = clampPercent(
    (event.target as HTMLInputElement).value,
    60,
    100,
    DEFAULT_BACKGROUND_PREFS.panelAlpha,
  );
  saveBackground();
}

async function onPickBackground(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  bgImageBusy.value = true;
  bgImageError.value = "";
  let saved = false;
  try {
    const dataUrl = await compressImage(file);
    await saveBackgroundImage(dataUrl);
    await saveCustomAsset("backgrounds", "background.webp", dataUrl).catch(() => null);
    backgroundImage.value = dataUrl;
    bgMode.value = "image";
    saveBackground();
    saved = true;
    notice.value = "背景图已存到自定义资源文件夹（最长边 1920）。";
  } catch (error) {
    bgImageError.value = error instanceof Error ? error.message : "图片处理失败。";
    notice.value = `背景图没存上：${bgImageError.value}`;
  } finally {
    bgImageBusy.value = false;
    // 失败时清掉选择，允许重试同一张；成功时保留文件名给用户看。
    if (!saved) input.value = "";
  }
}

async function removeBackgroundImage() {
  try {
    await deleteBackgroundImage();
  } catch {
    // 删除失败最多是旧图留在库里，不拦界面。
  }
  backgroundImage.value = null;
  bgMode.value = "solid";
  bgImageError.value = "";
  saveBackground();
  notice.value = "背景图已移除。";
}

function resetBackground() {
  bgMode.value = DEFAULT_BACKGROUND_PREFS.mode;
  bgPaper.value = DEFAULT_BACKGROUND_PREFS.paper;
  bgFit.value = DEFAULT_BACKGROUND_PREFS.fit;
  bgImageAlpha.value = DEFAULT_BACKGROUND_PREFS.imageAlpha;
  panelAlpha.value = DEFAULT_BACKGROUND_PREFS.panelAlpha;
  bgImageError.value = "";
  saveBackground();
}

function currentFontPrefs(): FontPrefs {
  return { id: fontId.value, name: fontName.value };
}

function syncFont() {
  applyFontPrefs(
    document.documentElement,
    currentFontPrefs(),
    customFontFace ? CUSTOM_FONT_FAMILY : undefined,
  );
}

function saveFont() {
  writeFontPrefs(window.localStorage, currentFontPrefs());
  syncFont();
}

function setFontPreset(id: FontPresetId) {
  fontId.value = id;
  fontError.value = "";
  saveFont();
}

async function activateCustomFont(stored: StoredFont) {
  // 先建新的，再删旧的；新字体坏了旧字体还在。
  const face = await createCustomFontFace(stored.data, CUSTOM_FONT_FAMILY);
  if (customFontFace) document.fonts.delete(customFontFace);
  customFontFace = face;
  fontName.value = stored.name;
  fontId.value = "custom";
  saveFont();
  void saveCustomAsset(
    "fonts",
    "custom-font.bin",
    bufferToDataUrl(stored.data, "application/octet-stream"),
  ).catch(() => null);
}

async function useCustomFont() {
  if (fontBusy.value) return;
  if (customFontFace) {
    fontId.value = "custom";
    fontError.value = "";
    saveFont();
    return;
  }
  fontBusy.value = true;
  fontError.value = "";
  try {
    const custom = await readCustomAsset("fonts", "custom-font.bin").catch(() => null);
    const buffer = custom ? dataUrlToBuffer(custom) : null;
    if (buffer) {
      await activateCustomFont({ name: fontName.value || "自定义字体", data: buffer });
      return;
    }
    const stored = await loadCustomFont();
    if (!stored) {
      fontError.value = "本机没有已存的字体文件，先上传一个。";
      return;
    }
    await activateCustomFont(stored);
  } catch (error) {
    fontError.value = error instanceof Error ? error.message : "字体加载失败。";
  } finally {
    fontBusy.value = false;
  }
}

async function onPickFont(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  fontBusy.value = true;
  fontError.value = "";
  let saved = false;
  try {
    const stored = await saveCustomFont(file);
    await activateCustomFont(stored);
    saved = true;
    notice.value = `字体已加载：${stored.name}`;
  } catch (error) {
    fontError.value = error instanceof Error ? error.message : "字体处理失败。";
    notice.value = `字体没加载上：${fontError.value}`;
  } finally {
    fontBusy.value = false;
    if (!saved) input.value = "";
  }
}

async function removeCustomFont() {
  if (customFontFace) {
    document.fonts.delete(customFontFace);
    customFontFace = null;
  }
  try {
    await deleteCustomFont();
  } catch {
    // 删除失败最多是旧字体留在库里，不拦界面。
  }
  const hadCustom = fontId.value === "custom";
  fontName.value = "";
  fontId.value = DEFAULT_FONT_PRESET;
  fontError.value = "";
  saveFont();
  if (hadCustom) notice.value = "自定义字体已移除，回到宋体。";
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
  syncStylePack();
  syncBackground();
  syncFont();
  void restorePlayerAssets();
  void (async () => {
    const custom = await readCustomAsset("backgrounds", "background.webp").catch(() => null);
    if (custom) {
      backgroundImage.value = custom;
      return;
    }
    const legacy = await loadBackgroundImage();
    if (legacy) backgroundImage.value = legacy;
  })();
  if (fontId.value === "custom") {
    fontBusy.value = true;
    void (async () => {
      try {
        const custom = await readCustomAsset("fonts", "custom-font.bin").catch(() => null);
        const buffer = custom ? dataUrlToBuffer(custom) : null;
        if (buffer) {
          await activateCustomFont({ name: fontName.value || "自定义字体", data: buffer });
          return;
        }
        const stored = await loadCustomFont();
        if (fontId.value !== "custom") return;
        if (!stored) {
          fontError.value = "自定义字体文件不在了，已回宋体。";
          fontName.value = "";
          fontId.value = DEFAULT_FONT_PRESET;
          saveFont();
          return;
        }
        await activateCustomFont(stored);
      } catch (error) {
        fontError.value = error instanceof Error ? error.message : "字体加载失败。";
        fontId.value = DEFAULT_FONT_PRESET;
        syncFont();
      } finally {
        fontBusy.value = false;
      }
    })();
  }
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
  <div :class="backgroundClassList" aria-hidden="true">
    <div v-if="hasBackgroundImage" class="app-bg-image" :style="bgImageStyle"></div>
    <div v-if="hasBackgroundImage" class="app-bg-shade"></div>
  </div>
  <div class="page" :class="[seasonClass, { 'player-mode': playerMode }]">
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

    <aside class="player-bar" :class="{ active: playerMode }">
      <button
        class="player-bar-main"
        type="button"
        @click="playerMode ? closePlayer() : openPlayer()"
      >
        <span class="player-avatar">
          <img v-if="playerAvatarImage" :src="playerAvatarImage" alt="" />
          <span v-else>{{ playerAvatar }}</span>
        </span>
        <span class="player-bar-copy">
          <b>{{ snapshot?.ownerName || "玩家" }}</b>
          <span class="muted">农场陪伴你 {{ farmDays }} 天</span>
        </span>
        <span class="player-bar-action">{{ playerMode ? "[返回农场]" : "[玩家]" }}</span>
      </button>
    </aside>

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
      <button class="tab" type="button" @click="selectTab('鱼塘')">鱼塘</button>
      <button class="tab" type="button" @click="selectTab('宠物')">宠物</button>
    </nav>
    <nav v-else class="tabs ranch-tabs">
      <button class="tab active" type="button" disabled>制造屋</button>
      <button class="tab" type="button" @click="selectTab('我的农场')">返回农场</button>
      <button class="tab" type="button" @click="selectTab('畜牧场')">畜牧场</button>
      <button class="tab" type="button" @click="selectTab('鱼塘')">鱼塘</button>
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
      <div v-show="activeTab === '我的农场'" class="farm-tab" :class="{ 'player-mode': playerMode }">
      <section v-if="playerMode" class="panel player-section player-profile">
        <div class="player-profile-head">
          <div class="player-profile-avatar">
            <img v-if="playerAvatarImage" :src="playerAvatarImage" alt="玩家头像" />
            <span v-else>{{ playerAvatar }}</span>
          </div>
          <div class="player-profile-main">
            <div class="player-profile-title">
              <strong>{{ snapshot?.ownerName || "玩家" }}</strong>
              <span class="muted">农场陪伴你 {{ farmDays }} 天</span>
            </div>
            <div class="row player-name-row">
              <input
                v-model="playerName"
                maxlength="12"
                placeholder="给自己起个昵称"
                @keyup.enter="savePlayerName"
              />
              <button class="link" type="button" :disabled="acting" @click="savePlayerName">
                [保存昵称]
              </button>
              <button class="link" type="button" @click="closePlayer">[返回农场]</button>
            </div>
            <div class="row player-avatar-row">
              <span class="muted">头像</span>
              <button
                v-for="choice in avatarChoices"
                :key="choice"
                class="avatar-choice"
                :class="{ active: !playerAvatarImage && playerAvatar === choice }"
                type="button"
                @click="setAvatar(choice)"
              >
                {{ choice }}
              </button>
              <label class="avatar-upload">
                上传图片
                <input type="file" accept="image/*" @change="onAvatarFile" />
              </label>
            </div>
            <div v-if="customDir" class="muted player-assets-path">
              自定义资源：{{ customDir }}
            </div>
          </div>
        </div>
      </section>
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
          <div v-if="seedPickerPlot === plot.index" class="seed-picker">
            <span class="muted">选种子：</span>
            <button
              v-for="seed in plantableSeeds"
              :key="seed.id"
              class="seed-choice"
              type="button"
              :disabled="acting"
              @click="plantWithSeed(plot, seed)"
            >
              <span v-if="seed.rare" class="rare-seed-tag">稀有</span>
              {{ seed.icon }} {{ seed.name }} ×{{ seed.qty }}
              <span class="muted">{{ seed.grow }} · {{ seed.off ? "反季" : "当季" }}</span>
            </button>
            <button class="link" type="button" @click="seedPickerPlot = null">[取消]</button>
          </div>
        </div>
      </section>

      <section class="panel" id="farm-warehouse">
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

      <section class="panel player-section player-settings" id="player-settings">
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

      <section class="panel player-section player-appearance">
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
        <div class="row">
          <span>布局</span>
          <button
            v-for="pack in STYLE_PACKS"
            :key="pack.id"
            class="rate-btn style-pack-btn"
            :class="{ active: stylePackId === pack.id }"
            type="button"
            @click="setStylePack(pack.id)"
          >
            {{ pack.name }}
          </button>
          <span class="muted">改钓鱼页/制造屋排版；小游戏不变</span>
        </div>
        <div class="row">
          <span>字体</span>
          <button
            v-for="preset in FONT_PRESETS"
            :key="preset.id"
            class="rate-btn font-preset"
            :class="{ active: fontId === preset.id }"
            type="button"
            @click="setFontPreset(preset.id)"
          >
            {{ preset.name }}
          </button>
          <button
            class="rate-btn font-custom"
            :class="{ active: fontId === 'custom' }"
            type="button"
            :disabled="!fontName || fontBusy"
            @click="useCustomFont"
          >
            自定义
          </button>
        </div>
        <div class="row">
          <span>上传字体</span>
          <input
            class="font-file"
            type="file"
            :accept="FONT_FILE_ACCEPT"
            :disabled="fontBusy"
            @change="onPickFont"
          />
          <span v-if="fontBusy" class="muted">正在加载字体…</span>
          <span v-else-if="fontName" class="muted">已存：{{ fontName }}</span>
          <button v-if="fontName" class="link" type="button" @click="removeCustomFont">
            [移除自定义字体]
          </button>
        </div>
        <div v-if="fontError" class="row bg-warning">{{ fontError }}</div>
        <div class="row muted">
          字体只存本机；请只上传你有权使用的字体（.ttf / .otf / .woff / .woff2，≤30MB）。
        </div>
        <details class="appearance-sub">
          <summary>背景与透明度</summary>
          <div class="row">
            <span>背景</span>
            <button
              v-for="mode in BACKGROUND_MODES"
              :key="mode.id"
              class="rate-btn"
              :class="{ active: bgMode === mode.id }"
              type="button"
              @click="setBgMode(mode.id)"
            >
              {{ mode.name }}
            </button>
          </div>
          <div v-if="bgMode === 'paper'" class="row">
            <span>纸纹</span>
            <button
              v-for="pattern in PAPER_PATTERNS"
              :key="pattern.id"
              class="rate-btn"
              :class="{ active: bgPaper === pattern.id }"
              type="button"
              @click="setBgPaper(pattern.id)"
            >
              {{ pattern.name }}
            </button>
          </div>
          <template v-if="bgMode === 'image'">
            <div class="row">
              <span>图片</span>
              <input
                class="bg-file"
                type="file"
                accept="image/*"
                :disabled="bgImageBusy"
                @change="onPickBackground"
              />
              <span v-if="bgImageBusy" class="muted">正在压缩…</span>
              <span
                v-else-if="backgroundImage"
                class="bg-preview"
                :style="{ backgroundImage: `url(${backgroundImage})` }"
              ></span>
              <button
                v-if="backgroundImage"
                class="link"
                type="button"
                @click="removeBackgroundImage"
              >
                [移除图片]
              </button>
            </div>
            <div v-if="!backgroundImage" class="row muted">
              还没选图片；选了以后会压到最长边 1920，再存本机。
            </div>
            <div v-if="backgroundImage" class="row">
              <span>图片不透明度</span>
              <input
                class="bg-slider bg-slider--image"
                type="range"
                min="0"
                max="100"
                step="1"
                :value="bgImageAlpha"
                @input="onImageAlphaInput"
              />
              <span class="muted">{{ bgImageAlpha }}%</span>
            </div>
            <div v-if="backgroundImage" class="row">
              <span>铺法</span>
              <button
                v-for="fit in BACKGROUND_FITS"
                :key="fit.id"
                class="rate-btn"
                :class="{ active: bgFit === fit.id }"
                type="button"
                @click="setBgFit(fit.id)"
              >
                {{ fit.name }}
              </button>
            </div>
            <div v-if="bgImageError" class="row bg-warning">{{ bgImageError }}</div>
          </template>
          <div class="row">
            <span>面板不透明度</span>
            <input
              class="bg-slider bg-slider--panel"
              type="range"
              min="60"
              max="100"
              step="1"
              :value="panelAlpha"
              @input="onPanelAlphaInput"
            />
            <span class="muted">{{ panelAlpha }}%</span>
          </div>
          <div v-if="bgReadabilityWarning" class="row bg-warning">
            图片不透明度超过 70%，字可能不好认；建议 30–50%。
          </div>
          <div class="row">
            <button class="link" type="button" @click="resetBackground">恢复背景默认</button>
            <span class="muted">图片存本机 IndexedDB，不写存档、不碰文件路径。</span>
          </div>
        </details>
        <div class="row muted appearance-hint">
          只存在这台电脑上（localStorage），不写存档；默认跟随系统 + 新版布局，改完立即生效。
        </div>
      </section>

      <section v-if="snapshot?.orders" class="panel farm-orders">
        <div class="panel-title">
          村庄订单
          <span class="muted">
            {{ snapshot.orders.day }} · 已接 {{ snapshot.orders.active.length }}/{{
              snapshot.orders.maxActive
            }}
          </span>
        </div>
        <div v-if="snapshot.orders.active.length" class="muted farm-orders-sub">进行中</div>
        <div
          v-for="order in snapshot.orders.active"
          :key="`active-${order.id}`"
          class="row farm-order"
        >
          <span class="farm-order-main">
            <b>{{ order.npcName }}</b> 收 {{ order.cropName }} ×{{ order.qty }}
          </span>
          <span class="muted">
            {{ order.expiresDay }} 前 · 仓库 {{ order.have }}/{{ order.qty }}
          </span>
          <button
            v-if="order.canDeliver"
            class="link"
            type="button"
            :disabled="acting"
            @click="deliverFarmOrder(order.id)"
          >
            [交付 +{{ order.rewardCoins }}]
          </button>
          <span v-else class="muted">还差 {{ order.qty - order.have }}</span>
        </div>
        <div v-if="snapshot.orders.offers.length" class="muted farm-orders-sub">可接</div>
        <div
          v-for="order in snapshot.orders.offers"
          :key="`offer-${order.id}`"
          class="row farm-order"
        >
          <span class="farm-order-main">
            <b>{{ order.npcName }}</b> 收 {{ order.cropName }} ×{{ order.qty }}
          </span>
          <span class="muted">{{ order.expiresDay }} 前 · 仓库 {{ order.have }}</span>
          <button
            class="link"
            type="button"
            :disabled="acting || snapshot.orders.active.length >= snapshot.orders.maxActive"
            @click="acceptFarmOrder(order.id)"
          >
            [接单]
          </button>
          <span class="muted">金币+{{ order.rewardCoins }} · 经验+{{ order.rewardExp }}</span>
        </div>
        <div v-if="!snapshot.orders.active.length && !snapshot.orders.offers.length" class="muted">
          今天没有订单，明天再来。
        </div>
        <div class="muted farm-orders-hint">
          每天 3 条，最多接 2 条；交付扣仓库作物；过期只掉 NPC 友好度，不扣金币。
        </div>
      </section>

      <section v-if="daily" class="panel" id="farm-daily">
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

      <details class="panel" id="farm-achievements">
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

      <details class="panel" id="farm-codex">
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

      <section class="panel" id="farm-shop">
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

      <section class="panel player-section player-save">
        <div class="panel-title">存档与内容包</div>
        <div class="file-actions">
          <button type="button" @click="exportSaveFile">导出存档</button>
          <button type="button" @click="importSaveFile">导入存档</button>
          <button type="button" @click="exportContentFile">导出内容包</button>
          <button type="button" @click="importContentFile">导入内容包</button>
        </div>
        <div class="muted">内容包不能删除正在种或仓库里还有的作物。</div>
      </section>

      <details v-if="editor" class="panel player-section player-editor">
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

      <section class="boot player-section player-boot">
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
      <section class="panel player-section player-skills">
        <div class="panel-title">
          专精技能
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
      <section class="panel player-section player-buffs-panel">
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

      <div v-show="activeTab === '神秘商店'">
        <section class="panel">
          <div class="panel-title">
            神秘商店
            <span class="muted">{{ mystery?.day }} · 每天刷新一次</span>
            <span class="mystery-wallet">当前金币 {{ snapshot?.coins ?? 0 }}</span>
          </div>
          <p class="muted mystery-intro">
            物品/加成两栏；加成覆盖农场、牧场、钓鱼、制造。只卖稀有货，专栏每天 3% 概率出传说，价格很贵。
          </p>
          <div v-if="mystery?.legendary" class="mystery-legendary">
            今天专栏出了一件传说商品。
          </div>
          <div class="mystery-sections">
            <section
              v-for="column in mysteryColumns"
              :key="column.title"
              class="panel mystery-section"
            >
              <div class="panel-title">
                {{ column.title }}
                <span class="muted">{{ column.hint }}</span>
              </div>
              <div class="mystery-grid">
                <div
                  v-for="offer in column.offers"
                  :key="offer.id"
                  class="mystery-card"
                  :class="[offer.rarity, { featured: offer.featured, sold: offer.sold }]"
                >
                  <div class="mystery-card-head">
                    <span class="mystery-rarity">{{ offer.rarityLabel }}</span>
                    <span v-if="offer.featured" class="mystery-featured">专栏</span>
                    <span class="mystery-tag">{{ offer.tag }}</span>
                  </div>
                  <strong>{{ offer.name }}</strong>
                  <span class="muted mystery-desc">{{ offer.desc }}</span>
                  <div class="mystery-buy">
                    <b>{{ offer.price }} 金币</b>
                    <button
                      v-if="!offer.sold"
                      class="link"
                      type="button"
                      :disabled="acting || !offer.affordable"
                      @click="buyMystery(offer.id)"
                    >
                      {{ offer.affordable ? "[购买]" : "[金币不足]" }}
                    </button>
                    <span v-else class="muted">已售罄</span>
                  </div>
                </div>
              </div>
              <div v-if="!column.offers.length" class="muted">今天这栏没有货。</div>
            </section>
          </div>
          <div v-if="!mystery?.offers.length" class="muted">今天还没进货，明天再来。</div>

          <div class="mystery-backpack">
            <div class="panel-title">
              我的背包
              <span class="muted">买了先放着，想用再用；今天不想钓鱼也可以囤着</span>
            </div>
            <div v-if="ownedBuffItems.length || rareSeedStacks.length || farmCards.length">
              <div v-for="item in ownedBuffItems" :key="item.key" class="row mystery-bag-row">
                <span><b>{{ item.name }}</b>×{{ item.qty }}</span>
                <span class="muted">{{ item.desc }}</span>
                <button class="link" type="button" :disabled="acting" @click="useBuffItem(item.key)">
                  [使用]
                </button>
              </div>
              <div v-for="stack in rareSeedStacks" :key="stack.key" class="row mystery-bag-row">
                <span><b>{{ stack.name }}</b>×{{ stack.qty }}</span>
                <span class="muted">去农场空地 [播种] 选用</span>
              </div>
              <div v-for="card in farmCards" :key="card.key" class="row mystery-bag-row">
                <span><b>{{ card.name }}</b>×{{ card.qty }}</span>
                <span class="muted">一键除虫/浇水/收获/补种（只用仓库普通种子）</span>
                <button class="link" type="button" :disabled="acting" @click="useFarmCard">
                  [一键执行]
                </button>
              </div>
            </div>
            <div v-else class="muted">背包里还没有神秘商店的东西。</div>
          </div>
        </section>
      </div>

      <div v-show="activeTab === '牧村'">
        <section class="panel">
          <div class="panel-title">
            牧村 · {{ currentRoom?.label ?? "我的小屋" }}
            <span class="muted">平时家具锁定；进入装修模式才能移动、收纳和换样式</span>
          </div>
          <RoomScene
            v-if="currentRoom"
            :room="currentRoom"
            :active="activeTab === '牧村'"
            :extra-unlocked="!!snapshot?.extraRoom"
            :extra-cost="EXTRA_ROOM_COST"
            :pets="activePets"
            @save="saveRoom"
            @travel="travelRoom"
            @unlock="unlockExtraRoom"
            @aquarium="selectedAquariumId = $event"
            @notice="notice = $event"
          />
          <div v-if="selectedAquarium" class="tank-panel">
            <div class="tank-head">
              <strong>{{ aquariumDef?.name ?? "鱼缸" }}</strong>
              <span class="muted">{{ selectedAquarium.fish.length }}/{{ aquariumDef?.capacity ?? 0 }}</span>
              <button class="link" type="button" @click="selectedAquariumId = ''">[关闭]</button>
            </div>
            <div class="tank-row">
              <span class="muted">缸里：</span>
              <template v-if="selectedAquarium.fish.length">
                <span v-for="(fish, index) in selectedAquarium.fish" :key="`${fish}-${index}`" class="tank-fish">
                  <img :src="`/fish/${fish}.png`" alt="" />
                  {{ fishName(fish) }}
                  <button class="link" type="button" @click="removeFishFromTank(fish)">[取出]</button>
                </span>
              </template>
              <span v-else class="muted">空的</span>
            </div>
            <div class="tank-row">
              <span class="muted">仓库里的鱼：</span>
              <template v-if="fishInventory.length">
                <span v-for="item in fishInventory" :key="item.key" class="tank-fish">
                  <img :src="`/fish/${item.key.slice(5)}.png`" alt="" />
                  {{ item.name }}×{{ item.qty }}
                  <button
                    class="link"
                    type="button"
                    :disabled="selectedAquarium.fish.length >= (aquariumDef?.capacity ?? 0)"
                    @click="moveFishToTank(item.key.slice(5))"
                  >
                    [放入]
                  </button>
                </span>
              </template>
              <span v-else class="muted">没有可放的鱼</span>
            </div>
            <p class="muted tank-tip">放进鱼缸的鱼不会出现在仓库，也不能出售；取出后才回到仓库。</p>
          </div>
          <div class="npc-room-list">
            <div class="panel-title">邻居小屋</div>
            <div v-for="room in snapshot?.npcRooms ?? []" :key="room.npcId" class="npc-room-row">
              <span>{{ room.name }} · {{ room.relation }} · 好感 {{ room.friendship }}/{{ room.threshold }}</span>
              <button
                v-if="room.unlocked"
                class="link"
                type="button"
                @click="selectedNpcRoomId = room.npcId"
              >
                [参观]
              </button>
              <span v-else class="muted">好感不足</span>
            </div>
          </div>
          <div v-if="selectedNpcRoom" class="npc-room-view">
            <div class="tank-head">
              <strong>{{ selectedNpcRoom.room.label }}</strong>
              <span class="muted">只读参观 · 不能装修</span>
              <button class="link" type="button" @click="selectedNpcRoomId = ''">[返回]</button>
            </div>
            <RoomScene
              :room="selectedNpcRoom.room"
              :active="true"
              :extra-unlocked="true"
              :extra-cost="0"
              :pets="[]"
              readonly
              @notice="notice = $event"
            />
          </div>

          <div class="decor-shop">
            <div class="panel-title">
              家具与装饰店
              <span class="muted">当前金币 {{ snapshot?.coins ?? 0 }}</span>
            </div>
            <p class="muted decor-shop-tip">
              家具直接金币买；地板/墙纸每天随机上架 2+2 件，买过永久拥有，不再看制造屋。
            </p>
            <div class="decor-shop-grid">
              <div v-for="offer in decorShop?.offers ?? []" :key="offer.id" class="decor-card">
                <span class="decor-kind">{{ offer.kind }}</span>
                <strong>{{ offer.name }}</strong>
                <span class="muted">{{ offer.price }} 金币</span>
                <button
                  v-if="!offer.sold && !offer.owned"
                  class="link"
                  type="button"
                  :disabled="acting || (snapshot?.coins ?? 0) < offer.price"
                  @click="buyDecor(offer.id)"
                >
                  {{ (snapshot?.coins ?? 0) >= offer.price ? "[购买]" : "[金币不足]" }}
                </button>
                <span v-else class="muted">{{ offer.owned ? "已拥有" : "已售罄" }}</span>
              </div>
            </div>
            <div class="panel-title">
              家具
              <span class="muted">图鉴 {{ furnitureOwnedCount }}/{{ furnitureCatalog.length }}</span>
            </div>
            <div class="decor-shop-grid">
              <div
                v-for="def in furnitureCatalog.filter((item) => item.price > 0)"
                :key="def.id"
                class="decor-card"
              >
                <strong>{{ def.name }}</strong>
                <span class="muted">
                  {{ def.price }} 金币 · 已有 {{ currentRoom?.ownedFurniture?.[def.id] ?? 0 }} ·
                  可摆 {{ currentRoom?.availableFurniture?.[def.id] ?? 0 }}
                </span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || (snapshot?.coins ?? 0) < def.price"
                  @click="buyFurniture(def.id)"
                >
                  {{ (snapshot?.coins ?? 0) >= def.price ? "[购买]" : "[金币不足]" }}
                </button>
              </div>
            </div>
          </div>
        </section>

          <section class="panel furniture-codex">
            <div class="panel-title">
              家具图鉴
              <span class="muted">已拥有 {{ furnitureOwnedCount }}/{{ furnitureCatalog.length }}</span>
            </div>
            <div class="codex-grid">
              <div
                v-for="def in furnitureCatalog"
                :key="def.id"
                class="codex-card"
                :class="{ owned: (currentRoom?.ownedFurniture?.[def.id] ?? 0) > 0 }"
              >
                <span class="decor-kind">{{ furnitureSourceLabel(def.source) }}</span>
                <strong>{{ def.name }}</strong>
                <span class="muted">
                  {{ (currentRoom?.ownedFurniture?.[def.id] ?? 0) > 0 ? "已拥有" : "未拥有" }}
                </span>
              </div>
            </div>
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
              <strong>{{ npc.emoji }} {{ npc.name }}</strong>
              <span class="muted">{{ npc.role }} · {{ npc.level }}级 · {{ npc.relation }}（{{ npc.friendship }}）</span>
              <button class="link" type="button" :disabled="acting" @click="chatWithNpc(npc)">
                [聊聊]
              </button>
              <button class="link" type="button" :disabled="acting" @click="greetNpc(npc)">
                [留句话]
              </button>
            </div>
            <div class="npc-profile">
              <span v-for="tag in npc.tags" :key="tag" class="npc-tag">{{ tag }}</span>
              <span class="muted">喜欢 {{ npc.likes }} · 不喜 {{ npc.dislikes }}</span>
            </div>
            <p class="npc-today">“{{ npc.todayLine }}”</p>
            <div class="npc-meta">
              <span class="muted">{{ npc.routine }}</span>
              <span v-if="npc.nextEvent" class="muted">· 下一段故事：{{ npc.nextEvent }}</span>
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
            <span class="muted">专用饲料提升品质；繁殖消耗 1 份饲料、需要空栏</span>
          </div>
          <div class="ranch-orders">
            <div class="ranch-orders-head">
              <strong>牧场订单</strong>
              <span class="muted">每天 2 单；接单上限 {{ ranch?.orders.maxActive ?? 2 }}，过期不扣东西</span>
            </div>
            <div v-if="!(ranch?.orders.offers.length || ranch?.orders.active.length)" class="muted">
              今天没有牧场订单
            </div>
            <div v-for="order in ranch?.orders.offers ?? []" :key="order.id" class="ranch-order">
              <span>{{ order.npcName }} 收 {{ order.qualityLabel }}{{ order.itemName }} ×{{ order.qty }}</span>
              <span class="muted">截止 {{ order.expiresDay }} · {{ order.rewardCoins }} 金币 + {{ order.rewardExp }} 经验 + {{ order.rewardFeed }} 饲料</span>
              <button class="link" type="button" :disabled="acting" @click="acceptRanchOrder(order.id)">[接单]</button>
            </div>
            <div v-for="order in ranch?.orders.active ?? []" :key="order.id" class="ranch-order active">
              <span>进行中：{{ order.qualityLabel }}{{ order.itemName }} ×{{ order.qty }}</span>
              <span class="muted">仓库 {{ order.have }}/{{ order.qty }}</span>
              <button class="link" type="button" :disabled="acting || !order.canDeliver" @click="deliverRanchOrder(order.id)">[交付]</button>
            </div>
          </div>
          <div class="ranch-pens">
            <div v-for="pen in ranch?.pens ?? []" :key="pen.index" class="ranch-pen">
              <div class="ranch-pen-head">
                畜栏 {{ pen.index }}
                <span class="muted">· 卫生 {{ pen.cleanliness }}/100</span>
              </div>
              <div class="ranch-meters">
                <span class="ranch-meter">
                  <em>卫生</em>
                  <span class="ranch-meter-track clean"><i :style="{ width: `${pen.cleanliness}%` }"></i></span>
                </span>
              </div>
              <template v-if="pen.animal">
                <div class="ranch-animal">
                  <span class="ranch-emoji">{{ pen.animal.emoji }}</span>
                  <div class="ranch-animal-main">
                    <div class="ranch-animal-name">
                      <strong>{{ pen.animal.name }}</strong>
                      <span class="ranch-trait">{{ pen.animal.traitLabel }}</span>
                      <span v-if="pen.animal.young" class="ranch-trait young">幼年</span>
                      <span v-if="pen.animal.inbred" class="ranch-trait inbred">近亲</span>
                      <span class="muted">{{ pen.animal.lineLabel }}</span>
                      <button class="link ranch-detail-toggle" type="button" @click="toggleRanchDetail(pen.index)">
                        {{ ranchDetail === pen.index ? "[收起]" : "[详情]" }}
                      </button>
                      <button class="link ranch-detail-toggle" type="button" @click="toggleRanchBreed(pen.index)">
                        {{ isRanchBreedSelected(pen.index) ? "[已选]" : "[选为亲本]" }}
                      </button>
                    </div>
                    <div class="muted">{{ pen.animal.label }} · {{ pen.animal.qualityLabel }} · 第{{ pen.animal.generation + 1 }}代 · {{ pen.animal.ageDays }} 天</div>
                    <div class="muted">饱食 {{ pen.animal.satiety }}/100 · 健康 {{ pen.animal.health }}/100</div>
                    <div v-if="pen.animal.sick" class="warn">生病了，需要兽医</div>
                    <div v-else-if="pen.animal.young" class="muted">
                      幼年，还要 {{ formatRemain(pen.animal.youngRemainingMs) }} 成年
                    </div>
                    <div v-else class="muted">
                      {{ pen.animal.canCollect ? `${pen.animal.product}可以收了` : `${pen.animal.product}还要 ${formatRemain(pen.animal.remainingMs)}` }}
                    </div>
                    <div v-if="ranchDetail === pen.index" class="ranch-detail">
                      <div>生产特性：{{ pen.animal.traitLabel }}（{{ pen.animal.traitHint }}）</div>
                      <div>血统：{{ pen.animal.lineLabel }} · 第{{ pen.animal.generation + 1 }}代</div>
                      <div v-if="pen.animal.young">幼年：还要 {{ formatRemain(pen.animal.youngRemainingMs) }} 成年</div>
                      <div v-else-if="!pen.animal.breedReady" class="muted">繁殖冷却：还要 {{ formatRemain(pen.animal.breedRemainingMs) }}</div>
                      <div v-if="pen.animal.parents" class="muted">亲本：{{ pen.animal.parents }}</div>
                      <div>特殊产物：{{ pen.animal.specialName }}（{{ pen.animal.specialHint }}）</div>
                      <div>栏舍卫生影响高品质概率和生病率；收产物会弄脏，记得清理。</div>
                      <div class="ranch-rename">
                        <input
                          v-model="ranchRenameDrafts[pen.index]"
                          class="ranch-rename-input"
                          type="text"
                          maxlength="12"
                          placeholder="新名字"
                        />
                        <button class="link" type="button" :disabled="acting" @click="renameRanchAnimal(pen.index)">[改名]</button>
                      </div>
                      <div class="ranch-contest-entry">
                        <button
                          class="link"
                          type="button"
                          :disabled="acting || ranch?.contest.entered || pen.animal.young || pen.animal.sick"
                          @click="enterRanchContest(pen.index)"
                        >
                          {{ ranch?.contest.entered ? "[本周已报名]" : "[报名参赛]" }}
                        </button>
                        <span class="muted">按品质/代数/特性/健康评分；每周一只</span>
                      </div>
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
                  <button class="link" type="button" :disabled="acting || !pen.canClean" @click="cleanRanchPen(pen.index)">
                    [清理畜栏]
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
          <div class="ranch-breed">
            <div class="ranch-breed-head">
              <strong>繁殖</strong>
              <span class="muted">选两只成年动物；双方 72 小时内只繁一次，幼崽 24 小时成年</span>
            </div>
            <div v-if="!ranchBreedSel.length" class="muted">在畜栏里点 [选为亲本] 选两只动物</div>
            <div v-else class="ranch-breed-picked">
              <span>已选：{{ ranchBreedNames }}</span>
              <button class="link" type="button" :disabled="acting || ranchBreedSel.length < 2" @click="loadRanchPreview">[刷新预览]</button>
              <button
                class="link"
                type="button"
                :disabled="acting || ranchBreedSel.length < 2 || !ranchPreview?.ok"
                @click="breedSelectedRanch"
              >
                [繁殖]
              </button>
              <button class="link" type="button" :disabled="acting" @click="clearRanchBreed">[清空]</button>
            </div>
            <div v-if="ranchPreview" class="ranch-breed-preview">
              <div>
                后代：{{ ranchPreview.kindLabel }} · {{ ranchPreview.qualityLabel }} · 第{{ ranchPreview.generation + 1 }}代 · {{ ranchPreview.traitLabel }} · {{ ranchPreview.lineLabel }}
              </div>
              <div v-if="ranchPreview.parents" class="muted">亲本：{{ ranchPreview.parents }}</div>
              <div v-for="warning in ranchPreview.warnings" :key="warning" class="warn">{{ warning }}</div>
              <div v-for="missing in ranchPreview.missing" :key="missing" class="warn">{{ missing }}</div>
            </div>
          </div>
          <div class="ranch-contest">
            <div class="ranch-contest-head">
              <strong>牧场评比</strong>
              <span class="muted">每周一只；不参加不损失，名次给金币 + 奖章</span>
            </div>
            <div class="muted">
              本周 {{ ranch?.contest.week || "—" }} · 累计奖章 {{ ranch?.contest.medals ?? 0 }}
              <template v-if="(ranch?.contest.bestRank ?? 0) > 0"> · 最好第 {{ ranch?.contest.bestRank }} 名</template>
            </div>
            <div v-if="ranch?.contest.result" class="ranch-contest-result">
              第 {{ ranch.contest.result.rank }} 名 · {{ ranch.contest.result.animalName }}（{{ ranch.contest.result.kindLabel }}）·
              得分 {{ ranch.contest.result.score }} · {{ ranch.contest.result.rewardCoins }} 金币 · 奖章 +{{ ranch.contest.result.medal }}
            </div>
            <div v-else class="muted">在动物详情里点 [报名参赛] 选一只。</div>
          </div>
          <div class="ranch-codex">
            <div class="ranch-codex-head">
              <strong>牧场图鉴</strong>
              <span class="muted">
                动物 {{ ranch?.codex.animalDone ?? 0 }}/{{ ranch?.codex.animalTotal ?? 20 }} ·
                特殊产物 {{ ranch?.codex.specialDone ?? 0 }}/{{ ranch?.codex.specialTotal ?? 4 }} ·
                奖章 {{ ranch?.codex.medals ?? 0 }}
              </span>
            </div>
            <div v-for="group in ranch?.codex.kinds ?? []" :key="group.kind" class="ranch-codex-row">
              <span class="ranch-codex-name">{{ group.emoji }} {{ group.label }} {{ group.done }}/{{ group.total }}</span>
              <span
                v-for="trait in group.traits"
                :key="trait.key"
                class="ranch-codex-chip"
                :class="{ have: trait.have }"
              >
                {{ trait.label }}{{ trait.have ? "✓" : "·" }}
              </span>
            </div>
            <div class="ranch-codex-row">
              <span class="ranch-codex-name">特殊产物 {{ ranch?.codex.specialDone ?? 0 }}/{{ ranch?.codex.specialTotal ?? 4 }}</span>
              <span
                v-for="item in ranch?.codex.specials ?? []"
                :key="item.key"
                class="ranch-codex-chip"
                :class="{ have: item.have }"
              >
                {{ item.name }}{{ item.have ? "✓" : "·" }}
              </span>
            </div>
          </div>
          <div class="ranch-workshop">
            <div class="ranch-workshop-head">
              <strong>🏭 制造屋</strong>
              <span class="muted">牧场原料可以拿到制造屋，做成奶酪/黄油/蛋糕/毛线/咸鸭蛋/甜点</span>
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

      <div v-show="activeTab === '制造屋'" class="workshop-page">
        <div class="level">{{ levelLine }}</div>
        <div class="sub">
          {{ currency }} {{ snapshot?.coins ?? 0 }}
          <span class="muted">· 制造屋只做加工，不影响动物状态</span>
        </div>
        <div class="rule"></div>
        <template v-if="workshop">
          <section v-if="!workshop.built" class="panel">
            <div class="panel-title">
              制造屋
              <span class="muted">建一座制造屋，就能把作物和牧场原料做成更值钱的东西</span>
            </div>
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
          </section>
          <template v-else>
            <div class="ws-body" :class="{ panel: classicLayout }">
              <div class="panel-title ws-classic-title">
                制造屋
                <span class="muted">加工坊放设备，各槽各跑各的；配方按设备分组</span>
              </div>
              <div class="ws-hero">
              <div class="ws-hero-main">
                <div class="ws-hero-title">
                  制造屋
                  <b>{{ workshop.slots }}</b> 个加工槽
                </div>
                <div class="ws-hero-meta">
                  正在加工 <b>{{ workshopActiveJobs }}</b> · 可取货
                  <b>{{ workshopCollectable }}</b>
                </div>
              </div>
              <div class="ws-hero-side">
                <span class="ws-pill" :class="{ on: workshopActiveJobs > 0 }">
                  {{ workshopActiveJobs > 0 ? "在忙" : "空闲" }}
                </span>
                <span class="muted">{{ workshop.slots }}/{{ workshop.maxSlots }} 槽</span>
              </div>
            </div>

            <div class="ws-card ws-today">
              <div class="ws-card-head">
                <b>今天做这几件</b>
                <span class="muted">不用翻配方，先看这里</span>
              </div>
              <div class="ws-today-row">
                <span class="ws-today-tag">取货</span>
                <template v-if="firstCollectable">
                  <span>{{ firstCollectable.job?.outputName }}</span>
                  <button class="link" type="button" :disabled="acting" @click="collectCraft(firstCollectable.index)">[取货]</button>
                </template>
                <span v-else class="muted">暂无货可取</span>
              </div>
              <div class="ws-today-row">
                <span class="ws-today-tag">开工</span>
                <template v-if="topRecipe">
                  <span>{{ topRecipe.label }} → {{ topRecipe.outputName }}<span v-if="topRecipe.price" class="muted">（{{ topRecipe.price }} 金币）</span></span>
                  <button class="link" type="button" :disabled="acting" @click="topRecipe.start()">[开工]</button>
                </template>
                <span v-else class="muted">原料不够，先去农场/牧场/钓鱼</span>
              </div>
              <div class="ws-today-row">
                <span class="ws-today-tag">下一个</span>
                <span class="muted">{{ nextUnlock }}</span>
              </div>
            </div>

            <div class="ws-card ws-card--slots">
              <div class="ws-card-head">
                <b>加工槽</b>
                <span class="muted">加工坊放设备，各槽各跑各的；手工台不用装设备</span>
              </div>
              <div class="workshop-slots">
              <div v-for="slot in workshop.slotViews" :key="slot.index" class="workshop-slot">
                <div class="workshop-slot-head">
                  {{ slot.index }} 号槽 · {{ slot.equipmentEmoji }} {{ slot.equipmentLabel }}
                  <span v-if="slot.index > 0 && slot.equipment" class="muted">
                    Lv{{ slot.equipmentLevel }}/{{ slot.equipmentMaxLevel }}
                  </span>
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
                <div v-if="slot.index > 0 && slot.equipment" class="workshop-slot-upgrade">
                  <button
                    v-if="slot.canUpgrade"
                    class="link"
                    type="button"
                    :disabled="acting"
                    @click="upgradeEquipment(slot.index)"
                  >
                    [升级 {{ slot.upgradeCost }}]
                  </button>
                  <span v-else-if="slot.equipmentLevel < slot.equipmentMaxLevel" class="muted">
                    {{ slot.upgradeMissing }}
                  </span>
                  <span v-else class="muted">已满级</span>
                  <span class="muted">
                    加工时间 -{{ slot.equipmentSpeedPct }}% · 取货增产 +{{ slot.equipmentYieldPct }}%
                  </span>
                </div>
              </div>
            </div>
            </div>

            <details class="ws-card ws-card--gear ws-fold" open>
              <summary @click="guardClassicFold">
                <span class="ws-fold-title">设备</span>
                <span class="ws-count muted">
                  已装 {{ workshopInstalled }} 台 ·
                  <template v-if="emptyEquipmentSlot !== null">下一个空槽 {{ emptyEquipmentSlot }} 号</template>
                  <template v-else>设备槽已满</template>
                  · {{ workshop.slots }}/{{ workshop.maxSlots }} 槽
                </span>
              </summary>
              <div class="equipment-options">
              <span
                v-for="option in visibleEquipmentOptions"
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
              <button class="link" type="button" @click="showAllEquipment = !showAllEquipment">
                {{ showAllEquipment ? "[收起设备]" : `[查看全部设备 ${workshop.equipmentOptions.length}]` }}
              </button>
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
            </details>

            <details class="ws-card ws-card--recipes ws-fold" open>
              <summary>
                <span class="ws-fold-title">配方</span>
                <span class="ws-count muted">可开工 {{ readyRecipeTotal }} / {{ recipeTotal }}</span>
                <span class="muted">按原料来源分组；默认只看能做的</span>
              </summary>
              <div class="ws-recipe-toolbar">
                <button class="link" type="button" @click="onlyReadyRecipes = !onlyReadyRecipes">
                  {{ onlyReadyRecipes ? "[显示全部配方]" : "[只看可做]" }}
                </button>
                <span class="muted">{{ onlyReadyRecipes ? "只显示原料齐的" : "显示全部" }} · 共 {{ recipeTotal }} 条</span>
              </div>
              <div class="ws-recipe-tabs">
                <button
                  v-for="group in wsRecipeGroups"
                  :key="group.key"
                  class="ws-recipe-tab"
                  :class="{ on: recipeGroup === group.key }"
                  type="button"
                  @click="recipeGroup = group.key"
                >
                  {{ group.label }} {{ group.ready }}/{{ group.total }}
                </button>
              </div>
              <template v-for="group in visibleRecipeGroups" :key="group.key">
                <div v-if="recipeGroup === group.key" class="ranch-recipes">
                  <div v-if="!group.rows.length" class="muted">这个分类暂时没有能开工的配方。</div>
                  <div v-for="row in group.rows" :key="row.key" class="ranch-recipe">
                    <span class="ranch-recipe-text">
                      {{ row.label }}：{{ row.inputText }} → {{ row.outputName }}
                      <span class="muted">
                        <template v-if="row.price">（{{ row.price }} 金币 · 约 {{ formatRemain(row.durationMs) }}）</template>
                        <template v-else>（约 {{ formatRemain(row.durationMs) }}）</template>
                      </span>
                    </span>
                    <button class="link" type="button" :disabled="acting || !row.canStart" @click="row.start()">[开工]</button>
                    <span v-if="row.missing" class="muted">{{ row.missing }}</span>
                  </div>
                </div>
              </template>
            </details>
            </div>
          </template>
        </template>
        <details class="panel ws-stock-panel">
          <summary class="ws-stock-summary">
            <b>原料与成品</b>
            <span class="muted">{{ workshopItems.length }} 种 · 加工品按品质分堆，展开可卖</span>
          </summary>
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
        </details>
      </div>

      <div v-show="activeTab === '鱼塘'" class="pond-page">
        <div class="level">鱼塘</div>
        <div class="sub">
          {{ currency }} {{ snapshot?.coins ?? 0 }}
          <span class="muted">· 把钓到的鱼暂养催肥，长到商品规格再捕</span>
        </div>
        <div class="rule"></div>
        <template v-if="!pond?.built">
          <section class="panel">
            <div class="panel-title">还没有鱼塘<span class="muted">需要农场 {{ pond?.buildLevel }} 级</span></div>
            <div class="row">
              <button class="link" type="button" :disabled="acting || !pond?.canBuild" @click="buildPond">[建鱼塘 {{ pond?.buildCost }}]</button>
              <span class="muted">{{ pond?.buildMissing }}</span>
            </div>
          </section>
        </template>
        <template v-else>
          <section class="panel pond-order-panel">
            <div class="panel-title">今日鱼塘订单<span class="muted">每天一条，交付后明天再来</span></div>
            <div v-if="pond?.orderDone" class="muted">今天的订单已经交付，明天再来。</div>
            <div v-else-if="!pond?.order" class="muted">放几条鱼进塘，就会有订单。</div>
            <div v-else class="pond-order">
              <div>
                要 {{ pond.order.qty }} 条{{ pond.order.fishName }}（单条 ≥{{ pond.order.minWeightLabel }}）
                <span class="muted">· 塘里 {{ pond.order.have }} 条</span>
              </div>
              <div class="row">
                <span>{{ pond.order.rewardCoins }} 金币</span>
                <button class="link" type="button" :disabled="acting || !pond.order.canDeliver" @click="deliverPondOrder">[交付订单]</button>
              </div>
            </div>
          </section>
          <section class="panel">
            <div class="panel-title">水质<span class="muted">{{ pond?.water?.score }}/100 · {{ pond?.statusLabel }}</span></div>
            <div class="pond-hint">{{ pond?.hint }}</div>
            <details class="pond-details">
              <summary>详细水质</summary>
              <div class="pond-water-grid">
                <span>水温 {{ pond?.water?.temperatureLabel }}</span>
                <span>溶氧 {{ pond?.water?.oxygen }}/100</span>
                <span>清洁 {{ pond?.water?.cleanliness }}/100</span>
                <span>氨氮 {{ pond?.water?.ammonia }}/100</span>
                <span>容量 {{ pond?.stockCount }}/{{ pond?.capacity }} · {{ pond?.speciesCount }}/{{ pond?.speciesCap }} 种</span>
              </div>
              <div class="ranch-meters">
                <span class="ranch-meter"><em>溶氧</em><span class="ranch-meter-track"><i :style="{ width: `${pond?.water?.oxygen ?? 0}%` }"></i></span></span>
                <span class="ranch-meter"><em>清洁</em><span class="ranch-meter-track clean"><i :style="{ width: `${pond?.water?.cleanliness ?? 0}%` }"></i></span></span>
                <span class="ranch-meter"><em>氨氮</em><span class="ranch-meter-track dirty"><i :style="{ width: `${pond?.water?.ammonia ?? 0}%` }"></i></span></span>
              </div>
            </details>
            <div class="ranch-actions">
              <button class="link" type="button" :disabled="acting || !pond?.canFeed" @click="feedPond">[投喂 {{ pond?.feedToday }}/{{ pond?.feedMax }}]</button>
              <button class="link" type="button" :disabled="acting || !pond?.canClean" @click="cleanPond">[清理鱼塘]</button>
              <button v-if="!pond?.aerator" class="link" type="button" :disabled="acting" @click="installPondAerator">[装增氧机 {{ pond?.aeratorCost }}]</button>
              <button v-if="!pond?.plants" class="link" type="button" :disabled="acting" @click="plantPondWeeds">[种水草 {{ pond?.plantsCost }}]</button>
              <button class="link" type="button" :disabled="acting" @click="togglePondFry">[留种：{{ pond?.autoFry ? "开" : "关" }}]</button>
              <span class="muted">留种后每天自动来 3 条鱼苗</span>
            </div>
          </section>
          <section class="panel">
            <div class="panel-title">放养<span class="muted">背包里的鱼，一次最多 5 条；最多混养 4 种</span></div>
            <div v-if="!(pond?.stock.length)" class="muted">背包里没有鱼，先去钓鱼。</div>
            <div v-for="item in pond?.stock ?? []" :key="item.fishId" class="pond-stock">
              <span>{{ item.name }} ×{{ item.have }}</span>
              <button class="link" type="button" :disabled="acting" @click="stockPond(item.fishId, Math.min(5, item.have))">
                [放养×{{ Math.min(5, item.have) }}]
              </button>
            </div>
          </section>
          <section class="panel">
            <div class="panel-title">塘里<span class="muted">长到商品规格才能捕，捕上来按重量/品质进鱼篓</span></div>
            <div v-if="!(pond?.fish.length)" class="muted">还没有放鱼。</div>
            <div v-for="fish in pond?.fish ?? []" :key="fish.fishId" class="pond-fish">
              <div>
                <strong>{{ fish.name }}</strong> ×{{ fish.count }}
                <span class="muted">· {{ fish.qualityLabel }} · {{ fish.weightLabel }} / {{ fish.marketWeightLabel }}</span>
              </div>
              <div class="ranch-meter">
                <em>成长</em>
                <span class="ranch-meter-track"><i :style="{ width: `${fish.progressPct}%` }"></i></span>
                <span class="muted">{{ fish.progressPct }}%</span>
              </div>
              <button class="link" type="button" :disabled="acting || !fish.ready" @click="harvestPond(fish.fishId)">[捕捞]</button>
            </div>
          </section>
          <section class="panel">
            <div class="panel-title">鱼塘手记<span class="muted">做过的事会记在这里</span></div>
            <div class="pond-milestones">
              <span v-for="item in pond?.milestones ?? []" :key="item.key" class="pond-milestone" :class="{ have: item.done }">
                {{ item.done ? "✓" : "·" }} {{ item.label }}
              </span>
            </div>
          </section>
        </template>
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
            <span class="muted">
              {{ classicLayout
                ? "按等级解锁小溪/河流/江面/大海；鱼竿、自动与图鉴平铺"
                : "先选钓点/鱼饵再抛竿；专精、钓具、自动和图鉴奖励都收进折叠" }}
            </span>
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
              <div class="fish-hero-meta">
                🌦️ {{ fishing.condition.summary }}
                <span class="muted">· 季节/昼夜/雨天只改鱼种权重，不会禁钓</span>
              </div>
              <div v-if="fishingPet" class="fish-hero-meta">
                🐾 {{ fishingPet.name }}在岸边帮忙 · 钓鱼地岗位 Lv{{ fishingPet.fishingJobLevel }}
                <span class="muted">（每天结算一次）</span>
              </div>
            </div>
            <div class="fish-hero-side">
              <span class="fish-auto-pill" :class="{ on: fishing.auto.enabled }">
                自动 {{ fishing.auto.enabled ? "开着" : "关着" }}
              </span>
              <span class="muted">每 {{ fishing.autoIntervalLabel }}</span>
            </div>
          </div>

          <details class="fish-card fish-card--skill fish-fold fish-section fish-section--skills">
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

          <div class="fish-card fish-card--play fish-section fish-section--play">
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
              :disabled="acting || !!fishing.session || (snapshot?.coins ?? 0) < fishing.maintenanceCost"
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
            :session="fishing.session"
            :active="activeTab === '钓鱼'"
            @resolve="resolveFishing"
            @notice="notice = $event"
          />
          <div v-if="!fishing.session" class="row">
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

          <details
            class="fish-card fish-card--quest fish-fold fish-section"
            :open="classicLayout ? true : undefined"
          >
            <summary @click="guardClassicFold">
              <span class="fish-fold-title">委托与比赛</span>
              <span class="muted">
                可接 {{ fishing.quests.offers.length }} 条 · 进行中 {{ fishing.quests.active.length }}/2
                <template v-if="fishing.quests.results.some((r) => !r.claimed)"> · 有比赛奖励可领</template>
              </span>
            </summary>
            <div class="fishing-quest-block">
              <div class="fishing-quest-head">
                <b>NPC 委托</b>
                <span class="muted">每天 3 条，今天有效；同时最多接 2 条</span>
                <button
                  v-if="fishing.quests.canRefresh"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="refreshFishingCommissions"
                >
                  [换一批]
                </button>
                <span v-else class="muted">今天已换过</span>
              </div>
              <div v-for="offer in fishing.quests.offers" :key="offer.id" class="fishing-quest">
                <span v-if="showSpotTag(offer)" class="fishing-spot-tag">{{ offer.spotLabel }}</span>
                <span>{{ offer.title }}</span>
                <span class="muted">
                  奖励 {{ offer.rewardCoins }} 金币 · 鱼饵×{{ offer.rewardBait }} · 经验 {{ offer.rewardExp }}
                </span>
                <button
                  class="link"
                  type="button"
                  :disabled="acting || fishing.quests.active.length >= 2"
                  @click="acceptFishingCommission(offer.id)"
                >
                  [接委托]
                </button>
              </div>
              <div v-if="!fishing.quests.offers.length" class="muted">今天的委托都接完了，明天再来。</div>
            </div>
            <div class="fishing-quest-block">
              <div class="fishing-quest-head">
                <b>进行中</b>
                <span class="muted">{{ fishing.quests.active.length }}/2</span>
              </div>
              <div v-for="commission in fishing.quests.active" :key="commission.id" class="fishing-quest">
                <span v-if="showSpotTag(commission)" class="fishing-spot-tag">{{ commission.spotLabel }}</span>
                <span>{{ commission.title }}</span>
                <span class="muted">进度 {{ commission.progress }}/{{ commission.qty }}</span>
                <button
                  v-if="commission.complete"
                  class="link"
                  type="button"
                  :disabled="acting"
                  @click="claimFishingCommission(commission.id)"
                >
                  [交委托]
                </button>
                <span v-else class="muted">
                  还差 {{ commission.qty - commission.progress }} 条（{{
                    (commission.minWeightTenths / 10).toFixed(1)
                  }}kg 以上）
                </span>
              </div>
              <div v-if="!fishing.quests.active.length" class="muted">还没接委托。</div>
            </div>
            <div v-if="fishing.quests.tournament" class="fishing-quest-block">
              <div class="fishing-quest-head">
                <b>今日最重一条</b>
                <span class="muted">和 {{ fishing.quests.tournament.npcName }} 比 · 明天结算</span>
              </div>
              <div class="fishing-quest">
                <span v-if="fishing.quests.tournament.bestTenths">
                  你的最重：{{ fishing.quests.tournament.bestFishName }}
                  {{ (fishing.quests.tournament.bestTenths / 10).toFixed(1) }}kg
                </span>
                <span v-else class="muted">你今天还没上鱼</span>
                <span class="muted">
                  {{ fishing.quests.tournament.npcName }} 目前
                  {{ (fishing.quests.tournament.npcBestTenths / 10).toFixed(1) }}kg
                </span>
              </div>
            </div>
            <div
              v-for="result in fishing.quests.results.filter((r) => !r.claimed)"
              :key="result.day"
              class="fishing-quest"
            >
              <span>{{ result.message }}</span>
              <span class="muted">
                金币+{{ result.rewardCoins }} · 鱼饵+{{ result.rewardBait }} · 经验+{{ result.rewardExp }}
              </span>
              <button
                class="link"
                type="button"
                :disabled="acting"
                @click="claimFishingTournament(result.day)"
              >
                [领奖]
              </button>
            </div>
          </details>

          <details
            class="fish-card fish-card--gear fish-fold fish-section fish-section--gear"
            :open="classicLayout ? true : undefined"
          >
            <summary @click="guardClassicFold">
              <span class="fish-fold-title">钓具</span>
              <span class="muted">
                {{ fishing.rodLabel }} · 磨损 {{ fishing.rodWear }}/{{ fishing.rodMaxWear }}
                <template v-if="fishing.needsMaintenance"> · 可保养 {{ fishing.maintenanceCost }}</template>
              </span>
              <button
                v-if="fishing.needsMaintenance"
                class="link"
                type="button"
                :disabled="acting || !!fishing.session"
                @click.stop="maintainRod(fishing.rod)"
              >
                [快速保养 {{ fishing.maintenanceCost }}]
              </button>
            </summary>
            <div class="ranch-recipe-group">鱼竿（框越小越难；金色变异按半条鱼王价收购）</div>
          <div class="ranch-recipes">
            <div v-for="rod in fishing.rods" :key="rod.kind" class="ranch-recipe">
              <span class="ranch-recipe-text">
                {{ rod.label }}：框 {{ rod.framePct }}% · 自动 {{ rod.autoMinutes }} 分钟 · 磨损
                {{ rod.wear }}/{{ rod.maxWear }}
              </span>
              <span v-if="rod.current" class="muted">使用中</span>
              <button
                v-else-if="rod.owned"
                class="link"
                type="button"
                :disabled="acting || !!fishing.session"
                @click="switchRod(rod.kind)"
              >
                [切换]
              </button>
              <span v-else-if="fishing.level < rod.level" class="muted">
                需要钓鱼等级 {{ rod.level }} 级
              </span>
              <button
                v-else
                class="link"
                type="button"
                :disabled="acting || !rod.canBuy || !!fishing.session"
                @click="buyRod(rod.kind)"
              >
                [换竿 {{ rod.cost }}]
              </button>
              <button
                v-if="rod.needsMaintenance && rod.owned"
                class="link"
                type="button"
                :disabled="acting || !!fishing.session || (snapshot?.coins ?? 0) < rod.maintenanceCost"
                @click="maintainRod(rod.kind)"
              >
                [保养 {{ rod.maintenanceCost }}]
              </button>
              <span v-if="!rod.owned && (snapshot?.level ?? 0) < rod.level" class="muted">
                需要 {{ rod.level }} 级
              </span>
              <span class="muted rod-profit">{{ rod.profitLabel }}</span>
            </div>
          </div>
          </details>

          <details
            class="fish-card fish-card--auto fish-fold fish-section fish-section--auto"
            :open="classicLayout ? true : undefined"
          >
            <summary @click="guardClassicFold">
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

          <div class="fish-card fish-card--codex fish-section fish-section--codex">
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
            <details
              class="fish-fold fish-fold--inline"
              :open="classicLayout ? true : undefined"
            >
              <summary @click="guardClassicFold">分钓点进度与鱼王 trophy</summary>
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
              <span v-if="item.conditionHint" class="muted">{{ item.conditionHint }}</span>
              <span v-if="item.caught" class="muted">
                最大 {{ (item.bestWeightTenths / 10).toFixed(1) }}kg · {{ item.bestQualityLabel }}
              </span>
              <span v-if="item.caught" class="muted">×{{ item.count }}</span>
              <span v-if="item.golden" class="golden-badge">金色 ×{{ item.goldenCount }}</span>
            </div>
          </div>
          </div>

          <div class="fish-card fish-card--keep fish-section fish-section--keep">
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

      <div v-show="activeTab === '宠物'">
        <section class="panel">
          <div class="panel-title">
            宠物
            <span class="muted">狗狗/猫猫 · 像素侧视，待机与互动有小动作</span>
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
              <p class="muted">不会死亡；生病可治疗。可派去牧场或钓鱼地；鱼塘等扩展位以后再做。</p>
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
                <div class="pet-portrait-stage">
                  <img
                    class="pet-portrait-img"
                    :class="[petAnim, { 'pet-still': pet.boarded || pet.sickness }]"
                    :src="pet.image"
                    :alt="pet.species"
                  />
                  <span v-if="petEmote" class="pet-emote">{{ petEmote }}</span>
                </div>
                <div class="pet-name">{{ pet.speciesEmoji }} {{ pet.name }}</div>
                <div class="pet-chips">
                  <span class="pet-chip">{{ pet.breedLabel }}</span>
                  <span class="pet-chip">{{ pet.temperamentLabel }}</span>
                  <span class="pet-chip">{{ pet.stageLabel }}</span>
                  <span class="pet-chip">Lv{{ pet.level }}</span>
                  <span class="pet-chip">第 {{ pet.ageDays }} 天</span>
                  <span class="pet-chip">{{ pet.moodLabel }}</span>
                </div>
                <section class="pet-card">
                  <div class="pet-card-head">
                    <b>状态</b>
                    <span class="muted">经验 {{ pet.exp }}/{{ pet.expNeed }}</span>
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
                </section>

              </div>

              <div class="pet-info">
                <div v-if="pet.boarded" class="warn pet-note">
                  {{ pet.name }}在宠物院子寄养中，每月自动扣 {{ pet.boardingFee }} 金币。
                  <button
                    class="link"
                    type="button"
                    :disabled="acting || activePets.length >= 3"
                    @click="setPetBoarded(pet.id, false)"
                  >
                    [接回]
                  </button>
                </div>
                <div v-if="pet.sickness" class="warn pet-note">
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

                <template v-if="!pet.boarded">
                  <section class="pet-card">
                    <div class="pet-card-head">
                      <b>日常</b>
                    </div>
                    <div class="row pet-actions">
                      <template v-if="pet.species === 'cat'">
                        <button
                          class="link"
                          type="button"
                          :disabled="acting || !petCare || petCare.catFood === 0"
                          @click="feedPetFood('cat_food')"
                        >
                          [喂猫粮]
                        </button>
                        <span class="muted">×{{ petCare?.catFood ?? 0 }}</span>
                        <button
                          class="link"
                          type="button"
                          :disabled="acting || !petCare || petCare.catCan === 0"
                          @click="feedPetFood('cat_can')"
                        >
                          [喂猫罐头]
                        </button>
                        <span class="muted">×{{ petCare?.catCan ?? 0 }}</span>
                        <button
                          class="link"
                          type="button"
                          :disabled="acting || !fishItems.length"
                          @click="feedPet"
                        >
                          [喂生鱼]
                        </button>
                        <span class="muted">×{{ fishItems.length }}</span>
                      </template>
                      <template v-else>
                        <button
                          class="link"
                          type="button"
                          :disabled="acting || !petCare || petCare.dogFood === 0"
                          @click="feedPetFood('dog_food')"
                        >
                          [喂狗粮]
                        </button>
                        <span class="muted">×{{ petCare?.dogFood ?? 0 }}</span>
                        <button
                          class="link"
                          type="button"
                          :disabled="acting || !petCare || petCare.dogBone === 0"
                          @click="feedPetFood('dog_bone')"
                        >
                          [喂狗骨头]
                        </button>
                        <span class="muted">×{{ petCare?.dogBone ?? 0 }}</span>
                        <span class="muted">狗不吃生鱼</span>
                      </template>
                    </div>
                    <div class="row pet-actions">
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
                    <div class="row pet-actions">
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
                      <span class="muted pet-action-hint">{{ pet.activeDescription }}</span>
                    </div>
                  </section>

                  <section class="pet-card">
                    <div class="pet-card-head">
                      <b>地点与岗位</b>
                      <span class="muted">每天结算一个岗位</span>
                    </div>
                    <div class="row pet-actions">
                      <button class="rate-btn" :class="{ active: pet.location === 'farm' }" type="button" :disabled="acting" @click="setPetLocation(pet.id, 'farm')">农场</button>
                      <button class="rate-btn" :class="{ active: pet.location === 'ranch' }" type="button" :disabled="acting" @click="setPetLocation(pet.id, 'ranch')">牧场</button>
                      <button class="rate-btn" :class="{ active: pet.location === 'fishing' }" type="button" :disabled="acting" @click="setPetLocation(pet.id, 'fishing')">钓鱼地</button>
                      <span class="muted">鱼塘岗位以后再加</span>
                    </div>
                    <div class="pet-job-line">
                      <template v-if="pet.location === 'ranch'">
                        牧场岗位 Lv{{ pet.jobLevel }}
                        <span class="muted">（{{ pet.jobExp }}/{{ pet.jobNeed }}）· 每天结算一次看护收益</span>
                      </template>
                      <template v-else-if="pet.location === 'fishing'">
                        钓鱼地岗位 Lv{{ pet.fishingJobLevel }}
                        <span class="muted">（{{ pet.fishingJobExp }}/{{ pet.fishingJobNeed }}）· 每天结算一次岸边收益</span>
                      </template>
                      <template v-else>
                        牧场岗位 Lv{{ pet.jobLevel }} · 钓鱼地岗位 Lv{{ pet.fishingJobLevel }}
                        <span class="muted">· 派去对应地点后每天有收益</span>
                      </template>
                    </div>
                    <details class="pet-subdetails">
                      <summary>岗位说明</summary>
                      <p class="muted">
                        挖到的种子直接进仓库；守卫会减少偷菜损失，狗狗有低概率把偷菜贼吓跑。
                        钓鱼地岗位：猫有概率叼回一条已点亮图鉴的鱼，狗会找鱼饵；每天只结算一个岗位。
                      </p>
                    </details>
                  </section>
                </template>

                <section class="pet-card pet-profile">
                  <div class="pet-card-head">
                    <b>档案</b>
                    <span class="muted">个性 / 小癖好 / 品种技能</span>
                  </div>
                  <div class="pet-tags">
                    <span class="pet-tag">个性：{{ pet.personality }}</span>
                    <span class="pet-tag">小癖好：{{ pet.quirk }}</span>
                    <span class="pet-tag">品种技能 · {{ pet.skill }}</span>
                    <span class="pet-tag">正在：{{ pet.activityLabel }}</span>
                  </div>
                  <div class="pet-quote">“{{ pet.behavior }}”</div>
                  <details class="pet-subdetails">
                    <summary>详细说明</summary>
                    <p class="muted">
                      {{ pet.quirkLine }} · {{ pet.activityLine }}
                      <span v-if="pet.activityStreak >= 2">· 连续 {{ pet.activityStreak }} 天</span>
                    </p>
                    <p class="muted">{{ pet.skillDescription }}</p>
                  </details>
                </section>

                <details class="pet-card pet-manage pet-fold">
                  <summary>
                    <b>管理</b>
                    <span class="muted">改名 / 寄养 / 放生</span>
                  </summary>
                  <div class="row pet-actions">
                    <input v-model="renameName" maxlength="12" placeholder="改个名字" />
                    <button class="link" type="button" :disabled="acting || !renameName.trim()" @click="renamePet">
                      [改名]
                    </button>
                  </div>
                  <div class="row pet-actions">
                    <button
                      v-if="!pet.boarded"
                      class="link"
                      type="button"
                      :disabled="acting"
                      @click="setPetBoarded(pet.id, true)"
                    >
                      [送去寄养 {{ pet.boardingFee }}/月]
                    </button>
                    <button class="link" type="button" :disabled="acting" @click="releasePet(pet.id, pet.name)">
                      [放生]
                    </button>
                  </div>
                  <p class="muted pet-manage-note">不会死亡；寄养每月自动扣费，可随时接回。</p>
                </details>
              </div>
            </div>
          </div>
          <div v-if="petCare" class="pet-care">
            <div class="pet-care-title">宠物用品</div>
            <div class="pet-food">
              <div class="pet-care-title">
                宠物食品
                <span class="muted">买来放家里，喂的时候不花钱</span>
              </div>
              <div class="pet-food-grid">
                <span v-for="food in petFoods" :key="food.kind" class="pet-food-item">
                  {{ food.label }} ×{{ food.count }}
                  <button
                    class="link"
                    type="button"
                    :disabled="acting"
                    @click="buyPetFood(food.kind)"
                  >
                    [买 {{ food.cost }}]
                  </button>
                </span>
              </div>
            </div>
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

          <details v-if="pet" class="pet-training pet-fold">
            <summary>
              <b>训练中心</b>
              <span v-if="pet.trainingCenter" class="muted">训练点 {{ pet.trainingSpent }}/{{ pet.trainingCap }}</span>
              <span v-else class="muted">未建设</span>
            </summary>
            <template v-if="pet.trainingCenter">
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
            </template>
            <p v-else class="muted">农场 12 级可建设训练中心（20,000 金币），之后才能训练守护/挖掘/拆家/陪伴。</p>
          </details>

          <details v-if="pet" class="pet-training pet-talents pet-fold">
            <summary>
              <b>天赋</b>
              <span class="muted">Lv5 / Lv12 / Lv20 · 每层二选一，可随时改选</span>
            </summary>
            <div class="pet-talent-grid">
              <div v-for="tier in pet.talentTiers" :key="tier.tier" class="pet-talent-tier">
                <div class="pet-talent-tier-title">
                  Lv{{ tier.threshold }} 天赋
                  <span v-if="!tier.unlocked" class="muted">· 还差 {{ tier.threshold - pet.level }} 级</span>
                </div>
                <div class="pet-talent-options">
                  <button
                    v-for="talent in tier.options"
                    :key="talent.id"
                    class="pet-talent-btn"
                    :class="{ active: talent.selected }"
                    type="button"
                    :disabled="acting || !tier.unlocked"
                    @click="choosePetTalent(talent.id)"
                  >
                    <strong>{{ talent.label }}</strong>
                    <span class="muted">{{ talent.description }}</span>
                  </button>
                </div>
              </div>
            </div>
          </details>
        </section>
      </div>

      <div v-show="activeTab === '标本室'">
        <section class="panel">
          <div class="panel-title">
            标本室
            <span class="muted">
              已收藏 {{ snapshot?.specimenUnique ?? 0 }}/{{ specimens.length }} 种 · 消耗作物和金币换稀有种子
            </span>
          </div>
          <p v-if="specimenFeatured" class="specimen-featured">
            今日推荐：{{ specimenFeatured.icon }} {{ specimenFeatured.name }}标本 · 金币奖励 +50%
          </p>
          <div class="specimen-milestones">
            <span class="muted">收藏奖励</span>
            <span
              v-for="milestone in specimenMilestones"
              :key="milestone.unique"
              class="specimen-milestone"
            >
              {{ milestone.unique }}种 · {{ milestone.rewardCoins }}金币
              <button
                v-if="!milestone.claimed"
                class="link"
                type="button"
                :disabled="acting || !milestone.claimable"
                @click="claimSpecimenMilestone(milestone.unique)"
              >
                {{ milestone.claimable ? "[领取]" : "[还差]" }}
              </button>
              <span v-else class="muted">✓ 已领</span>
            </span>
          </div>
          <div class="specimen-grid">
            <div
              v-for="specimen in specimens"
              :key="specimen.cropId"
              class="specimen-card"
              :class="{ featured: specimen.featured, crafted: specimen.crafted > 0 }"
            >
              <div class="specimen-head">
                <strong>{{ specimen.icon }} {{ specimen.name }}标本</strong>
                <span v-if="specimen.featured" class="specimen-tag">今日推荐</span>
                <span v-else-if="specimen.crafted > 0" class="specimen-tag">已收藏 ×{{ specimen.crafted }}</span>
                <span v-else class="muted">{{ specimen.level }}级作物</span>
              </div>
              <div>
                材料：{{ specimen.name }}×{{ specimen.needQty }}（有 {{ specimen.haveQty }}）
                + {{ specimen.coins }}金币
              </div>
              <div class="muted">
                奖励：{{ specimen.rewardHint }} + {{ Math.floor(specimen.coins / 2) }}金币
                <span v-if="specimen.featured">（推荐 +50%）</span>
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

        <div v-if="specimenResult" class="specimen-result-overlay" @click.self="specimenResult = null">
          <div class="specimen-result-card">
            <div class="panel-title">标本完成</div>
            <div class="specimen-glass-frame">
              <span class="specimen-glass-icon">{{ specimenResultCrop?.icon ?? "🌱" }}</span>
            </div>
            <strong>{{ specimenResult.cropName }}标本</strong>
            <p v-if="specimenResult.featured" class="specimen-featured">今日推荐，金币奖励 +50%</p>
            <p>
              获得 <b>{{ specimenResult.seedName }}种子 ×{{ specimenResult.seedQty }}</b>
              <span v-if="specimenResult.coins > 0"> + {{ specimenResult.coins }}金币</span>
            </p>
            <button class="link" type="button" @click="specimenResult = null">[收下]</button>
          </div>
        </div>
      </div>

      <div class="rule"></div>
      <h2 class="section-title">
        牧场广播
        <span class="muted">最新 10 条</span>
        <span class="muted">（只读记录）</span>
      </h2>
      <div class="log">
        <div v-for="(entry, index) in logLines" :key="`${entry.at}-${index}`" class="log-line">
          ({{ entry.stamp }}) {{ entry.text }}
        </div>
        <div v-if="!logLines.length" class="muted">还没有广播。</div>
      </div>

      <div v-if="npcTalk" class="npc-scene-overlay" @click.self="npcTalk = null">
        <div class="npc-scene-card">
          <div class="npc-scene-title">{{ npcTalk.npcName }} · {{ npcTalk.title }}</div>
          <p class="npc-scene-text">{{ npcTalk.text }}</p>
          <div v-if="npcTalk.reward || npcTalk.favor" class="npc-scene-reward">
            <span v-if="npcTalk.reward">收下：{{ npcTalk.reward }}</span>
            <span v-if="npcTalk.favor">好感 +{{ npcTalk.favor }}</span>
          </div>
          <button class="link" type="button" @click="npcTalk = null">[收下]</button>
        </div>
      </div>
    </main>

    <footer>
      <div class="footer-links">
        <button class="link" type="button" @click="openFarmSection('farm-warehouse')">仓库</button>
        |
        <button class="link" type="button" @click="openFarmSection('farm-shop')">种子铺</button>
        |
        <button class="link" type="button" @click="openCodex">图鉴</button>
        |
        <button class="link" type="button" @click="openFarmSection('farm-achievements')">成就</button>
        |
        <button class="link" type="button" @click="openPlayerSection('player-settings')">设置</button>
        |
        <button class="link" type="button" @click="showHelp">说明</button>
      </div>
      <div class="footer-links">
        <button class="link" type="button" @click="selectTab('我的农场')">我的农场</button>
        |
        <button class="link" type="button" @click="selectTab('邻居农场')">邻居农场</button>
        |
        <button class="link" type="button" @click="openFarmSection('farm-daily')">签到</button>
      </div>
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
        <div v-if="selectedFish.conditionHint" class="muted">
          条件加成：{{ selectedFish.conditionHint }}
        </div>
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
        <div class="catch-actions">
          <button class="link" type="button" @click="dismissCatch">[收下]</button>
          <span class="muted">按 F 收下并续钓</span>
        </div>
      </div>
    </div>
  </div>
</template>
