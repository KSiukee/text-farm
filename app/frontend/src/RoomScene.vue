<script setup lang="ts">
import { asset } from "./assets";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type {
  PetView,
  RoomDoor,
  RoomFurniture,
  RoomFurnitureDefView,
  RoomLayout,
  RoomSurfaceItem,
  RoomView,
  RoomWindow,
} from "./types";

const props = defineProps<{
  room: RoomView;
  active: boolean;
  extraUnlocked: boolean;
  extraCost: number;
  pets: PetView[];
  readonly?: boolean;
}>();
const emit = defineEmits<{
  save: [layout: RoomLayout];
  notice: [text: string];
  travel: [];
  unlock: [];
  aquarium: [id: string];
}>();

const CW = 900;
const CH = 480;
const TW = 42;
const TH = 21;
const OX = 460;
const OY = 140;
const WALL_H = 112;
const CANVAS_SCALE = 2;
const CAMERA_MIN = 0.7;
const CAMERA_MAX = 2.2;
const camera = ref({ zoom: 1.05, panX: 0, panY: 0 });
const windowBackdrop = ref<HTMLImageElement | null>(null);
const windowBackdropPath = computed(() => {
  if (props.room.roomId === "extra_1") return asset("/windows/extra.png");
  if (props.room.roomId === "living") return asset("/windows/living.png");
  return asset("/windows/garden.png");
});

function loadWindowBackdrop() {
  const path = windowBackdropPath.value;
  const img = new Image();
  img.onload = () => {
    windowBackdrop.value = img;
    draw();
  };
  img.onerror = () => {
    windowBackdrop.value = null;
  };
  img.src = path;
}
const panning = ref<{ x: number; y: number; panX: number; panY: number } | null>(null);
// 真机排障用的一行 HUD：装修模式下显示最近一次触摸事件走到了哪一步
const touchHud = ref("");
const zoomPct = computed(() => Math.round(camera.value.zoom * 100));
const sceneCenter = computed(() => {
  const floorW = ((props.room.width + props.room.height) * TW) / 2;
  const floorH = ((props.room.width + props.room.height) * TH) / 2;
  return {
    x: OX + ((props.room.width - props.room.height) * TW) / 4,
    y: (OY - WALL_H - 24 + OY + floorH + 12) / 2,
    floorW,
    floorH,
  };
});
const fitZoom = computed(() =>
  Math.max(
    CAMERA_MIN,
    Math.min(
      1.6,
      Math.min(CW / (sceneCenter.value.floorW + 120), CH / (WALL_H + sceneCenter.value.floorH + 100)),
    ),
  ),
);

const canvas = ref<HTMLCanvasElement | null>(null);
const editing = ref(false);
const selectedId = ref<string | null>(null);
const selectedWindowId = ref<string | null>(null);
const selectedDoorId = ref<string | null>(null);
const draft = ref<RoomLayout>(clone(props.room.layout));
const undoStack = ref<RoomLayout[]>([]);
const drag = ref<{ id: string; ox: number; oy: number } | null>(null);
const dragWindow = ref<{ id: string; ox: number; oy: number } | null>(null);
const dragDoor = ref<{ id: string; ox: number } | null>(null);
const spriteCache = new Map<string, HTMLImageElement>();
const silhouetteCache = new Map<string, HTMLCanvasElement>();
const petCache = new Map<string, HTMLImageElement>();
const petFootCache = new Map<string, { x: number; y: number; width: number }>();
const fishCache = new Map<string, HTMLImageElement>();
let seq = 0;

type PetState = "idle" | "walk" | "sit" | "sleep";
type PetActor = {
  petId: string;
  name: string;
  image: string;
  x: number;
  y: number;
  dir: "left" | "right";
  state: PetState;
  timer: number;
  phase: number;
  target: P | null;
};

const PET_HEIGHT = 36;
const petActors = ref<PetActor[]>([]);
let petTimer: number | null = null;
let lastPetTick = 0;
let nextEntryAt = 0;

const activePetCount = computed(() => props.pets.filter((pet) => !pet.boarded).length);
const hasAquariumFish = computed(() =>
  draft.value.furniture.some((item) => (item.fish?.length ?? 0) > 0),
);

type P = { x: number; y: number };

function clone(layout: RoomLayout): RoomLayout {
  return JSON.parse(JSON.stringify(layout)) as RoomLayout;
}

function iso(x: number, y: number): P {
  return { x: OX + ((x - y) * TW) / 2, y: OY + ((x + y) * TH) / 2 };
}

function rawPoint(event: PointerEvent): P {
  const el = canvas.value!;
  const rect = el.getBoundingClientRect();
  return {
    x: ((event.clientX - rect.left) * CW) / rect.width,
    y: ((event.clientY - rect.top) * CH) / rect.height,
  };
}

function screenToWorld(p: P): P {
  return {
    x: (p.x - CW / 2 - camera.value.panX) / camera.value.zoom + sceneCenter.value.x,
    y: (p.y - CH / 2 - camera.value.panY) / camera.value.zoom + sceneCenter.value.y,
  };
}

function setZoom(next: number) {
  camera.value.zoom = Math.max(CAMERA_MIN, Math.min(CAMERA_MAX, next));
  draw();
}

function zoomBy(delta: number) {
  setZoom(camera.value.zoom + delta);
}

function resetCamera() {
  camera.value.zoom = fitZoom.value;
  camera.value.panX = 0;
  camera.value.panY = 0;
  draw();
}

function onWheel(event: WheelEvent) {
  event.preventDefault();
  setZoom(camera.value.zoom + (event.deltaY < 0 ? 0.1 : -0.1));
}

const defs = computed(() => new Map(props.room.catalog.map((def) => [def.id, def])));
const floorColor = computed(() => {
  const id = draft.value.floor;
  if (id === "tile") {
    return {
      base: "#d7cfbf",
      alt: "#d3cbbb",
      line: "rgba(120,104,82,.18)",
      grain: "rgba(255,255,255,.10)",
    };
  }
  if (id === "stone") {
    return {
      base: "#aab3ab",
      alt: "#a6afa7",
      line: "rgba(60,76,68,.20)",
      grain: "rgba(235,240,230,.12)",
    };
  }
  if (id === "terracotta") {
    return {
      base: "#b9785d",
      alt: "#b37258",
      line: "rgba(92,50,34,.22)",
      grain: "rgba(255,226,196,.12)",
    };
  }
  if (id === "tatami") {
    return {
      base: "#c8b47b",
      alt: "#c2ae75",
      line: "rgba(100,86,48,.20)",
      grain: "rgba(255,247,210,.14)",
    };
  }
  if (id === "cafe") {
    return {
      base: "#8f6a4a",
      alt: "#896446",
      line: "rgba(48,30,18,.25)",
      grain: "rgba(230,205,175,.12)",
    };
  }
  return {
    base: "#c79c68",
    alt: "#c39763",
    line: "rgba(100,65,35,.20)",
    grain: "rgba(255,239,205,.13)",
  };
});
const wallColor = computed(() => {
  const id = draft.value.wall;
  if (id === "sage") return { top: "#c7d1bc", bottom: "#aebba3", trim: "#8e9b83" };
  if (id === "sky") return { top: "#cad9e1", bottom: "#b2c5d0", trim: "#8fa6b4" };
  if (id === "dusk") return { top: "#c4b4bd", bottom: "#a997a4", trim: "#887783" };
  if (id === "linen") return { top: "#e4d6c2", bottom: "#d2c0a6", trim: "#b8a58a" };
  return { top: "#f0e7d7", bottom: "#ddd2be", trim: "#c5b79e" };
});
const lockedStyles = computed(() =>
  [...props.room.floors, ...props.room.walls].filter((style) => style.locked),
);
const WALL_HEIGHT_UNITS = 48;
const WALL_HEIGHT_TILES = 6;

const sorted = computed(() =>
  [...draft.value.furniture].sort((a, b) => {
    const da = defs.value.get(a.def);
    const db = defs.value.get(b.def);
    const la = da?.layer ?? 2;
    const lb = db?.layer ?? 2;
    if (la !== lb) return la - lb;
    return a.y + (da?.depth ?? 1) * props.room.grid - (b.y + (db?.depth ?? 1) * props.room.grid);
  }),
);

const selected = computed(() => draft.value.furniture.find((item) => item.id === selectedId.value) ?? null);
const surfaceDefs = computed(() => new Map(props.room.surfaceCatalog.map((def) => [def.id, def])));
const surfaceForSelected = computed(() =>
  selectedId.value
    ? draft.value.surfaceItems.find((item) => item.supportId === selectedId.value) ?? null
    : null,
);
const selectedWindow = computed(() => draft.value.windows.find((item) => item.id === selectedWindowId.value) ?? null);
const selectedDoor = computed(() =>
  draft.value.door && draft.value.door.id === selectedDoorId.value ? draft.value.door : null,
);

const windowStyles = computed(() => new Map(props.room.windowStyles.map((style) => [style.id, style])));

function wallLength(wall: "left" | "right"): number {
  return (wall === "left" ? props.room.height : props.room.width) * props.room.grid;
}

function windowInvalid(window: RoomWindow): boolean {
  const len = wallLength(window.wall);
  if (window.w < 8 || window.h < 8) return true;
  if (window.x < 0 || window.y < 0) return true;
  if (window.x + window.w > len || window.y + window.h > WALL_HEIGHT_UNITS) return true;
  return draft.value.windows.some((other) => {
    if (other.id === window.id || other.wall !== window.wall) return false;
    return window.x < other.x + other.w &&
      other.x < window.x + window.w &&
      window.y < other.y + other.h &&
      other.y < window.y + window.h;
  });
}

function doorPoints(door: RoomDoor): P[] {
  const grid = props.room.grid;
  const u0 = door.x / grid;
  const u1 = (door.x + door.w) / grid;
  const v1 = door.h / grid;
  return [
    wallPoint(door.wall, u0, 0),
    wallPoint(door.wall, u1, 0),
    wallPoint(door.wall, u1, v1),
    wallPoint(door.wall, u0, v1),
  ];
}

function doorInvalid(door: RoomDoor): boolean {
  const len = wallLength(door.wall);
  if (door.x < 0 || door.w < 8 || door.h < 8) return true;
  if (door.x + door.w > len || door.h > WALL_HEIGHT_UNITS) return true;
  return draft.value.windows.some((window) => window.wall === door.wall &&
    door.x < window.x + window.w &&
    window.x < door.x + door.w &&
    window.y < door.h);
}

function findWindowSpot(wall: "left" | "right", w: number, h: number): { x: number; y: number } | null {  const len = wallLength(wall);
  for (const y of [16, 24, 8, 32, 40, 0]) {
    if (y + h > WALL_HEIGHT_UNITS) continue;
    for (let x = 0; x + w <= len; x += 8) {
      const candidate: RoomWindow = {
        id: "__candidate__",
        wall,
        x,
        y,
        w,
        h,
        style: "small",
      };
      if (!draft.value.windows.some((other) => other.wall === wall &&
        candidate.x < other.x + other.w &&
        other.x < candidate.x + candidate.w &&
        candidate.y < other.y + other.h &&
        other.y < candidate.y + candidate.h)) {
        return { x, y };
      }
    }
  }
  return null;
}

