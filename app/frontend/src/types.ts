export type PlotStatus = "growing" | "ready" | "withered";
export type ItemKind = "seed" | "produce" | "ranch" | "craft" | "fish";
export type RanchAnimalKind = "chicken" | "duck" | "cow" | "sheep";
export type RanchRecipe = "cheese" | "butter" | "cake" | "yarn" | "salted_duck_egg";

export type FacilityKind = "sprinkler" | "harvester" | "windmill" | "greenhouse" | "trainingCenter";
export type PestKind = "grass" | "bug";
export type DisasterKind = "drought" | "flood" | "hail" | "frost" | "blight" | "typhoon";
export type Season = "spring" | "summer" | "autumn" | "winter";
export type Region = "north" | "south";
export type RegionAffinity = "all" | "north" | "south";
export type PetSpecies = "dog" | "cat";
export type PetBreed =
  | "corgi"
  | "german_shepherd"
  | "golden_retriever"
  | "border_collie"
  | "shiba"
  | "rural_dog"
  | "bichon"
  | "domestic_cat"
  | "ragdoll"
  | "golden_shaded"
  | "silver_shaded"
  | "blue_cat"
  | "coffee_cat";
export type PetStage = "baby" | "adult" | "senior";
export type PetLocation = "farm" | "ranch" | "pond" | "fishing";

export type PetTalent =
  | "gatekeeper"
  | "fetch"
  | "night_watch"
  | "herding"
  | "iron_wall"
  | "leader"
  | "sharp_claw"
  | "clingy"
  | "sly"
  | "fishing_eye"
  | "treasure_nose"
  | "keen_eye";

export interface PetTalentView {
  id: PetTalent;
  label: string;
  description: string;
  selected: boolean;
}

export interface PetTalentTierView {
  tier: number;
  threshold: number;
  unlocked: boolean;
  options: PetTalentView[];
}
export type PetSupplyKind = "dog_bed" | "cat_bed" | "bowl" | "dog_toy" | "cat_toy" | "medicine";
export type PetTrainingKind = "guard" | "dig" | "mischief" | "companion";

export interface Resistances {
  drought: number;
  flood: number;
  cold: number;
  pest: number;
}

export interface SeedMetrics {
  profitPerHour: number;
  expPerHour: number;
  resistanceTotal: number;
}

export type ReviewStatus = "accepted" | "revised" | "rejected";

export interface SeedReview {
  crop: CropDef;
  status: ReviewStatus;
  reasons: string[];
  metrics: SeedMetrics;
}

export interface RanchAnimalView {
  kind: RanchAnimalKind;
  label: string;
  emoji: string;
  name: string;
  satiety: number;
  health: number;
  sick: boolean;
  product: string;
  productReadyAt: number;
  remainingMs: number;
  canCollect: boolean;
  canFeed: boolean;
  canHeal: boolean;
  quality: number;
  qualityLabel: string;
  generation: number;
  traitLabel: string;
  traitHint: string;
  ageDays: number;
  specialName: string;
  specialHint: string;
  lineId: number;
  lineLabel: string;
  young: boolean;
  youngRemainingMs: number;
  inbred: boolean;
  parents: string;
  breedReady: boolean;
  breedRemainingMs: number;
}

export interface RanchBreedPreview {
  ok: boolean;
  kind: RanchAnimalKind;
  kindLabel: string;
  quality: number;
  qualityLabel: string;
  generation: number;
  lineId: number;
  lineLabel: string;
  traitLabel: string;
  traitHint: string;
  inbred: boolean;
  parents: string;
  warnings: string[];
  missing: string[];
}

export interface RanchPenView {
  index: number;
  animal: RanchAnimalView | null;
  cleanliness: number;
  canClean: boolean;
}

export interface RanchRecipeView {
  recipe: RanchRecipe;
  label: string;
  inputText: string;
  durationMs: number;
  outputName: string;
  outputPrice: number;
  canStart: boolean;
  missing: string;
}

export type EquipmentKind = "manual" | "stone_mill" | "juicer" | "boiling_pot";
export type EquipmentRecipe =
  | "corn_starch"
  | "coffee_powder"
  | "ginseng_slice"
  | "lingzhi_powder"
  | "cane_juice"
  | "strawberry_juice"
  | "grape_juice"
  | "watermelon_juice"
  | "sugar"
  | "cream"
  | "strawberry_jam";

