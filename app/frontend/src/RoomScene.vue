<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import type { RoomFurniture, RoomFurnitureDefView, RoomLayout, RoomView } from "./types";

const props = defineProps<{ room: RoomView; active: boolean }>();
const emit = defineEmits<{ save: [layout: RoomLayout]; notice: [text: string] }>();

const CW = 900;
const CH = 520;
const TW = 34;
const TH = 17;
const OX = 520;
const OY = 150;
const WALL_H = 104;

const canvas = ref<HTMLCanvasElement | null>(null);
const editing = ref(false);
const selectedId = ref<string | null>(null);
const draft = ref<RoomLayout>(clone(props.room.layout));
const drag = ref<{ id: string; ox: number; oy: number } | null>(null);
let seq = 0;

type P = { x: number; y: number };

function clone(layout: RoomLayout): RoomLayout {
  return JSON.parse(JSON.stringify(layout)) as RoomLayout;
}

function iso(x: number, y: number): P {
  return { x: OX + ((x - y) * TW) / 2, y: OY + ((x + y) * TH) / 2 };
}

const defs = computed(() => new Map(props.room.catalog.map((def) => [def.id, def])));
const floorColor = computed(() => {
  const id = draft.value.floor;
  if (id === "tile") return { base: "#d8d1c3", alt: "#cdc5b5", line: "rgba(70,60,45,.18)" };
  if (id === "stone") return { base: "#a8b3ad", alt: "#9aa69f", line: "rgba(40,55,50,.20)" };
  return { base: "#c9a46b", alt: "#bd965d", line: "rgba(75,50,25,.22)" };
});
const wallColor = computed(() => {
  const id = draft.value.wall;
  if (id === "sage") return "#b7c4ad";
  if (id === "sky") return "#b8c9d6";
  return "#e7dfd0";
});

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

function startEdit() {
  draft.value = clone(props.room.layout);
  editing.value = true;
  selectedId.value = null;
}

function cancelEdit() {
  draft.value = clone(props.room.layout);
  editing.value = false;
  selectedId.value = null;
  emit("notice", "已取消装修，布局没有变化");
}

function saveEdit() {
  editing.value = false;
  drag.value = null;
  selectedId.value = null;
  emit("save", clone(draft.value));
}

function addFurniture(def: RoomFurnitureDefView) {
  const g = props.room.grid;
  const w = def.width * g;
  const d = def.depth * g;
  const item: RoomFurniture = {
    id: `f-${Date.now()}-${seq++}`,
    def: def.id,
    x: Math.max(0, Math.round((props.room.width * g - w) / 2)),
    y: Math.max(0, Math.round((props.room.height * g - d) / 2)),
    rot: 0,
  };
  draft.value.furniture.push(item);
  selectedId.value = item.id;
}

function rotateSelected() {
  const item = selected.value;
  if (!item) return;
  item.rot = (item.rot + 1) % 4;
}