function pushUndo() {
  undoStack.value.push(clone(draft.value));
  if (undoStack.value.length > 30) undoStack.value.shift();
}

function changeFloor(event: Event) {
  pushUndo();
  draft.value.floor = (event.target as HTMLSelectElement).value;
}

function changeWall(event: Event) {
  pushUndo();
  draft.value.wall = (event.target as HTMLSelectElement).value;
}

function undo() {
  const previous = undoStack.value.pop();
  if (!previous) return;
  draft.value = previous;
  selectedId.value = null;
  selectedWindowId.value = null;
  selectedDoorId.value = null;
  drag.value = null;
  dragWindow.value = null;
  dragDoor.value = null;
  draw();
}

function addWindow(styleId: string) {
  if (draft.value.windows.length >= 5) {
    emit("notice", "每间房最多 5 扇窗");
    return;
  }
  const style = windowStyles.value.get(styleId);
  if (!style) return;
  pushUndo();
  const wall: "left" | "right" = "right";
  const spot = findWindowSpot(wall, style.width, style.height);
  if (!spot) {
    emit("notice", "这面墙放不下这扇窗了");
    return;
  }
  const item: RoomWindow = {
    id: `w-${Date.now()}-${seq++}`,
    wall,
    x: spot.x,
    y: spot.y,
    w: style.width,
    h: style.height,
    style: style.id,
  };
  draft.value.windows.push(item);
  selectedWindowId.value = item.id;
  selectedId.value = null;
}

function moveWindowWall(wall: "left" | "right") {
  const item = selectedWindow.value;
  if (!item) return;
  pushUndo();
  item.wall = wall;
  item.x = Math.max(0, Math.min(item.x, wallLength(wall) - item.w));
}

function moveDoorWall(wall: "left" | "right") {
  const door = selectedDoor.value;
  if (!door) return;
  pushUndo();
  door.wall = wall;
  door.x = Math.max(0, Math.min(door.x, wallLength(wall) - door.w));
}

function resizeWindow(dw: number, dh: number) {
  const item = selectedWindow.value;
  if (!item) return;
  pushUndo();
  item.w = Math.max(8, Math.min(wallLength(item.wall) - item.x, item.w + dw));
  item.h = Math.max(8, Math.min(WALL_HEIGHT_UNITS - item.y, item.h + dh));
}

function storeWindow() {
  if (!selectedWindowId.value) return;
  pushUndo();
  draft.value.windows = draft.value.windows.filter((item) => item.id !== selectedWindowId.value);
  selectedWindowId.value = null;
}

function startEdit() {
  if (props.readonly) return;
  undoStack.value = [];
  draft.value = clone(props.room.layout);
  editing.value = true;
  selectedId.value = null;
  selectedWindowId.value = null;
  selectedDoorId.value = null;
}

function cancelEdit() {
  undoStack.value = [];
  draft.value = clone(props.room.layout);
  editing.value = false;
  selectedId.value = null;
  selectedWindowId.value = null;
  selectedDoorId.value = null;
  emit("notice", "已取消装修，布局没有变化");
}

function saveEdit() {
  undoStack.value = [];
  editing.value = false;
  drag.value = null;
  dragWindow.value = null;
  dragDoor.value = null;
  selectedId.value = null;
  selectedWindowId.value = null;
  selectedDoorId.value = null;
  emit("save", clone(draft.value));
}

function availableFor(def: RoomFurnitureDefView): number {
  const base = props.room.availableFurniture?.[def.id] ?? 0;
  const placed = draft.value.furniture.filter((item) => item.def === def.id).length;
  const initial = props.room.layout.furniture.filter((item) => item.def === def.id).length;
  return Math.max(0, base - (placed - initial));
}

function addFurniture(def: RoomFurnitureDefView) {
  if (availableFor(def) <= 0) return;
  pushUndo();
  const g = props.room.grid;
  const w = def.width * g;
  const d = def.depth * g;
  const item: RoomFurniture = {
    id: `f-${Date.now()}-${seq++}`,
    def: def.id,
    x: Math.max(0, Math.round((props.room.width * g - w) / 2)),
    y: Math.max(0, Math.round((props.room.height * g - d) / 2)),
    rot: 0,
    fish: [],
  };
  draft.value.furniture.push(item);
  selectedId.value = item.id;
  selectedWindowId.value = null;
}

function addSurfaceItem() {
  const def = props.room.surfaceCatalog[0];
  if (!def) return;
  pushUndo();
  const support =
    (selected.value &&
    def.supportKinds.includes(defs.value.get(selected.value.def)?.kind ?? "") &&
    !surfaceForSelected.value
      ? selected.value
      : null) ??
    draft.value.furniture.find(
      (item) =>
        def.supportKinds.includes(defs.value.get(item.def)?.kind ?? "") &&
        !draft.value.surfaceItems.some((surface) => surface.supportId === item.id),
    );
  if (!support) {
    emit("notice", `先选中一张${def.supportKinds.includes("desk") ? "书桌/床头柜" : "家具"}，或桌面已经摆满了`);
    return;
  }
  const item: RoomSurfaceItem = {
    id: `s-${Date.now()}-${seq++}`,
    def: def.id,
    supportId: support.id,
  };
  draft.value.surfaceItems.push(item);
  selectedId.value = support.id;
  emit("notice", `把${def.name}放到了${defs.value.get(support.def)?.name ?? "家具"}上`);
}

function removeSurfaceItem() {
  if (!surfaceForSelected.value) return;
  pushUndo();
  draft.value.surfaceItems = draft.value.surfaceItems.filter(
    (item) => item.id !== surfaceForSelected.value?.id,
  );
}

function surfaceSpritePath(sprite: string, rotations: number, rot: number): string {
  if (rotations > 1) {
    return asset(`/sprites/${sprite}_${String(((rot % rotations) + rotations) % rotations).padStart(2, "0")}.png`);
  }
  return asset(`/sprites/${sprite}.png`);
}

function drawSurfaceItems(ctx: CanvasRenderingContext2D) {
  for (const surface of draft.value.surfaceItems) {
    const support = draft.value.furniture.find((item) => item.id === surface.supportId);
    if (!support) continue;
    const supportBox = itemBox(support);
    const def = surfaceDefs.value.get(surface.def);
    if (!supportBox || !def) continue;
    const img = spriteCache.get(surfaceSpritePath(def.sprite, def.spriteRotations, support.rot));
    if (!img || !img.complete || img.naturalWidth === 0) continue;
    const base = iso(supportBox.x + supportBox.w / 2, supportBox.y + supportBox.d);
    const topOffset = supportBox.def.kind === "desk" ? 39 : 31;
    const left = Math.round(base.x - img.naturalWidth / 2);
    const top = Math.round(base.y - topOffset - img.naturalHeight);
    ctx.drawImage(img, left, top);
  }
}

function rotateSelected() {
  const item = selected.value;
  if (!item) return;
  pushUndo();
  item.rot = (item.rot + 1) % 4;
}

function storeSelected() {
  if (!selectedId.value) return;
  pushUndo();
  const id = selectedId.value;
  draft.value.furniture = draft.value.furniture.filter((item) => item.id !== id);
  draft.value.surfaceItems = draft.value.surfaceItems.filter((item) => item.supportId !== id);
  selectedId.value = null;
}

function footprint(def: RoomFurnitureDefView, rot: number): [number, number] {
  return rot % 2 === 1 ? [def.depth, def.width] : [def.width, def.depth];
}

function itemBox(item: RoomFurniture): { x: number; y: number; w: number; d: number; def: RoomFurnitureDefView } | null {
  const def = defs.value.get(item.def);
  if (!def) return null;
  const [w, d] = footprint(def, item.rot);
  return { x: item.x / props.room.grid, y: item.y / props.room.grid, w, d, def };
}

function isInvalid(item: RoomFurniture): boolean {
  const box = itemBox(item);
  if (!box) return true;
  if (box.x < 0 || box.y < 0) return true;
  if (box.x + box.w > props.room.width || box.y + box.d > props.room.height) return true;
  if (!box.def.collision) return false;
  return draft.value.furniture.some((other) => {
    if (other.id === item.id) return false;
    const b = itemBox(other);
    return !!b && b.def.collision &&
      box.x < b.x + b.w && b.x < box.x + box.w &&
      box.y < b.y + b.d && b.y < box.y + box.d;
  });
}

function poly(
  ctx: CanvasRenderingContext2D,
  points: P[],
  fill: string | CanvasGradient,
  stroke?: string,
  width = 1,
) {
  ctx.beginPath();
  points.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
  ctx.closePath();
  ctx.fillStyle = fill;
  ctx.fill();
  if (stroke) {
    ctx.strokeStyle = stroke;
    ctx.lineWidth = width;
    ctx.stroke();
  }
}

function drawFloor(ctx: CanvasRenderingContext2D) {
  const c = floorColor.value;
  for (let y = 0; y < props.room.height; y++) {
    for (let x = 0; x < props.room.width; x++) {
      const a = iso(x, y);
      const b = iso(x + 1, y);
      const d = iso(x + 1, y + 1);
      const e = iso(x, y + 1);
      poly(ctx, [a, b, d, e], (x + y) % 2 === 0 ? c.base : c.alt);
    }
  }

  // 地板纹理：木纹是长条 + 短木纹，砖/石是细缝。
  if (draft.value.floor === "wood") {
    ctx.strokeStyle = c.line;
    ctx.lineWidth = 1;
    for (let y = 1; y < props.room.height; y += 2) {
      const a = iso(0, y);
      const b = iso(props.room.width, y);
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }
    ctx.strokeStyle = c.grain;
    for (let y = 0; y < props.room.height; y += 2) {
      for (let x = 0; x < props.room.width; x += 3) {
        const p = iso(x + 0.4, y + 0.55);
        ctx.beginPath();
        ctx.moveTo(p.x - 10, p.y);
        ctx.lineTo(p.x + 12, p.y);
        ctx.stroke();
      }
    }
  } else {
    ctx.strokeStyle = c.line;
    ctx.lineWidth = 1;
    for (let x = 0; x <= props.room.width; x++) {
      const a = iso(x, 0);
      const b = iso(x, props.room.height);
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }
    for (let y = 0; y <= props.room.height; y++) {
      const a = iso(0, y);
      const b = iso(props.room.width, y);
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }
    if (draft.value.floor === "stone") {
      ctx.strokeStyle = c.grain;
      for (let y = 0; y < props.room.height; y++) {
        for (let x = 0; x < props.room.width; x++) {
          if ((x * 7 + y * 3) % 5 !== 0) continue;
          const p = iso(x + 0.5, y + 0.5);
          ctx.beginPath();
          ctx.moveTo(p.x - 4, p.y);
          ctx.lineTo(p.x + 6, p.y);
          ctx.stroke();
        }
      }
    }
  }

  // 房间外沿压一圈深色，让地板不飘。
  const o0 = iso(0, 0);
  const o1 = iso(props.room.width, 0);
  const o2 = iso(props.room.width, props.room.height);
  const o3 = iso(0, props.room.height);
  poly(ctx, [o0, o1, o2, o3], "rgba(0,0,0,0)", "rgba(83,61,40,.55)", 2);
}

function wallGradient(ctx: CanvasRenderingContext2D, top: number, bottom: number, colors: { top: string; bottom: string }) {
  const gradient = ctx.createLinearGradient(0, top, 0, bottom);
  gradient.addColorStop(0, colors.top);
  gradient.addColorStop(1, colors.bottom);
  return gradient;
}