export interface WorkshopJobView {
  kind: string;
  label: string;
  isCrop: boolean;
  outputName: string;
  outputPrice: number;
  quality: number;
  qualityLabel: string;
  readyAt: number;
  remainingMs: number;
  canCollect: boolean;
}

export interface WorkshopSlotView {
  index: number;
  equipment: EquipmentKind | null;
  equipmentLabel: string;
  equipmentEmoji: string;
  equipmentLevel: number;
  equipmentMaxLevel: number;
  equipmentSpeedPct: number;
  equipmentYieldPct: number;
  upgradeCost: number;
  canUpgrade: boolean;
  upgradeMissing: string;
  job: WorkshopJobView | null;
}

export interface WorkshopEquipmentView {
  kind: EquipmentKind;
  label: string;
  emoji: string;
  cost: number;
  level: number;
  installed: boolean;
  canInstall: boolean;
  missing: string;
}

export interface WorkshopRecipeView {
  recipe: EquipmentRecipe;
  equipment: EquipmentKind;
  equipmentLabel: string;
  label: string;
  inputText: string;
  durationMs: number;
  outputName: string;
  outputPrice: number;
  canStart: boolean;
  missing: string;
}

export interface CropRecipeView {
  cropId: string;
  name: string;
  qty: number;
  need: number;
  outputName: string;
  outputPrice: number;
  durationMs: number;
  canStart: boolean;
  missing: string;
}

export type BaitRecipe = "corn" | "dough" | "worm" | "lure";
export type FishRecipe = "jerky" | "ball" | "soup" | "oil";

export interface BaitRecipeView {
  recipe: BaitRecipe;
  label: string;
  inputText: string;
  outputText: string;
  durationMs: number;
  canStart: boolean;
  missing: string;
}

export interface FishRecipeView {
  recipe: FishRecipe;
  label: string;
  inputText: string;
  outputText: string;
  durationMs: number;
  canStart: boolean;
  missing: string;
}

export interface WorkshopView {
  built: boolean;
  canBuild: boolean;
  cost: number;
  level: number;
  slots: number;
  maxSlots: number;
  canExpand: boolean;
  expandCost: number;
  expandLevel: number;
  slotViews: WorkshopSlotView[];
  equipmentOptions: WorkshopEquipmentView[];
  manualRecipes: RanchRecipeView[];
  baitRecipes: BaitRecipeView[];
  fishRecipes: FishRecipeView[];
  cropRecipes: CropRecipeView[];
  equipmentRecipes: WorkshopRecipeView[];
}

export interface RanchOrdersView {
  day: string;
  offers: RanchOrderView[];
  active: RanchOrderView[];
  maxActive: number;
}

export interface RanchOrderView {
  id: string;
  npcName: string;
  baseKey: string;
  itemName: string;
  quality: number;
  qualityLabel: string;
  qty: number;
  expiresDay: string;
  rewardCoins: number;
  rewardExp: number;
  rewardFeed: number;
  have: number;
  canDeliver: boolean;
}

export interface RanchCodexTraitView {
  key: string;
  label: string;
  have: boolean;
}

export interface RanchCodexKindView {
  kind: RanchAnimalKind;
  label: string;
  emoji: string;
  traits: RanchCodexTraitView[];
  done: number;
  total: number;
}

export interface RanchCodexSpecialView {
  key: string;
  name: string;
  have: boolean;
}

export interface RanchCodexView {
  kinds: RanchCodexKindView[];
  specials: RanchCodexSpecialView[];
  animalDone: number;
  animalTotal: number;
  specialDone: number;
  specialTotal: number;
}

export interface RanchView {
  pens: RanchPenView[];
  canExpand: boolean;
  expandCost: number;
  maxPens: number;
  feedCount: number;
  orders: RanchOrdersView;
  codex: RanchCodexView;
}

