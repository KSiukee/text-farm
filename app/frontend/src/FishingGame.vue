<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { FishingSessionView } from "./types";

const props = defineProps<{ session: FishingSessionView }>();
const emit = defineEmits<{ resolve: [success: boolean] }>();

const phase = ref<"waiting" | "bite" | "playing">("waiting");
const frameX = ref(50);
const frameY = ref(50);
const frameVX = ref(0);
const frameVY = ref(0);
const fishX = ref(32);
const fishY = ref(38);
const fishTargetX = ref(32);
const fishTargetY = ref(38);
const fishVX = ref(0);
const fishVY = ref(0);
// 图标默认朝向：facesRight=true 的鱼先镜像成朝左，再按游动方向翻。
const fishFacing = ref(-1); // -1 朝左，1 朝右
const progress = ref(35);
const done = ref(false);
const message = ref("浮标落水了，等鱼咬钩……");
const inZone = ref(false);
const graceLeft = ref(0);
const trail = ref<{ x: number; y: number; facing: number }[]>([]);
const touchX = ref(0);
const touchY = ref(0);
const stickActive = ref(false);
const stickKnobX = ref(0);
const stickKnobY = ref(0);
const bubbles = Array.from({ length: 8 }, (_, index) => ({
  left: 8 + index * 11,
  delay: index * 0.55,
  duration: 4 + (index % 3),
  size: 3 + (index % 3) * 2,
}));
const keys = new Set<string>();
const pond = ref<HTMLElement | null>(null);
const pondAspect = ref(2.3);
let pondObserver: ResizeObserver | null = null;

let raf = 0;
let biteTimer = 0;
let last = 0;
let elapsed = 0;
let targetTimer = 0;
let stuckMs = 0;
let facingLockAt = 0;
let trailAt = 0;

const POND_MIN = 16;
const POND_MAX = 84;
const GRACE_SECONDS = 0.8;

/// 把目标点折回池塘内（边界像墙一样反弹），避免目标被夹在边上把鱼钉住。
function keepInPond(value: number): number {
  const span = POND_MAX - POND_MIN;
  const period = span * 2;
  let v = (((value - POND_MIN) % period) + period) % period;
  if (v > span) v = period - v;
  return POND_MIN + v;
}

const frameSize = computed(() => props.session.params.framePct);
const frameW = computed(() => frameSize.value);
// 窗口是宽屏，框本身保持正方形：高度百分比按池塘宽高比换算。
const frameH = computed(() => Math.min(100, frameSize.value * pondAspect.value));
const frameStyle = computed(() => ({
  left: `${frameX.value}%`,
  top: `${frameY.value}%`,
  width: `${frameW.value}%`,
  height: `${frameH.value}%`,
  transform: "translate(-50%, -50%)",
}));
const fishStyle = computed(() => {
  const base = props.session.facesRight ? -1 : 1;
  const scale = fishFacing.value < 0 ? base : -base;
  return {
    left: `${fishX.value}%`,
    top: `${fishY.value}%`,
    transform: `translate(-50%, -50%) scaleX(${scale})`,
  };
});
const progressStyle = computed(() => ({
  height: `${Math.max(0, Math.min(100, progress.value))}%`,
}));
const stickKnobStyle = computed(() => ({
  transform: `translate(${stickKnobX.value}px, ${stickKnobY.value}px)`,
}));
const progressHue = computed(
  () => 8 + Math.max(0, Math.min(100, progress.value)) * 1.05,
);
const progressColor = computed(
  () => `hsl(${progressHue.value.toFixed(1)}, 72%, 58%)`,
);
const progressGlow = computed(
  () => `hsla(${progressHue.value.toFixed(1)}, 72%, 58%, 0.32)`,
);
const progressVars = computed<Record<string, string>>(() => ({
  "--fg-progress-color": progressColor.value,
  "--fg-progress-glow": progressGlow.value,
}));
const zoneText = computed(() => {
  if (phase.value === "waiting") return "等咬钩";
  if (phase.value === "bite") return "咬钩，按 F";
  if (inZone.value) return "圈内，收线中";
  if (graceLeft.value > 0) return `圈外 · 缓冲 ${graceLeft.value.toFixed(1)}s`;
  return "圈外，收线在掉";
});