function wallPoint(wall: "left" | "right", u: number, v: number): P {
  const base = wall === "right" ? iso(u, 0) : iso(0, u);
  return { x: base.x, y: base.y - (v / WALL_HEIGHT_TILES) * WALL_H };
}

function windowPoints(window: RoomWindow): P[] {
  const grid = props.room.grid;
  const u0 = window.x / grid;
  const u1 = (window.x + window.w) / grid;
  const v0 = window.y / grid;
  const v1 = (window.y + window.h) / grid;
  return [
    wallPoint(window.wall, u0, v0),
    wallPoint(window.wall, u1, v0),
    wallPoint(window.wall, u1, v1),
    wallPoint(window.wall, u0, v1),
  ];
}

function drawDoor(ctx: CanvasRenderingContext2D) {
  const door = draft.value.door;
  if (!door) return;
  const points = doorPoints(door);
  const selected = selectedDoorId.value === door.id;
  const invalid = doorInvalid(door);
  const outline = selected ? "#efb14a" : invalid ? "#d95d4f" : undefined;
  poly(ctx, points, "#5e432e", outline ?? "rgba(46,30,18,.75)", selected || invalid ? 3 : 2);

  const center = {
    x: points.reduce((sum, p) => sum + p.x, 0) / 4,
    y: points.reduce((sum, p) => sum + p.y, 0) / 4,
  };
  const inner = points.map((p) => ({
    x: (p.x + center.x) / 2,
    y: (p.y + center.y) / 2,
  }));
  poly(ctx, inner, "#ad7d4f", "rgba(58,38,22,.75)", 2);
  // 门板竖纹
  ctx.strokeStyle = "rgba(85,55,32,.45)";
  ctx.lineWidth = 1;
  for (let i = 1; i < 4; i++) {
    const a = {
      x: inner[0].x + ((inner[1].x - inner[0].x) * i) / 4,
      y: inner[0].y + ((inner[1].y - inner[0].y) * i) / 4,
    };
    const b = {
      x: inner[3].x + ((inner[2].x - inner[3].x) * i) / 4,
      y: inner[3].y + ((inner[2].y - inner[3].y) * i) / 4,
    };
    ctx.beginPath();
    ctx.moveTo(a.x, a.y);
    ctx.lineTo(b.x, b.y);
    ctx.stroke();
  }
  // 门把手
  const handle = {
    x: inner[1].x * 0.65 + inner[2].x * 0.35,
    y: inner[1].y * 0.65 + inner[2].y * 0.35,
  };
  ctx.fillStyle = "#e8c477";
  ctx.strokeStyle = "rgba(70,45,25,.55)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.arc(handle.x - 5, handle.y + 8, 3, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();
}

function drawWindows(ctx: CanvasRenderingContext2D) {
  for (const window of draft.value.windows) {
    const points = windowPoints(window);
    const selected = selectedWindowId.value === window.id;
    const invalid = windowInvalid(window);
    const outline = selected ? "#efb14a" : invalid ? "#d95d4f" : undefined;

    // 窗外天空：直接盖住墙纸，模拟墙体开洞。
    const top = Math.min(...points.map((p) => p.y));
    const bottom = Math.max(...points.map((p) => p.y));
    const sky = ctx.createLinearGradient(0, top, 0, bottom);
    sky.addColorStop(0, "#cfe2ec");
    sky.addColorStop(0.65, "#e7efe8");
    sky.addColorStop(1, "#c9d9bd");
    poly(ctx, points, sky, outline ?? "rgba(48,34,22,.65)", selected || invalid ? 3 : 1.6);

    const backdrop = windowBackdrop.value;
    if (backdrop && backdrop.complete && backdrop.naturalWidth > 0) {
      ctx.save();
      ctx.beginPath();
      points.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
      ctx.closePath();
      ctx.clip();
      const w = backdrop.naturalWidth;
      const h = backdrop.naturalHeight;
      const topX = (points[2].x - points[3].x) / w;
      const topY = (points[2].y - points[3].y) / w;
      const leftX = (points[0].x - points[3].x) / h;
      const leftY = (points[0].y - points[3].y) / h;
      ctx.transform(topX, topY, leftX, leftY, points[3].x, points[3].y);
      ctx.globalAlpha = 0.95;
      ctx.drawImage(backdrop, 0, 0);
      ctx.restore();
    }

    // 窗框
    const frame = 3;
    ctx.save();
    ctx.beginPath();
    points.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
    ctx.closePath();
    ctx.clip();
    ctx.strokeStyle = "#a57a50";
    ctx.lineWidth = frame * 2;
    ctx.stroke();

    // 玻璃反光 + 中梃
    const inner = points.map((p) => ({
      x: (p.x + (points[0].x + points[2].x) / 2) / 2,
      y: (p.y + (points[0].y + points[2].y) / 2) / 2,
    }));
    ctx.globalAlpha = 0.22;
    poly(ctx, inner, "#ffffff");
    ctx.globalAlpha = 1;
    ctx.strokeStyle = "rgba(165,122,80,.85)";
    ctx.lineWidth = 3;
    const c = {
      x: points.reduce((sum, p) => sum + p.x, 0) / 4,
      y: points.reduce((sum, p) => sum + p.y, 0) / 4,
    };
    ctx.beginPath();
    ctx.moveTo(c.x, top);
    ctx.lineTo(c.x, bottom);
    ctx.stroke();
    ctx.restore();
  }
}

function drawWalls(ctx: CanvasRenderingContext2D) {
  const colors = wallColor.value;
  const top = (p: P): P => ({ x: p.x, y: p.y - WALL_H });
  const left0 = iso(0, 0);
  const left1 = iso(0, props.room.height);
  const right1 = iso(props.room.width, 0);

  const leftTop = top(left0);
  const leftBottomTop = top(left1);
  const rightTop = top(right1);

  // 左墙
  ctx.beginPath();
  ctx.moveTo(left0.x, left0.y);
  ctx.lineTo(left1.x, left1.y);
  ctx.lineTo(leftBottomTop.x, leftBottomTop.y);
  ctx.lineTo(leftTop.x, leftTop.y);
  ctx.closePath();
  ctx.fillStyle = wallGradient(ctx, leftTop.y, left1.y, colors);
  ctx.fill();
  ctx.strokeStyle = "rgba(70,52,34,.42)";
  ctx.lineWidth = 1.5;
  ctx.stroke();

  // 右墙
  ctx.beginPath();
  ctx.moveTo(left0.x, left0.y);
  ctx.lineTo(right1.x, right1.y);
  ctx.lineTo(rightTop.x, rightTop.y);
  ctx.lineTo(leftTop.x, leftTop.y);
  ctx.closePath();
  ctx.fillStyle = wallGradient(ctx, rightTop.y, right1.y, colors);
  ctx.fill();
  ctx.fillStyle = "rgba(54,38,22,.06)";
  ctx.fill();
  ctx.stroke();

  // 墙纸竖纹
  ctx.strokeStyle = "rgba(255,255,255,.16)";
  ctx.lineWidth = 1;
  for (let i = 0.5; i < props.room.height; i += 0.65) {
    const p = iso(0, i);
    ctx.beginPath();
    ctx.moveTo(p.x, p.y - 4);
    ctx.lineTo(p.x, p.y - WALL_H + 4);
    ctx.stroke();
  }
  for (let i = 0.5; i < props.room.width; i += 0.65) {
    const p = iso(i, 0);
    ctx.beginPath();
    ctx.moveTo(p.x, p.y - 4);
    ctx.lineTo(p.x, p.y - WALL_H + 4);
    ctx.stroke();
  }

  // 踢脚线
  const base = 7;
  poly(
    ctx,
    [left0, left1, { x: left1.x, y: left1.y - base }, { x: left0.x, y: left0.y - base }],
    "#9a7650",
    "rgba(70,48,30,.5)",
    1,
  );
  poly(
    ctx,
    [left0, right1, { x: right1.x, y: right1.y - base }, { x: left0.x, y: left0.y - base }],
    "#a98258",
    "rgba(70,48,30,.5)",
    1,
  );
  ctx.strokeStyle = "rgba(255,238,205,.35)";
  ctx.beginPath();
  ctx.moveTo(left1.x, left1.y - base);
  ctx.lineTo(left0.x, left0.y - base);
  ctx.lineTo(right1.x, right1.y - base);
  ctx.stroke();

  // 墙顶压线
  ctx.strokeStyle = colors.trim;
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.moveTo(leftBottomTop.x, leftBottomTop.y);
  ctx.lineTo(leftTop.x, leftTop.y);
  ctx.lineTo(rightTop.x, rightTop.y);
  ctx.stroke();
  ctx.strokeStyle = "rgba(255,255,255,.35)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(leftBottomTop.x, leftBottomTop.y + 4);
  ctx.lineTo(leftTop.x, leftTop.y + 4);
  ctx.lineTo(rightTop.x, rightTop.y + 4);
  ctx.stroke();
}

function box(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  d: number,
  h: number,
  topColor: string,
  leftColor: string,
  rightColor: string,
  stroke = "rgba(52,36,24,.55)",
) {
  const p0 = iso(x, y);
  const p1 = iso(x + w, y);
  const p2 = iso(x + w, y + d);
  const p3 = iso(x, y + d);
  const t0 = { x: p0.x, y: p0.y - h };
  const t1 = { x: p1.x, y: p1.y - h };
  const t2 = { x: p2.x, y: p2.y - h };
  const t3 = { x: p3.x, y: p3.y - h };
  poly(ctx, [p0, p3, t3, t0], leftColor, stroke, 1.4);
  poly(ctx, [p1, p2, t2, t1], rightColor, stroke, 1.4);
  poly(ctx, [t0, t1, t2, t3], topColor, stroke, 1.4);
}