function storeSelected() {
  if (!selectedId.value) return;
  draft.value.furniture = draft.value.furniture.filter((item) => item.id !== selectedId.value);
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

function poly(ctx: CanvasRenderingContext2D, points: P[], fill: string, stroke?: string, width = 1) {
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
}

function drawWalls(ctx: CanvasRenderingContext2D) {
  const color = wallColor.value;
  const top = (p: P): P => ({ x: p.x, y: p.y - WALL_H });
  const left0 = iso(0, 0);
  const left1 = iso(0, props.room.height);
  poly(ctx, [left0, left1, top(left1), top(left0)], color, "rgba(55,45,35,.35)", 2);

  const right0 = iso(0, 0);
  const right1 = iso(props.room.width, 0);
  poly(ctx, [right0, right1, top(right1), top(right0)], color, "rgba(55,45,35,.35)", 2);

  // 墙脚/墙顶压线
  ctx.strokeStyle = "rgba(65,50,35,.35)";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(left0.x, left0.y);
  ctx.lineTo(left1.x, left1.y);
  ctx.lineTo(right1.x, right1.y);
  ctx.stroke();
  ctx.strokeStyle = "rgba(65,50,35,.22)";
  ctx.beginPath();
  ctx.moveTo(top(left0).x, top(left0).y);
  ctx.lineTo(top(left1).x, top(left1).y);
  ctx.lineTo(top(right1).x, top(right1).y);
  ctx.stroke();

  // 墙纸细纹
  ctx.strokeStyle = "rgba(255,255,255,.16)";
  ctx.lineWidth = 1;
  for (let i = 1; i < props.room.height; i++) {
    const p = iso(0, i);
    ctx.beginPath();
    ctx.moveTo(p.x, p.y - WALL_H + 2);
    ctx.lineTo(p.x, p.y - 2);
    ctx.stroke();
  }
  for (let i = 1; i < props.room.width; i++) {
    const p = iso(i, 0);
    ctx.beginPath();
    ctx.moveTo(p.x, p.y - WALL_H + 2);
    ctx.lineTo(p.x, p.y - 2);
    ctx.stroke();
  }
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
  stroke = "rgba(62,45,30,.35)",
) {
  const p0 = iso(x, y);
  const p1 = iso(x + w, y);
  const p2 = iso(x + w, y + d);
  const p3 = iso(x, y + d);
  const t0 = { x: p0.x, y: p0.y - h };
  const t1 = { x: p1.x, y: p1.y - h };
  const t2 = { x: p2.x, y: p2.y - h };
  const t3 = { x: p3.x, y: p3.y - h };
  poly(ctx, [p0, p3, t3, t0], leftColor, stroke);
  poly(ctx, [p1, p2, t2, t1], rightColor, stroke);
  poly(ctx, [t0, t1, t2, t3], topColor, stroke);
}

function drawItem(ctx: CanvasRenderingContext2D, item: RoomFurniture) {
  const boxInfo = itemBox(item);
  if (!boxInfo) return;
  const { x, y, w, d, def } = boxInfo;
  const outline = selectedId.value === item.id ? "#f0b34a" : isInvalid(item) ? "#d95d4f" : undefined;
  const line = outline ?? "rgba(62,45,30,.35)";

  if (def.kind === "rug") {
    const p0 = iso(x + 0.08, y + 0.08);
    const p1 = iso(x + w - 0.08, y + 0.08);
    const p2 = iso(x + w - 0.08, y + d - 0.08);
    const p3 = iso(x + 0.08, y + d - 0.08);
    poly(ctx, [p0, p1, p2, p3], "#cfd7b4", outline ?? "rgba(80,80,55,.35)", outline ? 3 : 1);
    const q0 = iso(x + 0.35, y + 0.35);
    const q1 = iso(x + w - 0.35, y + 0.35);
    const q2 = iso(x + w - 0.35, y + d - 0.35);
    const q3 = iso(x + 0.35, y + d - 0.35);
    poly(ctx, [q0, q1, q2, q3], "#e4e8ca", "rgba(120,125,85,.25)");
    return;
  }

  if (def.kind === "bed") {
    box(ctx, x, y, w, d, 14, "#8ea9c6", "#6b829e", "#7d95b0", line);
    const mx = x + 0.12;
    const my = y + 0.12;
    const mw = w - 0.24;
    const md = d - 0.24;
    box(ctx, mx, my, mw, md, 8, "#dfe8ee", "#c2d0d9", "#d1dce4", line);
    // 枕头
    box(ctx, x + 0.22, y + 0.22, Math.max(0.5, w - 0.44), 0.45, 5, "#f6f1e7", "#ddd5c7", "#e9e1d4", line);
    return;
  }

  if (def.kind === "desk") {
    box(ctx, x, y, w, d, 23, "#b98753", "#906538", "#a27343", line);
    box(ctx, x + 0.08, y + 0.08, w - 0.16, d - 0.16, 3, "#d2a66d", "#b07d47", "#c08d55", line);
    return;
  }

  if (def.kind === "chair") {
    box(ctx, x + 0.1, y + 0.12, w - 0.2, d - 0.2, 12, "#88b3a2", "#638f7e", "#76a391", line);
    box(ctx, x + 0.18, y + 0.1, w - 0.36, 0.18, 28, "#7aa795", "#5c8876", "#6c9886", line);
    return;
  }

  if (def.kind === "cabinet") {
    box(ctx, x, y, w, d, 30, "#d8bc8f", "#ad8c61", "#c5a477", line);
    ctx.strokeStyle = "rgba(90,65,40,.3)";
    for (let i = 1; i < 4; i++) {
      const p = iso(x + w * 0.15, y + (d * i) / 4);
      const q = iso(x + w * 0.85, y + (d * i) / 4);
      ctx.beginPath();
      ctx.moveTo(p.x, p.y - 26);
      ctx.lineTo(q.x, q.y - 26);
      ctx.stroke();
    }
    return;
  }

  if (def.kind === "plant") {
    box(ctx, x + 0.18, y + 0.42, w - 0.36, d - 0.42, 12, "#bb7650", "#8f5437", "#a56545", line);
    const c = iso(x + w / 2, y + d * 0.35);
    ctx.fillStyle = "#5f8e5b";
    ctx.beginPath();
    ctx.ellipse(c.x, c.y - 24, 18, 22, -0.25, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#78a56c";
    ctx.beginPath();
    ctx.ellipse(c.x - 8, c.y - 30, 12, 18, 0.35, 0, Math.PI * 2);
    ctx.fill();
    return;
  }

  box(ctx, x, y, w, d, 20, "#c7a57a", "#9b7b55", "#b18f68", line);
}

function draw() {
  const el = canvas.value;
  if (!el) return;
  const ctx = el.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, CW, CH);
  ctx.imageSmoothingEnabled = false;
  drawFloor(ctx);
  drawWalls(ctx);
  for (const item of sorted.value) drawItem(ctx, item);
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
    const height = b.def.kind === "rug" ? 2 : 30;
    const topPoints = points.map((p) => ({ x: p.x, y: p.y - height }));
    if (pointInPoly(px, py, points) || pointInPoly(px, py, topPoints)) return item;
  }
  return null;
}