export interface PetCareView {
  dogBed: boolean;
  catBed: boolean;
  dogBedBroken: boolean;
  catBedBroken: boolean;
  bowls: number;
  dogToy: boolean;
  catToy: boolean;
  dogToyBroken: boolean;
  catToyBroken: boolean;
  medicine: number;
  dogBedCost: number;
  catBedCost: number;
  bowlCost: number;
  dogToyCost: number;
  catToyCost: number;
  medicineCost: number;
  dogBedRepair: number;
  catBedRepair: number;
  dogToyRepair: number;
  catToyRepair: number;
}

export interface PetView {
  id: string;
  name: string;
  species: PetSpecies;
  speciesLabel: string;
  speciesEmoji: string;
  breed: PetBreed;
  breedLabel: string;
  image: string;
  temperament: string;
  temperamentLabel: string;
  personality: string;
  quirk: string;
  quirkLine: string;
  skill: string;
  skillDescription: string;
  activeSkill: string;
  activeDescription: string;
  skillRemainingMs: number;
  canUseSkill: boolean;
  activity: string;
  activityLabel: string;
  activityLine: string;
  activityStreak: number;
  mischiefChance: number;
  behavior: string;
  digSkill: number;
  digExp: number;
  digNeed: number;
  guardSkill: number;
  guardExp: number;
  guardNeed: number;
  guardTrain: number;
  digTrain: number;
  mischiefTrain: number;
  companionTrain: number;
  trainingSpent: number;
  trainingCap: number;
  trainingCenter: boolean;
  boarded: boolean;
  boardingFee: number;
  stage: PetStage;
  stageLabel: string;
  ageDays: number;
  level: number;
  exp: number;
  expNeed: number;
  bond: number;
  satiety: number;
  energy: number;
  mood: number;
  moodLabel: string;
  health: number;
  sickness: string | null;
  location: PetLocation;
  locationLabel: string;
  jobLevel: number;
  jobExp: number;
  jobNeed: number;
  fishingJobLevel: number;
  fishingJobExp: number;
  fishingJobNeed: number;
  talentTiers: PetTalentTierView[];
  canFeed: boolean;
  canTouch: boolean;
  canPlay: boolean;
  canHeal: boolean;
  canDig: boolean;
  touchRemainingMs: number;
  playRemainingMs: number;
  digRemainingMs: number;
}

export interface CropDef {
  id: string;
  name: string;
  icon: string;
  unlockLevel: number;
  growMinutes: number;
  regrowMinutes: number;
  seasons: number;
  yieldPerSeason: number;
  seedPrice: number;
  sellPrice: number;
  exp: number;
  note: string;
  sowSeasons: Season[];
  region: RegionAffinity;
  resistances: Resistances;
}

export interface Nouns {
  farm: string;
  owner: string;
  plot: string;
  currency: string;
}

export interface ContentPack {
  schema: number;
  crops: CropDef[];
  nouns: Nouns;
}

export interface PestView {
  kind: PestKind;
  label: string;
  since: number;
}

export interface DisasterView {
  kind: DisasterKind;
  label: string;
  since: number;
  endsAt: number;
  yieldPenalty: number;
  growthSlow: number;
  mitigation: string;
  canMitigate: boolean;
}

export interface CropView {
  cropId: string;
  name: string;
  icon: string;
  status: PlotStatus;
  readyAt: number;
  season: number;
  seasons: number;
  remaining: number;
  watered: boolean;
  canWater: boolean;
  canHarvest: boolean;
  pest: PestView | null;
  canClearPest: boolean;
  disaster: DisasterView | null;
  canMitigate: boolean;
  offSeason: boolean;
  greenhouse: boolean;
  withered: boolean;
  regionLabel: string;
}

export interface PlotEffectsView {
  expPct: number;
  coinPct: number;
  yieldPct: number;
  luckPct: number;
}

export interface PlotView {
  index: number;
  label: string;
  unlocked: boolean;
  kind: "field" | "greenhouse";
  unlockLevel: number;
  unlockCost: number;
  tier: number;
  tierName: string;
  effects: PlotEffectsView;
  upgradeCost: number;
  upgradeLevel: number;
  maxTier: boolean;
  canUpgrade: boolean;
  crop: CropView | null;
}

export interface FacilityView {
  kind: FacilityKind;
  name: string;
  owned: boolean;
  unlockLevel: number;
  cost: number;
  count: number;
  maxCount: number;
  nextCost: number;
  damaged: number;
}