function softShadow(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  rx: number,
  ry: number,
  alpha = 0.2,
) {
  if (rx <= 0 || ry <= 0) return;
  ctx.save();
  ctx.fillStyle = `rgba(45,28,16,${alpha})`;
  ctx.beginPath();
  ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

function footprintShadow(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, d: number) {
  const quad = (expand: number, alpha: number) => {
    const points = [
      iso(x - expand, y - expand),
      iso(x + w + expand, y - expand),
      iso(x + w + expand, y + d + expand),
      iso(x - expand, y + d + expand),
    ];
    ctx.fillStyle = `rgba(45,30,18,${alpha})`;
    ctx.beginPath();
    points.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
    ctx.closePath();
    ctx.fill();
  };
  quad(0.12, 0.06);
  quad(0.04, 0.08);
}

function contactShadow(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, d: number) {
  footprintShadow(ctx, x, y, w, d);
}

function spritePath(def: RoomFurnitureDefView, rot: number): string | null {
  if (!def.sprite) return null;
  if (def.spriteRotations > 1) {
    const index = ((rot % def.spriteRotations) + def.spriteRotations) % def.spriteRotations;
    return asset(`/sprites/${def.sprite}_${String(index).padStart(2, "0")}.png`);
  }
  return asset(`/sprites/${def.sprite}.png`);
}

function preloadSprites() {
  for (const def of props.room.catalog) {
    const rotations = Math.max(1, def.spriteRotations || 1);
    for (let rot = 0; rot < rotations; rot++) {
      const path = spritePath(def, rot);
      if (!path || spriteCache.has(path)) continue;
      const img = new Image();
      img.onload = () => draw();
      img.onerror = () => {
        spriteCache.delete(path);
      };
      img.src = path;
      spriteCache.set(path, img);
    }
  }
  for (const def of props.room.surfaceCatalog) {
    const rotations = Math.max(1, def.spriteRotations || 1);
    for (let rot = 0; rot < rotations; rot++) {
      const path = surfaceSpritePath(def.sprite, rotations, rot);
      if (spriteCache.has(path)) continue;
      const img = new Image();
      img.onload = () => draw();
      img.onerror = () => spriteCache.delete(path);
      img.src = path;
      spriteCache.set(path, img);
    }
  }
}

function petImage(pet: PetView): string {
  return pet.image || asset(`/pets/${pet.breed}.png`);
}

function petFootPoint(path: string, img: HTMLImageElement): { x: number; y: number; width: number } {
  const cached = petFootCache.get(path);
  if (cached) return cached;
  const w = img.naturalWidth;
  const h = img.naturalHeight;
  const fallback = { x: w / 2, y: h, width: w * 0.4 };
  if (w === 0 || h === 0) return fallback;
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return fallback;
  ctx.drawImage(img, 0, 0);
  let data: Uint8ClampedArray;
  try {
    data = ctx.getImageData(0, 0, w, h).data;
  } catch {
    return fallback;
  }
  let minX = w;
  let maxX = 0;
  let maxY = 0;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (data[(y * w + x) * 4 + 3] > 32) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y > maxY) maxY = y;
      }
    }
  }
  if (maxX < minX) return fallback;
  const band = Math.max(4, Math.round((maxY - 0) * 0.12));
  let footMin = w;
  let footMax = 0;
  for (let y = Math.max(0, maxY - band); y <= maxY; y++) {
    for (let x = 0; x < w; x++) {
      if (data[(y * w + x) * 4 + 3] > 32) {
        if (x < footMin) footMin = x;
        if (x > footMax) footMax = x;
      }
    }
  }
  const point =
    footMax >= footMin
      ? { x: (footMin + footMax) / 2, y: maxY, width: footMax - footMin }
      : fallback;
  petFootCache.set(path, point);
  return point;
}

function preloadPetImages() {
  for (const pet of props.pets) {
    if (pet.boarded) continue;
    const path = petImage(pet);
    if (petCache.has(path)) continue;
    const img = new Image();
    img.onload = () => draw();
    img.src = path;
    petCache.set(path, img);
  }
}

function preloadFishImages() {
  for (const item of draft.value.furniture) {
    for (const fish of item.fish ?? []) {
      const path = asset(`/fish/${fish}.png`);
      if (fishCache.has(path)) continue;
      const img = new Image();
      img.onload = () => draw();
      img.src = path;
      fishCache.set(path, img);
    }
  }
}

function drawAquariumFish(
  ctx: CanvasRenderingContext2D,
  item: RoomFurniture,
  boxInfo: { x: number; y: number; w: number; d: number; def: RoomFurnitureDefView },
) {
  const path = spritePath(boxInfo.def, item.rot);
  if (!path) return;
  const tank = spriteCache.get(path);
  if (!tank || !tank.complete || tank.naturalWidth === 0) return;
  const base = iso(boxInfo.x + boxInfo.w / 2, boxInfo.y + boxInfo.d);
  const width = tank.naturalWidth;
  const height = tank.naturalHeight;
  const left = base.x - width / 2;
  const top = base.y - height;
  const padX = width * 0.18;
  const padTop = height * 0.16;
  const waterW = width * 0.64;
  const waterH = height * 0.42;
  if (waterW <= 0 || waterH <= 0) return;

  ctx.save();
  ctx.beginPath();
  ctx.rect(left + padX, top + padTop, waterW, waterH);
  ctx.clip();
  const now = performance.now() / 1000;
  const fishSize = Math.max(10, Math.round(height * 0.14));
  (item.fish ?? []).forEach((fish, index) => {
    const img = fishCache.get(asset(`/fish/${fish}.png`));
    if (!img || !img.complete || img.naturalWidth === 0) return;
    const phase = now * 0.8 + index * 1.9;
    const x = left + padX + waterW * (0.5 + 0.38 * Math.sin(phase));
    const y = top + padTop + waterH * (0.5 + 0.32 * Math.cos(phase * 0.7));
    const dir = Math.cos(phase) >= 0 ? 1 : -1;
    ctx.save();
    ctx.translate(x, y);
    ctx.scale(dir, 1);
    ctx.drawImage(img, -fishSize / 2, -fishSize / 2, fishSize, fishSize);
    ctx.restore();
  });
  ctx.restore();
}

function petFree(x: number, y: number): boolean {
  if (x < 0.45 || y < 0.45 || x > props.room.width - 0.45 || y > props.room.height - 0.45) {
    return false;
  }
  for (const item of draft.value.furniture) {
    const b = itemBox(item);
    if (!b || !b.def.collision) continue;
    if (
      x > b.x - 0.18 &&
      x < b.x + b.w + 0.18 &&
      y > b.y - 0.18 &&
      y < b.y + b.d + 0.18
    ) {
      return false;
    }
  }
  return true;
}

function randomPetSpot(): P {
  for (let i = 0; i < 40; i++) {
    const x = 0.7 + Math.random() * (props.room.width - 1.4);
    const y = 0.7 + Math.random() * (props.room.height - 1.4);
    if (petFree(x, y)) return { x, y };
  }
  return { x: props.room.width / 2, y: props.room.height / 2 };
}

function spawnPet(pet: PetView): boolean {
  if (pet.boarded) return false;
  if (petActors.value.some((actor) => actor.petId === pet.id)) return false;
  if (petActors.value.length >= 3) return false;
  const spot = randomPetSpot();
  petActors.value.push({
    petId: pet.id,
    name: pet.name,
    image: petImage(pet),
    x: spot.x,
    y: spot.y,
    dir: Math.random() > 0.5 ? "left" : "right",
    state: "idle",
    timer: 1.5 + Math.random() * 2,
    phase: 0,
    target: null,
  });
  ensurePetLoop();
  draw();
  return true;
}

function callPets() {
  if (editing.value) return;
  const limit = 3 - petActors.value.length;
  let added = 0;
  for (const pet of props.pets) {
    if (added >= limit) break;
    if (spawnPet(pet)) added++;
  }
  if (added === 0) {
    emit("notice", petActors.value.length >= 3 ? "屋里已经有 3 只宠物了" : "还没有可进屋的宠物");
  } else {
    emit("notice", `${added} 只宠物进屋了`);
  }
}

function choosePetTarget(actor: PetActor): P {
  for (let i = 0; i < 20; i++) {
    const spot = randomPetSpot();
    if (Math.hypot(spot.x - actor.x, spot.y - actor.y) > 0.8) return spot;
  }
  return randomPetSpot();
}

function updatePets(dt: number) {
  if (!props.active || editing.value) return;
  const now = Date.now();
  if (activePetCount.value > 0 && petActors.value.length < 3 && now >= nextEntryAt) {
    const candidates = props.pets.filter(
      (pet) => !pet.boarded && !petActors.value.some((actor) => actor.petId === pet.id),
    );
    if (candidates.length > 0) {
      spawnPet(candidates[Math.floor(Math.random() * candidates.length)]);
    }
    nextEntryAt = now + 35_000 + Math.random() * 40_000;
  }

  for (const actor of petActors.value) {
    actor.phase += dt;
    actor.timer -= dt;

    if (actor.state === "walk") {
      if (!actor.target) {
        actor.state = "idle";
        actor.timer = 1;
        continue;
      }
      const dx = actor.target.x - actor.x;
      const dy = actor.target.y - actor.y;
      const dist = Math.hypot(dx, dy);
      if (dist < 0.04) {
        actor.state = "idle";
        actor.timer = 1.5 + Math.random() * 3;
        actor.target = null;
        continue;
      }
      if (Math.abs(dx) > 0.01) actor.dir = dx > 0 ? "right" : "left";
      const step = Math.min(dist, dt * 0.85);
      const nx = actor.x + (dx / dist) * step;
      const ny = actor.y + (dy / dist) * step;
      if (!petFree(nx, ny)) {
        actor.state = "idle";
        actor.timer = 1;
        actor.target = null;
      } else {
        actor.x = nx;
        actor.y = ny;
      }
      continue;
    }

    if (actor.timer > 0) continue;

    const roll = Math.random();
    if (roll < 0.55) {
      actor.state = "walk";
      actor.target = choosePetTarget(actor);
      actor.timer = 8;
    } else if (roll < 0.75) {
      actor.state = "sit";
      actor.timer = 4 + Math.random() * 5;
    } else if (roll < 0.9) {
      actor.state = "sleep";
      actor.timer = 6 + Math.random() * 7;
    } else {
      actor.state = "idle";
      actor.timer = 2 + Math.random() * 3;
    }
  }
}