function trailStyle(point: { x: number; y: number; facing: number }, index: number) {
  const base = props.session.facesRight ? -1 : 1;
  const scale = point.facing < 0 ? base : -base;
  return {
    left: `${point.x}%`,
    top: `${point.y}%`,
    transform: `translate(-50%, -50%) scaleX(${scale})`,
    opacity: `${0.14 - index * 0.055}`,
  };
}

function finish(success: boolean, text: string) {
  if (done.value) return;
  done.value = true;
  phase.value = "playing";
  inZone.value = false;
  message.value = text;
  cancelAnimationFrame(raf);
  clearTimeout(biteTimer);
  emit("resolve", success);
}

function startGame() {
  if (done.value || phase.value !== "bite") return;
  phase.value = "playing";
  graceLeft.value = GRACE_SECONDS;
  message.value = "WASD / 方向键 / 下方轮盘移动框，把鱼圈住";
  last = 0;
  raf = requestAnimationFrame(tick);
}

function fail() {
  finish(false, "你收竿了，鱼跑了");
}

function focusGame() {
  pond.value?.focus();
}

function measurePond() {
  const el = pond.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  if (rect.width > 0 && rect.height > 0) {
    pondAspect.value = rect.width / rect.height;
  }
}

function watchPond() {
  pondObserver?.disconnect();
  pondObserver = null;
  if (typeof ResizeObserver !== "undefined" && pond.value) {
    pondObserver = new ResizeObserver(measurePond);
    pondObserver.observe(pond.value);
  }
  measurePond();
}

function updateStick(event: PointerEvent) {
  const el = event.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  let dx = (event.clientX - (rect.left + rect.width / 2)) / (rect.width / 2);
  let dy = (event.clientY - (rect.top + rect.height / 2)) / (rect.height / 2);
  const len = Math.hypot(dx, dy);
  if (len > 1) {
    dx /= len;
    dy /= len;
  }
  touchX.value = dx;
  touchY.value = dy;
  stickKnobX.value = dx * 30;
  stickKnobY.value = dy * 30;
}