export interface ItemStack {
  key: string;
  name: string;
  qty: number;
  kind: ItemKind;
  sellable: boolean;
  unitPrice: number;
  detail?: string;
  locked: boolean;
}

export interface LogEntry {
  at: number;
  text: string;
}

export interface DailyTaskView {
  id: number;
  name: string;
  desc: string;
  progress: number;
  target: number;
  claimed: boolean;
  claimable: boolean;
  rewardCoins: number;
  rewardExp: number;
}

export interface DailyView {
  day: string;
  signIn: boolean;
  tasks: DailyTaskView[];
}

export interface FarmOrderView {
  id: string;
  npcName: string;
  cropId: string;
  cropName: string;
  qty: number;
  expiresDay: string;
  rewardCoins: number;
  rewardExp: number;
  have: number;
  canDeliver: boolean;
}

export interface FarmOrdersView {
  day: string;
  offers: FarmOrderView[];
  active: FarmOrderView[];
  maxActive: number;
}

export interface MysteryOfferView {
  id: string;
  kind: string;
  name: string;
  desc: string;
  tag: string;
  price: number;
  rarity: string;
  rarityLabel: string;
  featured: boolean;
  sold: boolean;
  affordable: boolean;
}

export interface MysteryShopView {
  day: string;
  offers: MysteryOfferView[];
  legendary: boolean;
}

export interface LevelRewardView {
  level: number;
  coins: number;
  text: string;
  claimed: boolean;
  claimable: boolean;
}

export interface CodexView {
  cropId: string;
  name: string;
  icon: string;
  discovered: boolean;
  firstAt: number;
  total: number;
  best: number;
}

export interface AchievementView {
  id: string;
  name: string;
  desc: string;
  progress: number;
  target: number;
  earned: boolean;
  rewardCoins: number;
  rewardExp: number;
}

export interface NpcPlotView {
  index: number;
  label: string;
  cropName: string;
  icon: string;
  status: "empty" | "growing" | "ready";
  readyAt: number;
  remaining: number;
  canHelp: boolean;
  canSteal: boolean;
  helped: boolean;
}

export interface NpcRanchAnimalView {
  index: number;
  kind: RanchAnimalKind;
  label: string;
  emoji: string;
  product: string;
  readyAt: number;
  remainingMs: number;
  canHelp: boolean;
  canSteal: boolean;
  helped: boolean;
}

export interface NpcView {
  id: string;
  name: string;
  level: number;
  friendship: number;
  relation: string;
  plots: NpcPlotView[];
  ranch: NpcRanchAnimalView[];
}

export interface NpcRoomView {
  npcId: string;
  name: string;
  relation: string;
  friendship: number;
  threshold: number;
  unlocked: boolean;
  room: RoomView;
}

export interface NpcMessageView {
  at: number;
  npcName: string;
  text: string;
  fromPlayer: boolean;
}

export interface SpecimenView {
  cropId: string;
  name: string;
  icon: string;
  level: number;
  needQty: number;
  coins: number;
  haveQty: number;
  canCraft: boolean;
  rewardHint: string;
  crafted: number;
  featured: boolean;
}

export interface SpecimenMilestoneView {
  unique: number;
  rewardCoins: number;
  claimed: boolean;
  claimable: boolean;
}

export type FishingSpot = "creek" | "river" | "jiang" | "sea";
export type RodKind = "wood" | "bamboo" | "fiberglass" | "carbon";
export type BaitQuality = "normal" | "fine" | "prime" | "master";
export type BaitType = "dough" | "corn" | "worm" | "lure";
export type FishRarity = "common" | "uncommon" | "rare" | "king";
export type FishPersonality = "lazy" | "normal" | "active" | "fierce";
export type FishBehavior = "bottom" | "surface" | "migratory" | "burst";

export interface FishingParams {
  framePct: number;
  frameSpeedPct: number;
  frameAccelPct: number;
  fishSpeedPct: number;
  fishJerkPct: number;
  fishEscapePct: number;
  fleeRadiusPct: number;
  turnPct: number;
  targetMinMs: number;
  targetMaxMs: number;
  personality: FishPersonality;
  personalityLabel: string;
  catchMs: number;
  progressGain: number;
  progressLoss: number;
  behavior: FishBehavior;
  behaviorLabel: string;
  behaviorHint: string;
  tensionGainPct: number;
}