function drawPetSteps(
  ctx: CanvasRenderingContext2D,
  actor: PetActor,
  footX: number,
  footY: number,
) {
  const dir = actor.dir === "right" ? 1 : -1;
  ctx.save();
  ctx.fillStyle = "#8a7a63";
  for (let i = 0; i < 2; i++) {
    const t = (actor.phase * 1.7 + i * 0.5) % 1;
    ctx.globalAlpha = (1 - t) * 0.16;
    ctx.beginPath();
    ctx.arc(footX - dir * (4 + t * 9), footY - 1 - t * 2, 1.2 + t * 1.1, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}

function drawPetSnooze(ctx: CanvasRenderingContext2D, base: P, phase: number) {
  const lift = 1.5 + Math.sin(phase * 1.5) * 1.5;
  ctx.save();
  ctx.fillStyle = `rgba(96,112,86,${0.32 + 0.18 * (0.5 + 0.5 * Math.sin(phase * 1.5))})`;
  ctx.font = "7px 'Segoe UI', sans-serif";
  ctx.fillText("z", base.x + 9, base.y - PET_HEIGHT - 2 + lift);
  ctx.font = "5px 'Segoe UI', sans-serif";
  ctx.fillText("z", base.x + 14, base.y - PET_HEIGHT - 8 + lift);
  ctx.restore();
}

function drawPets(ctx: CanvasRenderingContext2D) {
  if (editing.value || props.readonly) return;
  const actors = [...petActors.value].sort((a, b) => a.y - b.y);
  for (const actor of actors) {
    const img = petCache.get(actor.image);
    if (!img || !img.complete || img.naturalWidth === 0) continue;
    const base = iso(actor.x, actor.y);
    const width = Math.max(1, Math.round((img.naturalWidth / img.naturalHeight) * PET_HEIGHT));

    const foot = petFootPoint(actor.image, img);
    const footScale = PET_HEIGHT / img.naturalHeight;
    const footX = base.x + (foot.x - img.naturalWidth / 2) * footScale;
    const footY = base.y - (img.naturalHeight - foot.y) * footScale;
    softShadow(ctx, footX, footY - 1, Math.max(10, foot.width * footScale * 0.8), 3.5, 0.12);

    const walking = actor.state === "walk";
    const breath = Math.sin(actor.phase * 2.2);
    let bob = 0;
    let squash = 1;
    if (walking) {
      bob = Math.sin(actor.phase * 9) * 1.5;
      drawPetSteps(ctx, actor, footX, footY);
    } else if (actor.state === "idle") {
      bob = breath > 0.55 ? -1 : 0;
    } else if (actor.state === "sit") {
      bob = breath > 0.7 ? -0.5 : 0;
      squash = 0.9;
    } else if (actor.state === "sleep") {
      squash = 0.82 + Math.sin(actor.phase * 1.4) * 0.012;
    }

    ctx.save();
    ctx.translate(base.x, Math.round(base.y + bob));
    if (actor.dir === "right") ctx.scale(-1, 1);
    ctx.scale(1, squash);
    if (actor.state === "sleep") ctx.globalAlpha = 0.9;
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(img, -width / 2, -PET_HEIGHT, width, PET_HEIGHT);
    ctx.restore();

    if (actor.state === "sleep") drawPetSnooze(ctx, base, actor.phase);
  }
}

function ensurePetLoop() {
  if (petTimer !== null) return;
  if (!props.active || (activePetCount.value === 0 && !hasAquariumFish.value)) return;
  lastPetTick = performance.now();
  petTimer = window.setInterval(() => {
    const now = performance.now();
    const dt = Math.min(0.25, (now - lastPetTick) / 1000);
    lastPetTick = now;
    if (props.active && !editing.value) {
      updatePets(dt);
      draw();
    }
  }, 120);
}

function stopPetLoop() {
  if (petTimer !== null) {
    window.clearInterval(petTimer);
    petTimer = null;
  }
}

function spriteSilhouette(path: string, img: HTMLImageElement, color: string): HTMLCanvasElement {
  const key = `${path}|${color}`;
  const cached = silhouetteCache.get(key);
  if (cached) return cached;
  const canvas = document.createElement("canvas");
  canvas.width = img.naturalWidth;
  canvas.height = img.naturalHeight;
  const ctx = canvas.getContext("2d")!;
  ctx.drawImage(img, 0, 0);
  ctx.globalCompositeOperation = "source-in";
  ctx.fillStyle = color;
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  silhouetteCache.set(key, canvas);
  return canvas;
}

function drawSpriteItem(
  ctx: CanvasRenderingContext2D,
  item: RoomFurniture,
  boxInfo: { x: number; y: number; w: number; d: number; def: RoomFurnitureDefView },
  outline?: string,
): boolean {
  const path = spritePath(boxInfo.def, item.rot);
  if (!path) return false;
  const img = spriteCache.get(path);
  if (!img || !img.complete || img.naturalWidth === 0) return false;

  const base = iso(boxInfo.x + boxInfo.w / 2, boxInfo.y + boxInfo.d);
  const left = Math.round(base.x - img.naturalWidth / 2);
  const top = Math.round(base.y - img.naturalHeight);
  if (outline) {
    const silhouette = spriteSilhouette(path, img, outline);
    for (const [dx, dy] of [
      [-2, 0],
      [2, 0],
      [0, -2],
      [0, 2],
      [-2, -2],
      [2, -2],
      [-2, 2],
      [2, 2],
    ]) {
      ctx.drawImage(silhouette, left + dx, top + dy);
    }
  } else {
    const silhouette = spriteSilhouette(path, img, "rgba(88,64,44,.18)");
    for (const [dx, dy] of [
      [-1, 0],
      [1, 0],
      [0, -1],
      [0, 1],
    ]) {
      ctx.drawImage(silhouette, left + dx, top + dy);
    }
  }
  ctx.drawImage(img, left, top);
  return true;
}

function drawItem(ctx: CanvasRenderingContext2D, item: RoomFurniture) {
  const boxInfo = itemBox(item);
  if (!boxInfo) return;
  const { x, y, w, d, def } = boxInfo;
  const outline = selectedId.value === item.id ? "#efb14a" : isInvalid(item) ? "#d95d4f" : undefined;
  const line = outline ?? "rgba(52,36,24,.55)";

  if (drawSpriteItem(ctx, item, { x, y, w, d, def }, outline)) {
    if (def.capacity > 0) drawAquariumFish(ctx, item, { x, y, w, d, def });
    return;
  }

  if (def.kind === "rug") {
    const p0 = iso(x + 0.08, y + 0.08);
    const p1 = iso(x + w - 0.08, y + 0.08);
    const p2 = iso(x + w - 0.08, y + d - 0.08);
    const p3 = iso(x + 0.08, y + d - 0.08);
    poly(ctx, [p0, p1, p2, p3], "#c8d3a8", outline ?? "rgba(80,74,48,.55)", outline ? 3 : 1.4);
    const q0 = iso(x + 0.35, y + 0.35);
    const q1 = iso(x + w - 0.35, y + 0.35);
    const q2 = iso(x + w - 0.35, y + d - 0.35);
    const q3 = iso(x + 0.35, y + d - 0.35);
    poly(ctx, [q0, q1, q2, q3], "#e2e7c8", "rgba(110,115,75,.45)", 1);
    const c = iso(x + w / 2, y + d / 2);
    ctx.fillStyle = "rgba(170,176,125,.45)";
    ctx.beginPath();
    ctx.ellipse(c.x, c.y, TW * 0.28, TH * 0.28, 0, 0, Math.PI * 2);
    ctx.fill();
    return;
  }

  if (def.kind === "bed") {
    // 床架
    box(ctx, x, y, w, d, 16, "#8f6747", "#6b4b32", "#7b573a", line);
    // 床垫
    box(ctx, x + 0.1, y + 0.18, w - 0.2, d - 0.3, 10, "#eee5d4", "#d7cbb7", "#e2d7c5", line);
    // 蓝色被子（盖下半截）
    box(ctx, x + 0.12, y + d * 0.38, w - 0.24, d * 0.52, 8, "#86a5c2", "#647f9c", "#7693ae", line);
    // 枕头
    box(ctx, x + 0.2, y + 0.2, Math.max(0.5, w - 0.4), 0.42, 5, "#f3ece0", "#d9cdbc", "#e8dece", line);
    // 床头板
    box(ctx, x, y - 0.02, w, 0.18, 34, "#a97b55", "#7f593b", "#936747", line);
    return;
  }

  if (def.kind === "desk") {
    // 桌腿
    box(ctx, x + 0.08, y + 0.1, 0.18, 0.18, 22, "#7a573c", "#5e422d", "#6b4b34", line);
    box(ctx, x + w - 0.26, y + 0.1, 0.18, 0.18, 22, "#7a573c", "#5e422d", "#6b4b34", line);
    // 桌面
    box(ctx, x, y, w, d, 7, "#d2a66d", "#a97845", "#bd8a53", line);
    // 桌面下的抽屉/横档
    box(ctx, x + 0.12, y + 0.22, w - 0.24, d - 0.3, 16, "#b07b4b", "#845a36", "#997044", line);
    // 书本
    box(ctx, x + 0.28, y + 0.16, 0.34, 0.22, 6, "#b95f55", "#8f4138", "#a54e45", line);
    box(ctx, x + 0.64, y + 0.14, 0.3, 0.2, 9, "#5f7f9e", "#45627d", "#52708c", line);
    return;
  }

  if (def.kind === "chair") {
    // 椅腿
    box(ctx, x + 0.14, y + 0.18, 0.12, 0.12, 10, "#6e4b32", "#523624", "#60412b", line);
    box(ctx, x + w - 0.26, y + 0.18, 0.12, 0.12, 10, "#6e4b32", "#523624", "#60412b", line);
    // 坐垫
    box(ctx, x + 0.12, y + 0.14, w - 0.24, d - 0.24, 9, "#82a99a", "#5e8577", "#70998a", line);
    // 靠背
    box(ctx, x + 0.2, y + 0.08, w - 0.4, 0.16, 30, "#78a190", "#547968", "#668d7b", line);
    return;
  }

  if (def.kind === "cabinet") {
    box(ctx, x, y, w, d, 34, "#dcc198", "#ae8d63", "#c8a679", line);
    // 三层抽屉
    ctx.strokeStyle = "rgba(82,57,35,.55)";
    ctx.lineWidth = 1;
    for (let i = 1; i < 4; i++) {
      const p = iso(x + 0.12, y + (d * i) / 4);
      const q = iso(x + w - 0.12, y + (d * i) / 4);
      ctx.beginPath();
      ctx.moveTo(p.x, p.y - 26);
      ctx.lineTo(q.x, q.y - 26);
      ctx.stroke();
    }
    for (let i = 0; i < 3; i++) {
      const p = iso(x + w / 2, y + d * (i + 0.5) / 3);
      ctx.fillStyle = "#7b5a3a";
      ctx.beginPath();
      ctx.arc(p.x, p.y - 12, 2.2, 0, Math.PI * 2);
      ctx.fill();
    }
    return;
  }

  if (def.kind === "plant") {
    box(ctx, x + 0.2, y + 0.46, w - 0.4, d - 0.46, 14, "#c07b53", "#935538", "#a96945", line);
    const c = iso(x + w / 2, y + d * 0.35);
    // 茎
    ctx.strokeStyle = "#4f7448";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(c.x, c.y - 8);
    ctx.lineTo(c.x, c.y - 36);
    ctx.stroke();
    // 叶片
    const leaves = [
      [-12, -30, 11, 15, -0.5, "#6d9a61"],
      [10, -28, 10, 14, 0.45, "#7aa96b"],
      [-4, -40, 9, 12, -0.15, "#86b676"],
      [14, -40, 7, 11, 0.7, "#5f8b55"],
    ] as const;
    for (const [dx, dy, rx, ry, rot, color] of leaves) {
      ctx.fillStyle = color;
      ctx.strokeStyle = "rgba(60,48,30,.35)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.ellipse(c.x + dx, c.y + dy, rx, ry, rot, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
    return;
  }

  box(ctx, x, y, w, d, 20, "#c7a57a", "#9b7b55", "#b18f68", line);
}

function draw() {
  const el = canvas.value;
  if (!el) return;
  const ctx = el.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(CANVAS_SCALE, 0, 0, CANVAS_SCALE, 0, 0);
  ctx.clearRect(0, 0, CW, CH);
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.fillStyle = "#f4eee1";
  ctx.fillRect(0, 0, CW, CH);
  ctx.save();
  ctx.translate(CW / 2 + camera.value.panX, CH / 2 + camera.value.panY);
  ctx.scale(camera.value.zoom, camera.value.zoom);
  ctx.translate(-sceneCenter.value.x, -sceneCenter.value.y);
  drawFloor(ctx);
  drawWalls(ctx);
  drawWindows(ctx);
  drawDoor(ctx);
  for (const item of sorted.value) drawItem(ctx, item);
  drawSurfaceItems(ctx);
  drawPets(ctx);
  ctx.restore();
  if (editing.value) {
    ctx.strokeStyle = "rgba(70,80,60,.65)";
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 5]);
    ctx.strokeRect(1, 1, CW - 2, CH - 2);
    ctx.setLineDash([]);
  }
}

function pointInPoly(px: number, py: number, points: P[]): boolean {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i++) {
    const xi = points[i].x;
    const yi = points[i].y;
    const xj = points[j].x;
    const yj = points[j].y;
    const intersect = yi > py !== yj > py && px < ((xj - xi) * (py - yi)) / (yj - yi) + xi;
    if (intersect) inside = !inside;
  }
  return inside;
}