function onStickDown(event: PointerEvent) {
  event.preventDefault();
  (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  stickActive.value = true;
  updateStick(event);
}

function onStickMove(event: PointerEvent) {
  if (stickActive.value) updateStick(event);
}

function onStickUp() {
  stickActive.value = false;
  touchX.value = 0;
  touchY.value = 0;
  stickKnobX.value = 0;
  stickKnobY.value = 0;
}

function axis(): { x: number; y: number } {
  let x = touchX.value;
  let y = touchY.value;
  for (const key of keys) {
    if (key === "a" || key === "arrowleft") x -= 1;
    if (key === "d" || key === "arrowright") x += 1;
    if (key === "w" || key === "arrowup") y -= 1;
    if (key === "s" || key === "arrowdown") y += 1;
  }
  const len = Math.hypot(x, y);
  if (len > 1) {
    x /= len;
    y /= len;
  }
  return { x, y };
}

function tick(now: number) {
  if (done.value || phase.value !== "playing") return;
  if (!last) last = now;
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  elapsed += dt * 1000;

  if (Date.now() > props.session.expiresAt) {
    finish(false, "这一竿放太久了");
    return;
  }
  if (elapsed >= props.session.params.catchMs) {
    finish(false, "时间到，鱼跑了");
    return;
  }

  const input = axis();
  const accel = 650 * (props.session.params.frameSpeedPct / 100);
  frameVX.value += input.x * accel * dt;
  frameVY.value += input.y * accel * dt;
  const drag = Math.pow(0.88, dt * 60);
  frameVX.value *= drag;
  frameVY.value *= drag;
  const maxSpeed = 90 * (props.session.params.frameSpeedPct / 100);
  frameVX.value = Math.max(-maxSpeed, Math.min(maxSpeed, frameVX.value));
  frameVY.value = Math.max(-maxSpeed, Math.min(maxSpeed, frameVY.value));
  frameX.value += frameVX.value * dt;
  frameY.value += frameVY.value * dt;

  const halfX = frameW.value / 2;
  const halfY = frameH.value / 2;
  if (frameX.value < halfX) {
    frameX.value = halfX;
    frameVX.value = 0;
  }
  if (frameX.value > 100 - halfX) {
    frameX.value = 100 - halfX;
    frameVX.value = 0;
  }
  if (frameY.value < halfY) {
    frameY.value = halfY;
    frameVY.value = 0;
  }
  if (frameY.value > 100 - halfY) {
    frameY.value = 100 - halfY;
    frameVY.value = 0;
  }

  // 鱼不再瞬移换向：目标慢慢转、速度平滑逼近，追击/逃逸更像一条线。
  targetTimer -= dt;
  const toFrameX = fishX.value - frameX.value;
  const toFrameY = (fishY.value - frameY.value) / pondAspect.value;
  const frameDist = Math.hypot(toFrameX, toFrameY);
  if (targetTimer <= 0) {
    const minMs = props.session.params.targetMinMs;
    const maxMs = props.session.params.targetMaxMs;
    targetTimer = (minMs + Math.random() * (maxMs - minMs)) / 1000;
    const jerk = props.session.params.fishJerkPct / 100;
    const range = 13 + jerk * 8;
    let angle: number;
    if (frameDist < props.session.params.fleeRadiusPct) {
      // 框逼近：朝远离框的方向逃，只留少量偏转。
      angle = Math.atan2(toFrameY, toFrameX) + (Math.random() - 0.5) * 0.35;
    } else {
      const heading = Math.hypot(fishVX.value, fishVY.value) > 0.1
        ? Math.atan2(fishVY.value, fishVX.value)
        : Math.random() * Math.PI * 2;
      angle = heading + (Math.random() - 0.5) * 0.7;
    }
    let targetX = fishX.value + Math.cos(angle) * range;
    let targetY = fishY.value + Math.sin(angle) * range;
    targetX = keepInPond(targetX);
    targetY = keepInPond(targetY);
    // 折返后如果离鱼太近，就朝池心推一段，保证它继续游。
    if (Math.hypot(targetX - fishX.value, targetY - fishY.value) < 6) {
      const cx = 50 - fishX.value;
      const cy = 50 - fishY.value;
      const len = Math.hypot(cx, cy);
      if (len > 1) {
        targetX = keepInPond(fishX.value + (cx / len) * 12);
        targetY = keepInPond(fishY.value + (cy / len) * 12);
      } else {
        targetX = keepInPond(fishX.value + (Math.random() - 0.5) * 16);
        targetY = keepInPond(fishY.value + (Math.random() - 0.5) * 16);
      }
    }
    fishTargetX.value = targetX;
    fishTargetY.value = targetY;
  }

  const dx = fishTargetX.value - fishX.value;
  const dy = fishTargetY.value - fishY.value;
  const dist = Math.hypot(dx, dy);
  const escaping = frameDist < 30;
  const speed =
    (props.session.params.fishSpeedPct / 100) *
    70 *
    (escaping ? props.session.params.fishEscapePct / 100 : 1);
  if (dist > 0.01) {
    const desiredX = (dx / dist) * speed;
    const desiredY = (dy / dist) * speed;
    const turn = Math.min(1, dt * 2.4 * (props.session.params.turnPct / 100));
    fishVX.value += (desiredX - fishVX.value) * turn;
    fishVY.value += (desiredY - fishVY.value) * turn;
  } else {
    fishVX.value *= Math.max(0, 1 - dt * 3);
    fishVY.value *= Math.max(0, 1 - dt * 3);
  }
  if (fishVX.value > 4) {
    if (fishFacing.value !== 1 && now - facingLockAt > 220) {
      fishFacing.value = 1;
      facingLockAt = now;
    }
  } else if (fishVX.value < -4) {
    if (fishFacing.value !== -1 && now - facingLockAt > 220) {
      fishFacing.value = -1;
      facingLockAt = now;
    }
  }

  // 贴边/目标过近时鱼会几乎停住；停超过 250ms 就强制换目标。
  const speedNow = Math.hypot(fishVX.value, fishVY.value);
  if (speedNow < 1.5) {
    stuckMs += dt * 1000;
    if (stuckMs > 250) {
      targetTimer = 0;
      stuckMs = 0;
    }
  } else {
    stuckMs = 0;
  }

  fishX.value += fishVX.value * dt;
  fishY.value += fishVY.value * dt;
  if (fishX.value < 16 || fishX.value > 84) {
    fishX.value = Math.max(16, Math.min(84, fishX.value));
    fishVX.value = 0;
  }
  if (fishY.value < 16 || fishY.value > 84) {
    fishY.value = Math.max(16, Math.min(84, fishY.value));
    fishVY.value = 0;
  }

  const inside =
    Math.abs(fishX.value - frameX.value) <= halfX &&
    Math.abs(fishY.value - frameY.value) <= halfY;
  inZone.value = inside;
  if (inside) {
    graceLeft.value = GRACE_SECONDS;
  } else if (graceLeft.value > 0) {
    graceLeft.value = Math.max(0, graceLeft.value - dt);
  }
  trailAt += dt * 1000;
  if (trailAt >= 110) {
    trailAt = 0;
    trail.value = [
      { x: fishX.value, y: fishY.value, facing: fishFacing.value },
      ...trail.value,
    ].slice(0, 2);
  }
  if (inside) {
    progress.value += props.session.params.progressGain * dt;
  } else if (graceLeft.value <= 0) {
    progress.value -= props.session.params.progressLoss * dt;
  }
  if (progress.value >= 100) {
    progress.value = 100;
    finish(true, "鱼收进鱼护了！");
    return;
  }
  if (progress.value <= 0) {
    finish(false, "鱼挣脱了");
    return;
  }
  raf = requestAnimationFrame(tick);
}

function onKey(event: KeyboardEvent) {
  const key = event.key.toLowerCase();
  if (key === "f") {
    event.preventDefault();
    if (event.type !== "keydown") return;
    if (phase.value === "bite") {
      startGame();
    } else if (phase.value === "waiting") {
      message.value = "还没咬钩，再等等";
    }
    return;
  }
  if (phase.value !== "playing") return;
  if (key === "w" || key === "a" || key === "s" || key === "d" || key.startsWith("arrow")) {
    event.preventDefault();
    keys.add(key);
  }
}

function onKeyUp(event: KeyboardEvent) {
  keys.delete(event.key.toLowerCase());
}

watch(phase, () => {
  void nextTick(watchPond);
});

onMounted(() => {
  window.addEventListener("keydown", onKey);
  window.addEventListener("keyup", onKeyUp);
  pond.value?.focus();
  const delay = 1600 + (Number(props.session.id) % 3) * 700;
  biteTimer = window.setTimeout(() => {
    if (done.value) return;
    phase.value = "bite";
    message.value = "咬钩了！按 F 开始收线";
  }, delay);
});
onUnmounted(() => {
  cancelAnimationFrame(raf);
  clearTimeout(biteTimer);
  pondObserver?.disconnect();
  pondObserver = null;
  window.removeEventListener("keydown", onKey);
  window.removeEventListener("keyup", onKeyUp);
  onStickUp();
});
</script>

<template>
  <div class="fishing-game" :data-phase="phase">
    <div class="fg-head">
      <strong>{{ session.fishName }}</strong>
      <span class="muted">{{ session.spotLabel }} · {{ session.params.personalityLabel }} · {{ session.isKing ? "鱼王" : `难度 ${session.difficulty}` }}</span>
    </div>

    <div
      v-if="phase !== 'playing'"
      ref="pond"
      class="fg-wait"
      :class="{ 'fg-bite': phase === 'bite' }"
      tabindex="0"
      @click="focusGame"
    >
      <div class="fg-sky"></div>
      <div class="fg-water"></div>
      <div class="fg-water-glint"></div>
      <div class="fg-reeds">
        <span
          v-for="i in 5"
          :key="i"
          :style="{ left: `${8 + i * 18}%`, height: `${18 + (i % 3) * 8}px` }"
        ></span>
      </div>
      <div class="fg-line"></div>
      <div class="fg-bobber-wrap">
        <div class="fg-bobber">
          <span class="fg-bobber-antenna"></span>
          <span class="fg-bobber-body"></span>
        </div>
        <div class="fg-ripple"></div>
        <div class="fg-ripple fg-ripple-2"></div>
        <div class="fg-ripple fg-ripple-3"></div>
      </div>
      <div v-if="phase === 'bite'" class="fg-bite-badge">咬钩</div>
      <div class="fg-wait-copy">
        <span class="fg-kicker">{{ phase === "bite" ? "浮标猛沉" : "浮标落水" }}</span>
        <strong>{{ phase === "bite" ? "按 F 开始收线" : "等几秒，别惊了鱼……" }}</strong>
      </div>
    </div>
    <div
      v-else
      ref="pond"
      class="fg-pond"
      :class="{ 'in-zone': inZone, 'is-king': session.isKing }"
      :style="progressVars"
      tabindex="0"
      @click="focusGame"
      @pointerdown="focusGame"
    >
      <div class="fg-pond-glow"></div>
      <div class="fg-caustics"></div>
      <div class="fg-bubbles">
        <span
          v-for="(bubble, index) in bubbles"
          :key="index"
          class="fg-bubble"
          :style="{
            left: `${bubble.left}%`,
            animationDelay: `${bubble.delay}s`,
            animationDuration: `${bubble.duration}s`,
            width: `${bubble.size}px`,
            height: `${bubble.size}px`,
          }"
        ></span>
      </div>
      <div class="fg-progress"><span :style="progressStyle"></span></div>
      <div class="fg-trail">
        <img
          v-for="(point, index) in trail"
          :key="index"
          :src="`/fish/${session.fishId}.png`"
          :style="trailStyle(point, index)"
          alt=""
        />
      </div>
      <div class="fg-zone-glow" :style="frameStyle"></div>
      <div class="fg-frame" :style="frameStyle">
        <i class="fg-corner tl"></i>
        <i class="fg-corner tr"></i>
        <i class="fg-corner bl"></i>
        <i class="fg-corner br"></i>
        <span class="fg-frame-pct">{{ Math.round(progress) }}%</span>
      </div>
      <div class="fg-fish-shadow" :style="fishStyle"></div>
      <img class="fg-fish" :src="`/fish/${session.fishId}.png`" :style="fishStyle" alt="" />
      <div class="fg-pond-vignette"></div>
    </div>

    <div class="fg-status">
      <span>{{ message }}</span>
      <span class="fg-zone-tag" :class="{ active: inZone }">{{ zoneText }}</span>
      <span v-if="phase === 'playing'" class="muted">{{ Math.round(progress) }}%</span>
    </div>
    <div class="row">
      <button v-if="phase === 'bite'" class="link" type="button" :disabled="done" @click="startGame">
        [开始收线]
      </button>
      <button class="link" type="button" :disabled="done" @click="fail">[收竿]</button>
      <span v-if="phase === 'waiting'" class="muted">F 抛竿后等几秒，咬钩再按 F</span>
      <span v-else-if="phase === 'bite'" class="muted">按 F 或点 [开始收线]</span>
      <span v-else class="muted">WASD / 方向键 / 下方轮盘移动框；进度满自动收鱼</span>
    </div>
    <div v-if="phase === 'playing'" class="fg-stick-wrap">
      <div
        class="fg-stick"
        :class="{ active: stickActive }"
        @pointerdown="onStickDown"
        @pointermove="onStickMove"
        @pointerup="onStickUp"
        @pointercancel="onStickUp"
      >
        <div class="fg-stick-knob" :style="stickKnobStyle"></div>
      </div>
      <span class="muted">拖轮盘移动鱼框</span>
    </div>
  </div>
</template>