export interface FishingSpotView {
  spot: FishingSpot;
  label: string;
  emoji: string;
  level: number;
  unlocked: boolean;
  fishCount: number;
}

export interface RodView {
  kind: RodKind;
  label: string;
  cost: number;
  level: number;
  owned: boolean;
  current: boolean;
  canBuy: boolean;
  framePct: number;
  autoMinutes: number;
  wear: number;
  maxWear: number;
  profitLabel: string;
  maintenanceCost: number;
  needsMaintenance: boolean;
}

export interface FishingSessionView {
  id: number;
  spot: FishingSpot;
  spotLabel: string;
  fishId: string;
  fishName: string;
  facesRight: boolean;
  price: number;
  difficulty: number;
  isKing: boolean;
  isGolden: boolean;
  expiresAt: number;
  remainingMs: number;
  expired: boolean;
  params: FishingParams;
}

export interface FishingAutoView {
  enabled: boolean;
  spot: FishingSpot;
  spotLabel: string;
  rod: RodKind;
  rodLabel: string;
  baitQuality: BaitQuality;
  baitQualityLabel: string;
  baitType: BaitType;
  baitTypeLabel: string;
  intervalMs: number;
  intervalLabel: string;
  nextAt: number;
  canToggle: boolean;
}

export interface BaitView {
  baitType: BaitType;
  quality: BaitQuality;
  label: string;
  price: number;
  count: number;
  qualityBonus: number;
  weightBonus: number;
}

export interface BaitTypeView {
  baitType: BaitType;
  label: string;
}

export interface AutoCatchView {
  fishId: string;
  name: string;
  count: number;
}

export interface AutoReportView {
  at: number;
  baitBought: number;
  baitUsed: number;
  coinsSpent: number;
  catches: AutoCatchView[];
  totalCaught: number;
}

export interface FishCodexView {
  id: string;
  name: string;
  spot: FishingSpot;
  spotLabel: string;
  rarity: FishRarity;
  rarityLabel: string;
  price: number;
  difficulty: number;
  caught: boolean;
  count: number;
  bestWeightTenths: number;
  bestQuality: number;
  bestQualityLabel: string;
  bestPrice: number;
  golden: boolean;
  goldenCount: number;
  manualCount: number;
  manualBestWeightTenths: number;
  manualBestQuality: number;
  manualBestQualityLabel: string;
  manualBestPrice: number;
  autoCount: number;
  autoBestWeightTenths: number;
  autoBestQuality: number;
  autoBestQualityLabel: string;
  autoBestPrice: number;
  conditionHint: string;
}

export interface FishingConditionView {
  seasonLabel: string;
  timeLabel: string;
  weatherLabel: string;
  summary: string;
}

export interface FishCatchView {
  fishId: string;
  name: string;
  weightTenths: number;
  quality: number;
  qualityLabel: string;
  price: number;
  marketPct: number;
  isKing: boolean;
  golden: boolean;
  isRecord: boolean;
  at: number;
}

export interface SpotProgressView {
  spot: FishingSpot;
  label: string;
  caught: number;
  total: number;
  rewardCoins: number;
  rewardBaitLabel: string;
  claimed: boolean;
  complete: boolean;
}

export interface KingTrophyView {
  fishId: string;
  name: string;
  spotLabel: string;
  caught: boolean;
  count: number;
  bestWeightTenths: number;
}

export interface SkillView {
  id: string;
  name: string;
  desc: string;
  level: number;
  maxLevel: number;
  cost: number;
  canUpgrade: boolean;
}

export type DisciplineKind = "planting" | "ranching" | "fishing" | "processing";

export interface DisciplineView {
  kind: DisciplineKind;
  label: string;
  level: number;
  exp: number;
  expNeed: number;
  points: number;
  skills: SkillView[];
}

export interface BuffView {
  id: string;
  name: string;
  desc: string;
  remainingMs: number;
}

export interface BuffItemView {
  key: string;
  name: string;
  desc: string;
}