function canvasPoint(event: PointerEvent): P {
  const el = canvas.value!;
  const rect = el.getBoundingClientRect();
  return {
    x: ((event.clientX - rect.left) * CW) / rect.width,
    y: ((event.clientY - rect.top) * CH) / rect.height,
  };
}

function toRoom(px: number, py: number): P {
  const dx = px - OX;
  const dy = py - OY;
  return {
    x: (dx / (TW / 2) + dy / (TH / 2)) / 2,
    y: (dy / (TH / 2) - dx / (TW / 2)) / 2,
  };
}

function onPointerDown(event: PointerEvent) {
  const p = canvasPoint(event);
  const hit = hitTest(p.x, p.y);
  selectedId.value = hit?.id ?? null;
  if (!editing.value || !hit) return;
  event.preventDefault();
  const room = toRoom(p.x, p.y);
  drag.value = {
    id: hit.id,
    ox: hit.x - room.x * props.room.grid,
    oy: hit.y - room.y * props.room.grid,
  };
  canvas.value?.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
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

function onPointerUp() {
  drag.value = null;
}

watch(draft, draw, { deep: true });
watch([editing, selectedId], draw);
watch(
  () => props.room.layout,
  (layout) => {
    if (!editing.value) {
      draft.value = clone(layout);
      draw();
    }
  },
  { deep: true },
);
onMounted(draw);
</script>

<template>
  <div class="room-view">
    <div class="room-toolbar">
      <strong>{{ room.label }}</strong>
      <span class="muted">{{ room.width }}×{{ room.height }} · 1/8 格吸附</span>
      <template v-if="!editing">
        <button class="link" type="button" @click="startEdit">[装修]</button>
        <span v-if="selected" class="muted">{{ defs.get(selected.def)?.name ?? selected.def }}</span>
      </template>
      <template v-else>
        <button class="link" type="button" @click="saveEdit">[完成]</button>
        <button class="link" type="button" @click="cancelEdit">[取消]</button>
      </template>
    </div>

    <div class="room-canvas-wrap">
      <canvas
        ref="canvas"
        :width="CW"
        :height="CH"
        class="room-canvas"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerUp"
      />
    </div>

    <div v-if="editing" class="room-editor">
      <div class="room-row">
        <label>地板
          <select v-model="draft.floor">
            <option v-for="style in room.floors" :key="style.id" :value="style.id">{{ style.name }}</option>
          </select>
        </label>
        <label>墙纸
          <select v-model="draft.wall">
            <option v-for="style in room.walls" :key="style.id" :value="style.id">{{ style.name }}</option>
          </select>
        </label>
      </div>
      <div class="room-row room-palette">
        <button
          v-for="def in room.catalog"
          :key="def.id"
          class="link"
          type="button"
          @click="addFurniture(def)"
        >
          +{{ def.name }}
        </button>
      </div>
      <div v-if="selected" class="room-row">
        <span class="muted">选中：{{ defs.get(selected.def)?.name ?? selected.def }}</span>
        <button class="link" type="button" @click="rotateSelected">[旋转]</button>
        <button class="link" type="button" @click="storeSelected">[收纳]</button>
        <span v-if="isInvalid(selected)" class="warn">位置不合法（红色）</span>
      </div>
      <p class="muted room-tip">拖动家具移动；松手吸附 1/8 格；红色表示越界或重叠，完成时会被拦下。</p>
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
.room-canvas-wrap {
  width: 100%;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #f4efe3;
}
.room-canvas {
  display: block;
  width: 100%;
  height: auto;
  image-rendering: pixelated;
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