function hitTest(px: number, py: number): RoomFurniture | null {
  for (const item of [...sorted.value].reverse()) {
    const b = itemBox(item);
    if (!b) continue;
    const points = [iso(b.x, b.y), iso(b.x + b.w, b.y), iso(b.x + b.w, b.y + b.d), iso(b.x, b.y + b.d)];
    const height = b.def.kind === "rug" ? 2 : 40;
    const topPoints = points.map((p) => ({ x: p.x, y: p.y - height }));
    if (pointInPoly(px, py, points) || pointInPoly(px, py, topPoints)) return item;
  }
  return null;
}

/// 触屏用的宽松命中：手指按不准，精确命中不到就在容差内找最近的一件。
/// 只在触屏路径用，鼠标照样要求点准。
function hitTestLoose(px: number, py: number, tolerance: number): RoomFurniture | null {
  const exact = hitTest(px, py);
  if (exact) return exact;
  let best: { item: RoomFurniture; dist: number } | null = null;
  for (const item of [...sorted.value].reverse()) {
    const b = itemBox(item);
    if (!b) continue;
    const poly = [
      iso(b.x, b.y),
      iso(b.x + b.w, b.y),
      iso(b.x + b.w, b.y + b.d),
      iso(b.x, b.y + b.d),
    ];
    const cx = (poly[0].x + poly[2].x) / 2;
    const cy = (poly[0].y + poly[2].y) / 2;
    const radius = Math.max(1, Math.hypot(poly[0].x - cx, poly[0].y - cy));
    const grow = 1 + tolerance / radius;
    const grown = poly.map((point) => ({ x: cx + (point.x - cx) * grow, y: cy + (point.y - cy) * grow }));
    if (!pointInPoly(px, py, grown)) continue;
    const dist = Math.hypot(px - cx, py - cy);
    if (!best || dist < best.dist) best = { item, dist };
  }
  return best?.item ?? null;
}

function canvasPoint(event: PointerEvent): P {
  return screenToWorld(rawPoint(event));
}

function toRoom(px: number, py: number): P {
  const dx = px - OX;
  const dy = py - OY;
  return {
    x: (dx / (TW / 2) + dy / (TH / 2)) / 2,
    y: (dy / (TH / 2) - dx / (TW / 2)) / 2,
  };
}

function hitDoor(px: number, py: number): RoomDoor | null {
  const door = draft.value.door;
  return door && pointInPoly(px, py, doorPoints(door)) ? door : null;
}

function doorPointer(wall: "left" | "right", px: number, py: number): P {
  return windowPointer(wall, px, py);
}

function hitWindow(px: number, py: number): RoomWindow | null {
  for (const window of [...draft.value.windows].reverse()) {
    if (pointInPoly(px, py, windowPoints(window))) return window;
  }
  return null;
}

function windowPointer(wall: "left" | "right", px: number, py: number): P {
  let u = 0;
  if (wall === "right") u = (px - OX) / (TW / 2);
  else u = (OX - px) / (TW / 2);
  const baseY = OY + u * (TH / 2);
  const v = ((baseY - py) / WALL_H) * WALL_HEIGHT_TILES;
  return { x: u * props.room.grid, y: v * props.room.grid };
}

// ---- 触屏手势（手机）----
// 鼠标那边是「左键拖家具、中/右键平移、滚轮缩放」，手机上三样都没有：
// 没有中键、没有滚轮、手指一按就把目标挡住。这里补一套触屏语义：
//   单指拖空白 = 平移视口
//   长按家具   = 拿起（抬到手指上方，别被手指挡住）
//   点一下     = 选中 / 进里屋
// 双指缩放先不做（页面上已有 +/- 按钮）
const TOUCH_SLOP_CSS = 10; // CSS 像素：超过这个距离算拖动，不算点击
const TOUCH_PICK_SLOP_CSS = 24; // 按在家具上时更宽容：超过这个距离才放弃拿起、改成平移
const TOUCH_HIT_TOLERANCE_CSS = 14; // 点选容差：家具小的时候手指按不准
const TOUCH_LONG_PRESS_MS = 220;
const TOUCH_LIFT_PX = 52; // 拿起后把家具抬到手指上方多少 CSS 像素

type TouchMode = "pending" | "pan" | "drag";
let touch: {
  mode: TouchMode;
  startX: number;
  startY: number;
  lastX: number;
  lastY: number;
  panStartX: number;
  panStartY: number;
  target: TouchTarget | null;
  moved: boolean;
  timer: number;
} | null = null;

function isTouchPointer(event: PointerEvent): boolean {
  return event.pointerType === "touch" || event.pointerType === "pen";
}

/// 把家具吸附到网格（触屏不追求像素级，落点对齐格子反而更稳）
function snapFurnitureTo(id: string, worldX: number, worldY: number) {
  const item = draft.value.furniture.find((entry) => entry.id === id);
  if (!item) return;
  const b = itemBox(item);
  if (!b) return;
  const g = props.room.grid;
  const maxX = props.room.width * g - b.w * g;
  const maxY = props.room.height * g - b.d * g;
  item.x = Math.max(0, Math.min(maxX, Math.round(worldX / g) * g));
  item.y = Math.max(0, Math.min(maxY, Math.round(worldY / g) * g));
}

/// 触屏的「点一下」：和鼠标按下时的选择语义保持一致
function handleTouchTap(x: number, y: number) {
  const world = screenToWorld({ x, y });
  const door = hitDoor(world.x, world.y);
  if (door) {
    if (props.extraUnlocked) emit("travel");
    else emit("notice", `先点「解锁里屋」花 ${props.extraCost} 金币`);
    return;
  }
  if (editing.value) {
    const win = hitWindow(world.x, world.y);
    if (win) {
      selectedWindowId.value = win.id;
      selectedId.value = null;
      selectedDoorId.value = null;
      return;
    }
  }
  const rect = canvas.value?.getBoundingClientRect();
  const tolerance = rect ? TOUCH_HIT_TOLERANCE_CSS * (CW / rect.width) : 0;
  const hit = hitTestLoose(world.x, world.y, tolerance);
  selectedId.value = hit?.id ?? null;
  if (!editing.value) {
    if (hit && hit.def.startsWith("aquarium")) emit("aquarium", hit.id);
    return;
  }
  if (!hit) {
    selectedWindowId.value = null;
    return;
  }
  selectedWindowId.value = null;
  selectedDoorId.value = null;
}

// 开发期调试钩子：窗在屏幕上的位置（自动测试用）
function debugWindowScreen(index = 0) {
  const win = draft.value.windows[index];
  if (!win || !canvas.value) return null;
  const points = windowPoints(win);
  const wx = points.reduce((sum, point) => sum + point.x, 0) / points.length;
  const wy = points.reduce((sum, point) => sum + point.y, 0) / points.length;
  const sx = (wx - sceneCenter.value.x) * camera.value.zoom + CW / 2 + camera.value.panX;
  const sy = (wy - sceneCenter.value.y) * camera.value.zoom + CH / 2 + camera.value.panY;
  const rect = canvas.value.getBoundingClientRect();
  return {
    x: (sx * rect.width) / CW + rect.left,
    y: (sy * rect.height) / CH + rect.top,
    item: { id: win.id, x: win.x, y: win.y },
  };
}

// 开发期调试钩子：算出某件家具在屏幕上的位置（自动测试用）
function debugFurnitureScreen(index = 0) {
  const item = draft.value.furniture[index];
  if (!item || !canvas.value) return null;
  const g = props.room.grid;
  const world = iso(item.x / g + 0.5, item.y / g + 0.5);
  const sx = (world.x - sceneCenter.value.x) * camera.value.zoom + CW / 2 + camera.value.panX;
  const sy = (world.y - sceneCenter.value.y) * camera.value.zoom + CH / 2 + camera.value.panY;
  const rect = canvas.value.getBoundingClientRect();
  return {
    x: (sx * rect.width) / CW + rect.left,
    y: (sy * rect.height) / CH + rect.top,
    item: { id: item.id, x: item.x, y: item.y },
  };
}

// 开发期调试钩子：桌面浏览器里用假触摸验手势，别的地方看不到相机
if (import.meta.env.DEV) {
  (window as { __roomCamera?: typeof camera; __roomDraft?: typeof draft }).__roomCamera = camera;
  (window as { __roomCamera?: typeof camera; __roomDraft?: typeof draft }).__roomDraft = draft;
  (window as { __roomFurnitureScreen?: typeof debugFurnitureScreen }).__roomFurnitureScreen =
    debugFurnitureScreen;
  (window as { __roomWindowScreen?: typeof debugWindowScreen }).__roomWindowScreen =
    debugWindowScreen;
  (window as { __roomTouchDebug?: () => unknown }).__roomTouchDebug = () => ({
    pointers: touchPointers.size,
    pinch: !!pinch,
    touchMode: touch?.mode ?? null,
    selected: selectedId.value,
    window: selectedWindowId.value,
    door: selectedDoorId.value,
    targetKind: touch?.target?.kind ?? null,
    editing: editing.value,
  });
}

// 双指缩放：两根手指的距离变化映射到 camera.zoom，
// 同时把「两指中点下面的那块地」钉在原地，缩放才不会跑偏。
const touchPointers = new Map<number, P>();
let pinch: {
  startDist: number;
  startZoom: number;
  worldX: number;
  worldY: number;
} | null = null;

/// 长按能抓起的东西：家具、墙上的窗、门。
/// 之前只认家具，按在窗/门上就落到「平移」分支，看起来就像整个屋子被拖走。
type TouchTarget =
  | { kind: "furniture"; id: string; ox: number; oy: number }
  | { kind: "window"; id: string; ox: number; oy: number }
  | { kind: "door"; id: string; ox: number };

function growPoly(points: P[], tolerance: number): P[] {
  const cx = points.reduce((sum, point) => sum + point.x, 0) / points.length;
  const cy = points.reduce((sum, point) => sum + point.y, 0) / points.length;
  const radius = Math.max(
    1,
    ...points.map((point) => Math.hypot(point.x - cx, point.y - cy)),
  );
  const grow = 1 + tolerance / radius;
  return points.map((point) => ({ x: cx + (point.x - cx) * grow, y: cy + (point.y - cy) * grow }));
}

function snapTo(unit: number, value: number): number {
  return Math.round(value / unit) * unit;
}

/// 触摸落点选中什么：门 > 窗 > 家具，命中都带 14 CSS 像素容差
function pickTouchTarget(world: P, tolerance: number): TouchTarget | null {
  const door = draft.value.door;
  if (door && pointInPoly(world.x, world.y, growPoly(doorPoints(door), tolerance))) {
    const local = doorPointer(door.wall, world.x, world.y);
    return { kind: "door", id: door.id, ox: door.x - local.x };
  }
  for (const win of [...draft.value.windows].reverse()) {
    if (pointInPoly(world.x, world.y, growPoly(windowPoints(win), tolerance))) {
      const local = windowPointer(win.wall, world.x, world.y);
      return { kind: "window", id: win.id, ox: win.x - local.x, oy: win.y - local.y };
    }
  }
  const item = hitTestLoose(world.x, world.y, tolerance);
  if (item) {
    const room = toRoom(world.x, world.y);
    const g = props.room.grid;
    return { kind: "furniture", id: item.id, ox: item.x - room.x * g, oy: item.y - room.y * g };
  }
  return null;
}