export interface SkillsView {
  disciplines: DisciplineView[];
  buffs: BuffView[];
  buffItems: BuffItemView[];
}

export interface FishingCommissionView {
  id: string;
  npcName: string;
  title: string;
  fishName: string;
  spotLabel: string;
  minWeightTenths: number;
  qty: number;
  progress: number;
  complete: boolean;
  rewardCoins: number;
  rewardBait: number;
  rewardExp: number;
}

export interface FishingTournamentView {
  day: string;
  npcName: string;
  npcBestTenths: number;
  bestTenths: number;
  bestFishName: string;
}

export interface FishingTournamentResultView {
  day: string;
  message: string;
  rank: number;
  rewardCoins: number;
  rewardBait: number;
  rewardExp: number;
  claimed: boolean;
  bestTenths: number;
  npcBestTenths: number;
}

export interface FishingQuestView {
  offers: FishingCommissionView[];
  active: FishingCommissionView[];
  tournament: FishingTournamentView | null;
  results: FishingTournamentResultView[];
  canRefresh: boolean;
  refreshUsed: boolean;
}

export interface FishingView {
  rod: RodKind;
  rodLabel: string;
  framePct: number;
  level: number;
  exp: number;
  expNeed: number;
  skillPoints: number;
  skills: SkillView[];
  autoIntervalMs: number;
  autoIntervalLabel: string;
  bait: number;
  baitPrice: number;
  activeBait: BaitQuality;
  activeBaitLabel: string;
  activeBaitType: BaitType;
  activeBaitTypeLabel: string;
  baitTypes: BaitTypeView[];
  baitCounts: BaitView[];
  rodWear: number;
  rodMaxWear: number;
  maintenanceCost: number;
  needsMaintenance: boolean;
  lockedFish: string[];
  spots: FishingSpotView[];
  rods: RodView[];
  session: FishingSessionView | null;
  auto: FishingAutoView;
  codex: FishCodexView[];
  codexCaught: number;
  codexTotal: number;
  spotProgress: SpotProgressView[];
  goldenCaught: number;
  goldenTotal: number;
  kingTrophies: KingTrophyView[];
  fullCodexClaimed: boolean;
  lastCatch: FishCatchView | null;
  lastAutoReport: AutoReportView | null;
  quests: FishingQuestView;
  condition: FishingConditionView;
}

export interface RoomFurniture {
  id: string;
  def: string;
  x: number;
  y: number;
  rot: number;
  fish: string[];
}

export interface RoomDoor {
  id: string;
  wall: "left" | "right";
  x: number;
  w: number;
  h: number;
  style: string;
}

export interface RoomWindow {
  id: string;
  wall: "left" | "right";
  x: number;
  y: number;
  w: number;
  h: number;
  style: string;
}

export interface RoomSurfaceItem {
  id: string;
  def: string;
  supportId: string;
}

export interface RoomLayout {
  floor: string;
  wall: string;
  door: RoomDoor | null;
  windows: RoomWindow[];
  furniture: RoomFurniture[];
  surfaceItems: RoomSurfaceItem[];
}

export interface RoomStyleView {
  id: string;
  name: string;
  locked: boolean;
  unlockHint: string;
  price: number;
}

export interface RoomFurnitureDefView {
  id: string;
  name: string;
  width: number;
  depth: number;
  layer: number;
  collision: boolean;
  kind: string;
  capacity: number;
  sprite: string | null;
  spriteRotations: number;
  price: number;
  source: string;
}

export interface RoomSurfaceDefView {
  id: string;
  name: string;
  sprite: string;
  spriteRotations: number;
  supportKinds: string[];
}

export interface RoomView {
  roomId: string;
  label: string;
  width: number;
  height: number;
  grid: number;
  layout: RoomLayout;
  floors: RoomStyleView[];
  walls: RoomStyleView[];
  windowStyles: RoomWindowStyleView[];
  catalog: RoomFurnitureDefView[];
  surfaceCatalog: RoomSurfaceDefView[];
  availableFurniture: Record<string, number>;
  ownedFurniture: Record<string, number>;
}

export interface DecorOfferView {
  id: string;
  kind: string;
  name: string;
  price: number;
  sold: boolean;
  owned: boolean;
}

export interface DecorShopView {
  day: string;
  offers: DecorOfferView[];
}

export interface RoomWindowStyleView {
  id: string;
  name: string;
  width: number;
  height: number;
}

export interface Snapshot {
  rev: number;
  now: number;
  season: Season;
  seasonLabel: string;
  seasonEmoji: string;
  region: Region;
  regionLabel: string;
  farmName: string;
  ownerName: string;
  createdAt: number;
  level: number;
  exp: number;
  expNeed: number;
  coins: number;
  plots: PlotView[];
  inventory: ItemStack[];
  facilities: FacilityView[];
  onboarded: boolean;
  daily: DailyView;
  orders: FarmOrdersView;
  mystery: MysteryShopView;
  levelRewards: LevelRewardView[];
  levelRewardsClaimable: number;
  codex: CodexView[];
  codexDiscovered: number;
  achievements: AchievementView[];
  achievementsEarned: number;
  specimens: SpecimenView[];
  specimenUnique: number;
  specimenFeatured: string | null;
  specimenMilestones: SpecimenMilestoneView[];
  pet: PetView | null;
  pets: PetView[];
  petCare: PetCareView;
  ranch: RanchView;
  workshop: WorkshopView;
  fishing: FishingView;
  room: RoomView;
  extraRoom: RoomView | null;
  decorShop: DecorShopView;
  npcRooms: NpcRoomView[];
  skills: SkillsView;
  boardingDebt: number;
  expRate: number;
  npcs: NpcView[];
  npcMessages: NpcMessageView[];
  stealsLeft: number;
  helpsLeft: number;
  ranchStealsLeft: number;
  ranchHelpsLeft: number;
  log: LogEntry[];
  contentCrops: number;
}

export interface SpecimenResultEffect {
  kind: "specimen";
  cropId: string;
  cropName: string;
  seedId: string;
  seedName: string;
  seedQty: number;
  coins: number;
  featured: boolean;
}

export type Effect =
  | { kind: "log"; text: string }
  | { kind: "levelUp"; level: number }
  | { kind: "achievement"; id: string; name: string }
  | SpecimenResultEffect;

export interface DispatchReply {
  snapshot: Snapshot;
  effects: Effect[];
  saveError: string | null;
}