function dragTargetTo(target: TouchTarget, world: P) {
  const g = props.room.grid;
  if (target.kind === "furniture") {
    const room = toRoom(world.x, world.y);
    const item = draft.value.furniture.find((entry) => entry.id === target.id);
    const b = item ? itemBox(item) : null;
    if (!item || !b) return;
    const maxX = props.room.width * g - b.w * g;
    const maxY = props.room.height * g - b.d * g;
    item.x = Math.max(0, Math.min(maxX, snapTo(g, room.x * g + target.ox)));
    item.y = Math.max(0, Math.min(maxY, snapTo(g, room.y * g + target.oy)));
    return;
  }
  if (target.kind === "window") {
    const win = draft.value.windows.find((entry) => entry.id === target.id);
    if (!win) return;
    const local = windowPointer(win.wall, world.x, world.y);
    const maxX = wallLength(win.wall) - win.w;
    const maxY = WALL_HEIGHT_UNITS - win.h;
    win.x = Math.max(0, Math.min(maxX, snapTo(g, local.x + target.ox)));
    win.y = Math.max(0, Math.min(maxY, snapTo(g, local.y + target.oy)));
    return;
  }
  const door = draft.value.door;
  if (!door || door.id !== target.id) return;
  const local = doorPointer(door.wall, world.x, world.y);
  door.x = Math.max(0, Math.min(wallLength(door.wall) - door.w, snapTo(g, local.x + target.ox)));
}

function hud(text: string) {
  touchHud.value = text;
}

function pinchPair(): [P, P] | null {
  const values = [...touchPointers.values()];
  return values.length >= 2 ? [values[0], values[1]] : null;
}

function startPinch() {
  const pair = pinchPair();
  if (!pair) return;
  const [a, b] = pair;
  const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
  const world = screenToWorld(mid);
  pinch = {
    startDist: Math.max(1, Math.hypot(a.x - b.x, a.y - b.y)),
    startZoom: camera.value.zoom,
    worldX: world.x,
    worldY: world.y,
  };
}

function onTouchDown(event: PointerEvent) {
  if (!canvas.value) return;
  const raw = rawPoint(event);
  hud(`down ${event.pointerType}`);
  touchPointers.set(event.pointerId, raw);
  canvas.value.setPointerCapture?.(event.pointerId);

  // 第二根手指落下：进入缩放，放弃单指的待定/长按
  if (touchPointers.size === 2) {
    if (touch) {
      window.clearTimeout(touch.timer);
      touch = null;
    }
    startPinch();
    draw();
    return;
  }
  if (touchPointers.size > 2 || pinch) return;

  const rect = canvas.value.getBoundingClientRect();
  const scale = CW / rect.width;
  const world = screenToWorld(raw);
  const hit = props.readonly || !editing.value ? null : pickTouchTarget(world, TOUCH_HIT_TOLERANCE_CSS * scale);
  const hitKey = hit ? `${hit.kind}:${hit.id}` : "";
  const selectedKey = selectedId.value
    ? `furniture:${selectedId.value}`
    : selectedWindowId.value
      ? `window:${selectedWindowId.value}`
      : selectedDoorId.value
        ? `door:${selectedDoorId.value}`
        : "";
  // 只有「已经选中的那件」才能直接拖。
  // 否则手指按在 A 旁边、命中容差抓到 B，就会把没选中的 B 拖走——手机上很容易误触。
  const canDragSelected = !!hit && hitKey === selectedKey;
  hud(
    `down ed=${editing.value ? 1 : 0} hit=${hitKey || "null"} sel=${selectedKey || "null"} drag=${canDragSelected ? 1 : 0}`,
  );

  touch = {
    mode: canDragSelected ? "drag" : "pending",
    startX: raw.x,
    startY: raw.y,
    lastX: raw.x,
    lastY: raw.y,
    panStartX: camera.value.panX,
    panStartY: camera.value.panY,
    target: canDragSelected ? hit : null,
    moved: false,
    timer: 0,
  };

  // 按在别的家具上：只切换选中，不拖；想移动它，松手再按一次
  if (hit && !canDragSelected) {
    selectedId.value = hit.kind === "furniture" ? hit.id : null;
    selectedWindowId.value = hit.kind === "window" ? hit.id : null;
    selectedDoorId.value = hit.kind === "door" ? hit.id : null;
    draw();
  }
}

function onTouchMove(event: PointerEvent) {
  if (!canvas.value) return;
  const raw = rawPoint(event);
  if (touchPointers.has(event.pointerId)) touchPointers.set(event.pointerId, raw);

  // 缩放优先：两根手指时不做平移/拖动
  if (pinch) {
    const pair = pinchPair();
    if (!pair) return;
    const [a, b] = pair;
    const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    const dist = Math.max(1, Math.hypot(a.x - b.x, a.y - b.y));
    const zoom = Math.max(
      CAMERA_MIN,
      Math.min(CAMERA_MAX, (pinch.startZoom * dist) / pinch.startDist),
    );
    camera.value.zoom = zoom;
    camera.value.panX = mid.x - CW / 2 - (pinch.worldX - sceneCenter.value.x) * zoom;
    camera.value.panY = mid.y - CH / 2 - (pinch.worldY - sceneCenter.value.y) * zoom;
    draw();
    return;
  }

  if (!touch) return;
  const dx = raw.x - touch.startX;
  const dy = raw.y - touch.startY;
  touch.lastX = raw.x;
  touch.lastY = raw.y;
  hud(`mv m=${touch.mode} dx=${Math.round(dx)} dy=${Math.round(dy)}`);

  if (touch.mode === "pending") {
    const scale = CW / canvas.value.getBoundingClientRect().width;
    // 抓到了东西就更宽容：手指抖一抖不算「要平移」
    const limit = (touch.target ? TOUCH_PICK_SLOP_CSS : TOUCH_SLOP_CSS) * scale;
    if (Math.hypot(dx, dy) > limit) {
      window.clearTimeout(touch.timer);
      touch.mode = "pan";
    }
  }

  if (touch.mode === "pan") {
    camera.value.panX = touch.panStartX + dx;
    camera.value.panY = touch.panStartY + dy;
    draw();
    return;
  }

  if (touch.mode === "drag" && touch.target) {
    const rect = canvas.value.getBoundingClientRect();
    const scale = CW / rect.width;
    if (!touch.moved) {
      // 手指还没移出“点击”范围：别把家具拽走，用户可能只是想选中
      if (Math.hypot(dx, dy) < TOUCH_SLOP_CSS * scale) return;
      touch.moved = true;
      pushUndo();
    }
    const lift = TOUCH_LIFT_PX * (CH / rect.height);
    dragTargetTo(touch.target, screenToWorld({ x: raw.x, y: raw.y - lift }));
    draw();
  }
}

function onTouchUp(event?: PointerEvent) {
  if (event) touchPointers.delete(event.pointerId);

  if (pinch) {
    if (touchPointers.size >= 2) {
      startPinch();
      return;
    }
    pinch = null;
    const rest = [...touchPointers.values()][0];
    touch =
      rest && touchPointers.size === 1
        ? {
            mode: "pending",
            startX: rest.x,
            startY: rest.y,
            lastX: rest.x,
            lastY: rest.y,
            panStartX: camera.value.panX,
            panStartY: camera.value.panY,
            target: null,
            moved: false,
            timer: 0,
          }
        : null;
    draw();
    return;
  }

  if (!touch) return;
  window.clearTimeout(touch.timer);
  hud(`up m=${touch.mode} pts=${touchPointers.size}`);
  if (touch.mode === "pending" && touchPointers.size === 0) {
    handleTouchTap(touch.startX, touch.startY);
  }
  touch = null;
  draw();
}

function onPointerDown(event: PointerEvent) {
  try {
    handlePointerDown(event);
  } catch (error) {
    hud(`ERR down: ${(error as Error).message}`);
    throw error;
  }
}

function handlePointerDown(event: PointerEvent) {
  if (isTouchPointer(event)) {
    onTouchDown(event);
    return;
  }
  if (event.button === 1 || event.button === 2) {
    event.preventDefault();
    const p = rawPoint(event);
    panning.value = {
      x: p.x,
      y: p.y,
      panX: camera.value.panX,
      panY: camera.value.panY,
    };
    canvas.value?.setPointerCapture(event.pointerId);
    return;
  }
  if (props.readonly) return;
  const p = canvasPoint(event);

  const door = hitDoor(p.x, p.y);
  if (door) {
    if (!editing.value) {
      if (props.extraUnlocked) emit("travel");
      else emit("notice", `先点「解锁里屋」花 ${props.extraCost} 金币`);
      return;
    }
    event.preventDefault();
    pushUndo();
    selectedDoorId.value = door.id;
    selectedId.value = null;
    selectedWindowId.value = null;
    const local = doorPointer(door.wall, p.x, p.y);
    dragDoor.value = { id: door.id, ox: door.x - local.x };
    canvas.value?.setPointerCapture(event.pointerId);
    return;
  }

  if (editing.value) {
    const win = hitWindow(p.x, p.y);
    if (win) {
      event.preventDefault();
      pushUndo();
      selectedWindowId.value = win.id;
      selectedId.value = null;
      selectedDoorId.value = null;
      const local = windowPointer(win.wall, p.x, p.y);
      dragWindow.value = {
        id: win.id,
        ox: win.x - local.x,
        oy: win.y - local.y,
      };
      canvas.value?.setPointerCapture(event.pointerId);
      return;
    }
  }

  const hit = hitTest(p.x, p.y);
  selectedId.value = hit?.id ?? null;
  if (!editing.value) {
    if (hit && hit.def.startsWith("aquarium")) emit("aquarium", hit.id);
    return;
  }
  if (!hit) {
    selectedWindowId.value = null;
    return;
  }
  selectedWindowId.value = null;
  selectedDoorId.value = null;
  event.preventDefault();
  pushUndo();
  const room = toRoom(p.x, p.y);
  drag.value = {
    id: hit.id,
    ox: hit.x - room.x * props.room.grid,
    oy: hit.y - room.y * props.room.grid,
  };
  canvas.value?.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  try {
    handlePointerMove(event);
  } catch (error) {
    hud(`ERR move: ${(error as Error).message}`);
  }
}