export type Action =
  | { type: "plant"; plot: number; cropId: string }
  | { type: "plantRareSeed"; plot: number; cropId: string }
  | { type: "water"; plot: number }
  | { type: "harvest"; plot: number }
  | { type: "harvestAll" }
  | { type: "clear"; plot: number }
  | { type: "clearPest"; plot: number }
  | { type: "unlockPlot"; plot: number }
  | { type: "upgradePlot"; plot: number }
  | { type: "mitigateDisaster"; plot: number }
  | { type: "setRegion"; region: Region }
  | { type: "craftSpecimen"; cropId: string }
  | { type: "claimSpecimenMilestone"; unique: number }
  | { type: "repairWindmills" }
  | { type: "adoptPet"; name: string; breed: PetBreed; board: boolean }
  | { type: "feedPet"; petId: string; cropId: string }
  | { type: "petCompanion"; petId: string }
  | { type: "playWithPet"; petId: string }
  | { type: "healPet"; petId: string }
  | { type: "digWithPet"; petId: string }
  | { type: "setPetLocation"; petId: string; location: PetLocation }
  | { type: "renamePet"; petId: string; name: string }
  | { type: "setPetBoarded"; petId: string; boarded: boolean }
  | { type: "releasePet"; petId: string }
  | { type: "settleBoardingDebt" }
  | { type: "buyRanchAnimal"; pen: number; kind: RanchAnimalKind }
  | { type: "feedRanchAnimal"; pen: number }
  | { type: "cleanRanchPen"; pen: number }
  | { type: "collectRanchProduct"; pen: number }
  | { type: "healRanchAnimal"; pen: number }
  | { type: "expandRanch" }
  | { type: "feedRanchPremium"; pen: number }
  | { type: "buyRanchFeed" }
  | { type: "breedRanchPair"; a: number; b: number }
  | { type: "renameRanchAnimal"; pen: number; name: string }
  | { type: "acceptRanchOrder"; id: string }
  | { type: "deliverRanchOrder"; id: string }
  | { type: "buildWorkshop" }
  | { type: "expandWorkshop" }
  | { type: "installEquipment"; slot: number; kind: EquipmentKind }
  | { type: "upgradeEquipment"; slot: number }
  | { type: "startRanchCraft"; recipe: RanchRecipe }
  | { type: "startCropCraft"; cropId: string }
  | { type: "startEquipmentCraft"; recipe: EquipmentRecipe }
  | { type: "startBaitCraft"; recipe: BaitRecipe }
  | { type: "startFishCraft"; recipe: FishRecipe }
  | { type: "collectCraft"; slot: number }
  | { type: "buyBait"; qty: number; quality?: BaitQuality; baitType?: BaitType }
  | { type: "setBaitType"; baitType: BaitType }
  | { type: "buyRod"; rod: RodKind }
  | { type: "switchRod"; rod: RodKind }
  | { type: "castLine"; spot: FishingSpot; quality?: BaitQuality }
  | { type: "resolveFishing"; success: boolean }
  | { type: "acceptFishingCommission"; id: string }
  | { type: "claimFishingCommission"; id: string }
  | { type: "refreshFishingCommissions" }
  | { type: "claimFishingTournament"; day: string }
  | { type: "setAutoFishing"; spot: FishingSpot; enabled: boolean; rod?: RodKind; baitQuality?: BaitQuality; baitType?: BaitType }
  | { type: "upgradeFishingSkill"; skill: string }
  | { type: "upgradeSkill"; discipline: DisciplineKind; skill: string }
  | { type: "useBuffItem"; key: string }
  | { type: "toggleFishLock"; fishId: string }
  | { type: "sellAllFish" }
  | { type: "maintainRod"; rod: RodKind }
  | { type: "buyPetSupply"; kind: PetSupplyKind }
  | { type: "repairPetSupply"; kind: PetSupplyKind }
  | { type: "usePetMedicine"; petId: string }
  | { type: "usePetSkill"; petId: string }
  | { type: "trainPet"; petId: string; kind: PetTrainingKind }
  | { type: "choosePetTalent"; petId: string; talent: PetTalent }
  | { type: "buySeed"; cropId: string; qty: number }
  | { type: "sell"; key: string; qty: number }
  | { type: "buyFacility"; kind: FacilityKind }
  | { type: "helpNpc"; npcId: string; plot: number }
  | { type: "stealNpc"; npcId: string; plot: number }
  | { type: "helpNpcRanch"; npcId: string; index: number }
  | { type: "stealNpcRanch"; npcId: string; index: number }
  | { type: "npcGreeting"; npcId: string }
  | { type: "claimSignIn" }
  | { type: "updateProfile"; ownerName: string }
  | { type: "claimDaily"; id: number }
  | { type: "acceptFarmOrder"; id: string }
  | { type: "deliverFarmOrder"; id: string }
  | { type: "buyMysteryOffer"; id: string }
  | { type: "buyFurniture"; def: string }
  | { type: "buyDecor"; id: string }
  | { type: "useFarmCard" }
  | { type: "claimLevelReward"; level: number }
  | { type: "updateSettings"; expRate: number }
  | { type: "showGuide" }
  | { type: "dismissGuide" }
  | { type: "saveRoomLayout"; room: string; layout: RoomLayout }
  | { type: "moveFishToAquarium"; room: string; furnitureId: string; fishId: string }
  | { type: "removeFishFromAquarium"; room: string; furnitureId: string; fishId: string }
  | { type: "unlockExtraRoom" }
  | { type: "broadcast"; text: string };

export interface ImportReply {
  snapshot: Snapshot;
  content: ContentPack;
  warning: string | null;
}

export interface EconomyPreview {
  cropId: string;
  name: string;
  coinsPerHour: number;
  expPerHour: number;
  paybackMinutes: number;
  warning: string | null;
}

export interface ContentPreview {
  issues: string[];
  crops: EconomyPreview[];
}

export interface AppInfo {
  version: string;
  coreVersion: string;
  savePath: string;
  schema: number;
  bootWarning: string | null;
}