function handlePointerMove(event: PointerEvent) {
  if (isTouchPointer(event)) {
    onTouchMove(event);
    return;
  }
  if (panning.value) {
    const p = rawPoint(event);
    camera.value.panX = panning.value.panX + (p.x - panning.value.x);
    camera.value.panY = panning.value.panY + (p.y - panning.value.y);
    draw();
    return;
  }
  if (dragDoor.value) {
    const door = draft.value.door;
    if (!door) return;
    const p = canvasPoint(event);
    const local = doorPointer(door.wall, p.x, p.y);
    door.x = Math.max(0, Math.min(wallLength(door.wall) - door.w, Math.round(local.x + dragDoor.value.ox)));
    return;
  }
  if (dragWindow.value) {
    const item = draft.value.windows.find((entry) => entry.id === dragWindow.value?.id);
    if (!item) return;
    const p = canvasPoint(event);
    const local = windowPointer(item.wall, p.x, p.y);
    const maxX = wallLength(item.wall) - item.w;
    const maxY = WALL_HEIGHT_UNITS - item.h;
    item.x = Math.max(0, Math.min(maxX, Math.round(local.x + (dragWindow.value?.ox ?? 0))));
    item.y = Math.max(0, Math.min(maxY, Math.round(local.y + (dragWindow.value?.oy ?? 0))));
    return;
  }
  if (!drag.value) return;
  const p = canvasPoint(event);
  const room = toRoom(p.x, p.y);
  const item = draft.value.furniture.find((entry) => entry.id === drag.value?.id);
  if (!item) return;
  const b = itemBox(item);
  if (!b) return;
  const g = props.room.grid;
  const maxX = props.room.width * g - b.w * g;
  const maxY = props.room.height * g - b.d * g;
  item.x = Math.max(0, Math.min(maxX, Math.round(room.x * g + (drag.value?.ox ?? 0))));
  item.y = Math.max(0, Math.min(maxY, Math.round(room.y * g + (drag.value?.oy ?? 0))));
}

function onPointerUp(event?: PointerEvent) {
  try {
    handlePointerUp(event);
  } catch (error) {
    hud(`ERR up: ${(error as Error).message}`);
  }
}

function handlePointerUp(event?: PointerEvent) {
  if (event && isTouchPointer(event)) {
    onTouchUp(event);
    return;
  }
  panning.value = null;
  drag.value = null;
  dragWindow.value = null;
  dragDoor.value = null;
}

watch(draft, draw, { deep: true });
watch([editing, selectedId, selectedWindowId, selectedDoorId], draw);
watch(
  () => props.room.layout,
  (layout) => {
    if (!editing.value) {
      draft.value = clone(layout);
      preloadFishImages();
      if (activePetCount.value === 0 && !hasAquariumFish.value) stopPetLoop();
      else ensurePetLoop();
      draw();
    }
  },
  { deep: true },
);
watch(
  () => props.pets.map((pet) => `${pet.id}:${pet.boarded}`).join(","),
  () => {
    const active = new Set(props.pets.filter((pet) => !pet.boarded).map((pet) => pet.id));
    petActors.value = petActors.value.filter((actor) => active.has(actor.petId));
    preloadPetImages();
    if (activePetCount.value === 0) stopPetLoop();
    else ensurePetLoop();
    draw();
  },
);
watch(
  () => props.active,
  (active) => {
    if (active) {
      nextEntryAt = Date.now() + 18_000 + Math.random() * 20_000;
      ensurePetLoop();
    } else {
      stopPetLoop();
    }
    draw();
  },
);
watch(
  () => props.room.roomId,
  () => {
    petActors.value = [];
    nextEntryAt = Date.now() + 15_000;
    loadWindowBackdrop();
    ensurePetLoop();
    draw();
  },
);
watch(editing, (value) => {
  if (!value) {
    nextEntryAt = Date.now() + 18_000;
    ensurePetLoop();
  }
  draw();
});
onMounted(() => {
  preloadSprites();
  preloadPetImages();
  preloadFishImages();
  loadWindowBackdrop();
  nextEntryAt = Date.now() + 18_000 + Math.random() * 20_000;
  ensurePetLoop();
  draw();
});
onBeforeUnmount(stopPetLoop);
</script>

<template>
  <div class="room-view">
    <div class="room-toolbar">
      <strong>{{ room.label }}</strong>
      <span class="muted">{{ room.width }}×{{ room.height }} · 1/8 格吸附</span>
      <span class="camera-controls">
        <button class="link" type="button" title="缩小" @click="zoomBy(-0.1)">[−]</button>
        <span class="muted">{{ zoomPct }}%</span>
        <button class="link" type="button" title="放大" @click="zoomBy(0.1)">[+]</button>
        <button class="link" type="button" title="恢复居中" @click="resetCamera">[居中]</button>
      </span>
      <template v-if="!editing">
        <button v-if="!readonly" class="link" type="button" @click="startEdit">[装修]</button>
        <button
          v-if="!readonly && activePetCount > 0"
          class="link"
          type="button"
          @click="callPets"
        >
          [呼唤宠物 {{ petActors.length }}/3]
        </button>
        <button
          v-if="!readonly && room.roomId === 'living' && !extraUnlocked"
          class="link"
          type="button"
          @click="$emit('unlock')"
        >
          [解锁里屋 {{ extraCost }}]
        </button>
        <button
          v-else-if="!readonly && room.roomId === 'living'"
          class="link"
          type="button"
          @click="$emit('travel')"
        >
          [进里屋]
        </button>
        <button v-else-if="!readonly" class="link" type="button" @click="$emit('travel')">[回客厅]</button>
        <span v-if="selected" class="muted">{{ defs.get(selected.def)?.name ?? selected.def }}</span>
      </template>
      <template v-else>
        <button class="link" type="button" @click="saveEdit">[完成]</button>
        <button class="link" type="button" @click="cancelEdit">[取消]</button>
        <button class="link" type="button" :disabled="undoStack.length === 0" @click="undo">
          [撤销]
        </button>
      </template>
    </div>

    <div class="room-canvas-wrap" @contextmenu.prevent @selectstart.prevent>
      <div v-if="editing" class="room-hud">{{ touchHud || "hud: 等你按一下" }}</div>
      <canvas
        ref="canvas"
        :width="CW * CANVAS_SCALE"
        :height="CH * CANVAS_SCALE"
        class="room-canvas"
        style="touch-action: none"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerUp"
        @wheel.prevent="onWheel"
        @contextmenu.prevent
      />
    </div>

    <p class="muted room-tip camera-hint">
      滚轮缩放；右键或中键拖动平移；[居中] 恢复默认视角。
    </p>

    <p v-if="!editing && !readonly && activePetCount > 0" class="muted room-tip">
      宠物会偶尔自己进屋，最多 3 只；也可以点「呼唤宠物」。装修模式下宠物暂停。
    </p>

    <div v-if="editing" class="room-editor">
      <div class="room-row">
        <label>地板
          <select :value="draft.floor" @change="changeFloor">
            <option
              v-for="style in room.floors"
              :key="style.id"
              :value="style.id"
              :disabled="style.locked"
            >
              {{ style.name }}{{ style.locked ? " 🔒" : "" }}
            </option>
          </select>
        </label>
        <label>墙纸
          <select :value="draft.wall" @change="changeWall">
            <option
              v-for="style in room.walls"
              :key="style.id"
              :value="style.id"
              :disabled="style.locked"
            >
              {{ style.name }}{{ style.locked ? " 🔒" : "" }}
            </option>
          </select>
        </label>
      </div>
      <p v-if="lockedStyles.length" class="muted room-tip">
        未解锁：{{ lockedStyles.map((style) => `${style.name}（${style.unlockHint}）`).join("；") }}
      </p>
      <div class="room-row room-palette">
        <button
          v-for="def in room.catalog"
          :key="def.id"
          class="link"
          type="button"
          :disabled="availableFor(def) <= 0"
          @click="addFurniture(def)"
        >
          +{{ def.name }}（剩 {{ availableFor(def) }}）
        </button>
        <button
          v-for="def in room.surfaceCatalog"
          :key="def.id"
          class="link"
          type="button"
          @click="addSurfaceItem"
        >
          +{{ def.name }}
        </button>
      </div>
      <div class="room-row">
        <span class="muted">窗户 {{ draft.windows.length }}/5</span>
        <button class="link" type="button" @click="addWindow('small')">+小窗</button>
        <button class="link" type="button" @click="addWindow('large')">+大窗</button>
        <button class="link" type="button" @click="addWindow('floor')">+落地窗</button>
      </div>
      <div v-if="selectedDoor" class="room-row">
        <span class="muted">
          门：{{ selectedDoor.wall === "left" ? "左墙" : "右墙" }} · 可移动、不可收纳
        </span>
        <button class="link" type="button" @click="moveDoorWall('left')">[左墙]</button>
        <button class="link" type="button" @click="moveDoorWall('right')">[右墙]</button>
        <span v-if="doorInvalid(selectedDoor)" class="warn">门越界或和窗户重叠</span>
      </div>
      <div v-else-if="selectedWindow" class="room-row">
        <span class="muted">
          窗户：{{ selectedWindow.wall === "left" ? "左墙" : "右墙" }} ·
          {{ selectedWindow.w }}×{{ selectedWindow.h }}
        </span>
        <button class="link" type="button" @click="moveWindowWall('left')">[左墙]</button>
        <button class="link" type="button" @click="moveWindowWall('right')">[右墙]</button>
        <button class="link" type="button" @click="resizeWindow(8, 0)">[宽+]</button>
        <button class="link" type="button" @click="resizeWindow(-8, 0)">[宽-]</button>
        <button class="link" type="button" @click="resizeWindow(0, 8)">[高+]</button>
        <button class="link" type="button" @click="resizeWindow(0, -8)">[高-]</button>
        <button class="link" type="button" @click="storeWindow">[删除]</button>
        <span v-if="windowInvalid(selectedWindow)" class="warn">窗户越界或重叠</span>
      </div>
      <div v-else-if="selected" class="room-row">
        <span class="muted">选中：{{ defs.get(selected.def)?.name ?? selected.def }}</span>
        <button class="link" type="button" @click="rotateSelected">[旋转]</button>
        <button class="link" type="button" @click="storeSelected">[收纳]</button>
        <button
          v-if="surfaceForSelected"
          class="link"
          type="button"
          @click="removeSurfaceItem"
        >
          [移除{{ surfaceDefs.get(surfaceForSelected.def)?.name ?? "摆件" }}]
        </button>
        <span v-if="isInvalid(selected)" class="warn">位置不合法（红色）</span>
      </div>
      <p class="muted room-tip">
        拖动家具/窗户移动；松手吸附 1/8 格；红色表示越界或重叠，完成时会被拦下。窗户只能放左右两面墙。
      </p>
    </div>
  </div>
</template>

<style scoped>
.room-view {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.room-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  font-size: 13px;
}
.camera-controls {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.camera-hint {
  margin-top: 2px;
}
.room-hud {
  position: sticky;
  top: 0;
  z-index: 5;
  margin-bottom: 4px;
  padding: 2px 6px;
  font: 12px/1.4 ui-monospace, monospace;
  color: #fff;
  background: rgb(0 0 0 / 65%);
  border-radius: 4px;
}

.room-canvas-wrap {
  width: 100%;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #f4efe3;
}
.room-canvas-wrap,
.room-canvas {
  /* 这个游戏正文是可选的（复制文本），但画布上长按会触发系统的“选中/长按”手势，
     安卓 WebView 随后发 pointercancel，拖动就断在半路——所以画布区域必须禁掉。 */
  user-select: none;
  -webkit-user-select: none;
  -webkit-touch-callout: none;
}

.room-canvas {
  display: block;
  width: 100%;
  height: auto;
  image-rendering: auto;
  touch-action: none;
  cursor: default;
}
.room-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px;
  border: 1px dashed var(--line);
  border-radius: 8px;
}
.room-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  font-size: 13px;
}
.room-row label {
  display: flex;
  align-items: center;
  gap: 6px;
}
.room-row select {
  padding: 2px 4px;
}
.room-palette {
  max-height: 80px;
  overflow: auto;
}
.room-tip {
  margin: 0;
  font-size: 12px;
}
</style>
