<script setup lang="ts">
import { setImmersive } from "./api";
import { asset } from "./assets";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { FishingSessionView } from "./types";

const props = defineProps<{ session: FishingSessionView | null; active: boolean }>();
const emit = defineEmits<{ resolve: [success: boolean]; notice: [text: string] }>();

type FeelMode = "current" | "bad";
// 调参开关：手感、打架时长、重力感应的入口先藏起来。
// 手感固定 2（追鱼）、时长固定 D2；重力感应手感还没调好，不可用，等调完再放出来。
const SHOW_TUNING = false;
const SHOW_TILT = false;

const FEEL_KEY = "farm.ui.fishingFeel";
type FightMode = "d2" | "d3";
const FIGHT_KEY = "farm.ui.fishingFight";

const phase = ref<"idle" | "waiting" | "bite" | "playing">(
  props.session ? "waiting" : "idle",
);
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
const peakProgress = ref(35);
const tension = ref(30);
const kingPhase = ref<"none" | "probe" | "power" | "sprint">("none");
const done = ref(false);
const message = ref(props.session ? "浮标落水了，等鱼咬钩……" : "按 F 抛竿，等浮标落水。");
const inZone = ref(false);
const graceLeft = ref(0);
const biteLeft = ref(0);
const feel = ref<FeelMode>("bad"); // 2 = 追鱼

// 重力感应：手机横竖无所谓，用 gamma（左右）/ beta（前后）映射成框的速度。
// 打开时记一次基准角，之后只吃相对变化，玩家不用把手机摆得跟水平仪一样。
const TILT_KEY = "farm.ui.fishingTilt";
const tiltOn = ref(false);
const tiltX = ref(0);
const tiltY = ref(0);
let tiltBase: { x: number; y: number } | null = null;
const TILT_DEAD_DEG = 2.5;
const TILT_RANGE_DEG = 22;

function readTilt(): boolean {
  try {
    return window.localStorage.getItem(TILT_KEY) === "1";
  } catch {
    return false;
  }
}

/// 按屏幕方向把 beta/gamma 换算成「屏幕上的左右/上下」。
/// 横屏时手机是侧着的，gamma 变成了俯仰，直接用会把框甩到角落。
function tiltAxes(beta: number, gamma: number): { x: number; y: number } {
  const angle = ((screen.orientation?.angle ?? 0) % 360 + 360) % 360;
  if (angle === 90) return { x: beta, y: -gamma };
  if (angle === 180) return { x: -gamma, y: -beta };
  if (angle === 270) return { x: -beta, y: gamma };
  return { x: gamma, y: beta };
}

function onTilt(event: DeviceOrientationEvent) {
  if (!tiltOn.value) return;
  const beta = event.beta;
  const gamma = event.gamma;
  // 有的设备头几次事件是 null / 异常值，拿它当基准就会把框按在角落
  if (beta === null || gamma === null) return;
  if (!Number.isFinite(beta) || !Number.isFinite(gamma)) return;
  if (Math.abs(beta) > 120 || Math.abs(gamma) > 120) return;

  const axes = tiltAxes(beta, gamma);
  if (!tiltBase) {
    tiltBase = { x: axes.x, y: axes.y };
    return;
  }
  const rawX = axes.x - tiltBase.x;
  const rawY = axes.y - tiltBase.y;
  // 基准之后手机被大幅挪动（换握姿/转屏），重新取基准，别硬顶
  if (Math.abs(rawX) > 45 || Math.abs(rawY) > 45) {
    tiltBase = { x: axes.x, y: axes.y };
    tiltX.value = 0;
    tiltY.value = 0;
    return;
  }
  const dx = rawX / TILT_RANGE_DEG;
  const dy = rawY / TILT_RANGE_DEG;
  const dead = TILT_DEAD_DEG / TILT_RANGE_DEG;
  tiltX.value = Math.abs(dx) < dead ? 0 : Math.max(-1, Math.min(1, dx));
  tiltY.value = Math.abs(dy) < dead ? 0 : Math.max(-1, Math.min(1, dy));
}

function setTilt(on: boolean) {
  tiltOn.value = on;
  tiltBase = null;
  tiltX.value = 0;
  tiltY.value = 0;
  try {
    window.localStorage.setItem(TILT_KEY, on ? "1" : "0");
  } catch {
    // 存不了就只影响这一次
  }
  if (on) {
    window.addEventListener("deviceorientation", onTilt);
  } else {
    window.removeEventListener("deviceorientation", onTilt);
  }
}
const fightMode = ref<FightMode>("d2"); // D2 = 中
// D2/D3 都只拉长打架：活泼/凶猛/鱼王；金色变异跟随鱼王。
const fightMultiplier = computed(() => {
  const session = props.session;
  if (!session) return 1;
  if (session.isKing || session.isGolden) {
    return fightMode.value === "d3" ? 2.2 : 1.8;
  }
  if (session.params.personalityLabel === "活泼") {
    return fightMode.value === "d3" ? 1.5 : 1.3;
  }
  if (session.params.personalityLabel === "凶猛") {
    return fightMode.value === "d3" ? 1.8 : 1.5;
  }
  return 1;
});
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
let biteWindowTimer = 0;
let biteCountdownTimer = 0;
let last = 0;
let elapsed = 0;
let targetTimer = 0;
let stuckMs = 0;
let facingLockAt = 0;
let trailAt = 0;
let badHoldMs = 0;
let badDirX = 0;
let badDirY = 0;
let badReversalMs = 0;
let burstLeft = 0;
let burstCooldown = 3;

const POND_MIN = 16;
const POND_MAX = 84;
const GRACE_SECONDS = 1.2;
const BITE_WINDOW_MS = 8_000;

/// 把目标点折回池塘内（边界像墙一样反弹），避免目标被夹在边上把鱼钉住。
function keepInPond(value: number): number {
  const span = POND_MAX - POND_MIN;
  const period = span * 2;
  let v = (((value - POND_MIN) % period) + period) % period;
  if (v > span) v = period - v;
  return POND_MIN + v;
}

/// 鱼种偏好的水深区间（百分比，0 = 水面）。
function preferredDepth(): [number, number] {
  switch (behavior.value) {
    case "bottom":
      return [58, 82];
    case "surface":
      return [18, 42];
    case "burst":
      return [28, 68];
    default:
      return [26, 72];
  }
}

function depthBias(): number {
  switch (behavior.value) {
    case "bottom":
    case "surface":
      return 0.55;
    case "burst":
      return 0.35;
    default:
      return 0.25;
  }
}

/// 触屏 UI（判定在 main.ts 里，统一一处）：手机/平板才算，
/// 带触摸屏的 Windows 笔记本不算，免得桌面上弹出摇杆和补偿。
const TOUCH_UI =
  typeof document !== "undefined" && document.documentElement.classList.contains("touch");
const TOUCH_FRAME_BONUS = 1.15;

const frameSize = computed(() => {
  const base = props.session?.params.framePct ?? 20;
  return TOUCH_UI ? base * TOUCH_FRAME_BONUS : base;
});
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
  const base = props.session?.facesRight ? -1 : 1;
  const scale = fishFacing.value < 0 ? base : -base;
  return {
    left: `${fishX.value}%`,
    top: `${fishY.value}%`,
    transform: `translate(-50%, -50%) scaleX(${scale})`,
  };
});
const progressStyle = computed(() => ({
  width: `${Math.max(0, Math.min(100, progress.value))}%`,
}));
const progressGhostStyle = computed(() => ({
  width: `${Math.max(0, Math.min(100, peakProgress.value))}%`,
}));
const tensionStyle = computed(() => ({
  width: `${Math.max(0, Math.min(100, tension.value))}%`,
}));
const tensionState = computed(() =>
  tension.value >= 75 ? "high" : tension.value <= 34 ? "low" : "good",
);
const behavior = computed(() => props.session?.params.behavior ?? "migratory");
const behaviorLabel = computed(() => props.session?.params.behaviorLabel ?? "洄游");
const behaviorHint = computed(() => props.session?.params.behaviorHint ?? "");
const bgSrc = computed(() => `/fishing/bg_${props.session?.spot ?? "creek"}.png`);
const kingPhaseLabel = computed(() => {
  switch (kingPhase.value) {
    case "probe":
      return "试探";
    case "power":
      return "发力";
    case "sprint":
      return "冲刺";
    default:
      return "";
  }
});
const isLosing = computed(
  () => phase.value === "playing" && !inZone.value && graceLeft.value <= 0,
);
// 磨损只降加速度：frame_accel_pct=62，基础速度仍是 100%。
const isWornRod = computed(() => (props.session?.params.frameAccelPct ?? 100) < 100);
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
  if (phase.value === "idle") return "等抛竿";
  if (phase.value === "waiting") return "等咬钩";
  if (phase.value === "bite") return "咬钩，按 F";
  if (inZone.value) return "圈内，收线中";
  if (graceLeft.value > 0) return `圈外 · 缓冲 ${graceLeft.value.toFixed(1)}s`;
  return "圈外，收线在掉";
});

function trailStyle(point: { x: number; y: number; facing: number }, index: number) {
  const base = props.session?.facesRight ? -1 : 1;
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
  clearBiteWindow();
  cancelAnimationFrame(raf);
  clearTimeout(biteTimer);
  if (!success) emit("notice", text);
  emit("resolve", success);
}

function clearBiteWindow() {
  clearTimeout(biteWindowTimer);
  clearInterval(biteCountdownTimer);
  biteWindowTimer = 0;
  biteCountdownTimer = 0;
  biteLeft.value = 0;
}

function beginBiteWindow() {
  clearBiteWindow();
  biteLeft.value = BITE_WINDOW_MS / 1000;
  message.value = `咬钩了！按 F 开始收线（${biteLeft.value} 秒内）`;
  biteWindowTimer = window.setTimeout(() => {
    if (phase.value === "bite" && !done.value) {
      finish(false, "鱼吃掉鱼饵，跑了");
    }
  }, BITE_WINDOW_MS);
  biteCountdownTimer = window.setInterval(() => {
    if (phase.value !== "bite" || done.value) return;
    biteLeft.value = Math.max(0, biteLeft.value - 1);
    message.value = `咬钩了！按 F 开始收线（还剩 ${biteLeft.value} 秒）`;
  }, 1000);
}

function startGame() {
  if (!props.session || done.value || phase.value !== "bite") return;
  clearBiteWindow();
  phase.value = "playing";
  graceLeft.value = GRACE_SECONDS;
  message.value = "WASD / 方向键 / 下方轮盘移动框，把鱼圈住";
  last = 0;
  raf = requestAnimationFrame(tick);
  // 横屏时页面上方还压着钓点/鱼饵等块，开打后把池塘顶到视野上方
  if (typeof matchMedia === "function" && matchMedia("(orientation: landscape)").matches) {
    pond.value?.scrollIntoView({ block: "start", behavior: "smooth" });
  }
}

function setFeel(mode: FeelMode) {
  feel.value = mode;
  try {
    window.localStorage.setItem(FEEL_KEY, mode);
  } catch {
    // 存不了就只影响本次；默认仍是当前手感。
  }
}

function setFight(mode: FightMode) {
  fightMode.value = mode;
  try {
    window.localStorage.setItem(FIGHT_KEY, mode);
  } catch {
    // 存不了就只影响本次；默认仍是 D2。
  }
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

/// 摇杆死区：半径的 20%。手指搭在旋钮上一定会抖几像素，
/// 键盘松手就是 0，触屏没有"松手"这回事——不留死区，框就会一直微动。
const STICK_DEAD = 0.2;

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

  const mag = Math.min(1, Math.hypot(dx, dy));
  if (mag <= STICK_DEAD) {
    // 死区内：当没动，旋钮也回中，让手能感觉到"这里不算"
    touchX.value = 0;
    touchY.value = 0;
    stickKnobX.value = 0;
    stickKnobY.value = 0;
    return;
  }

  // 死区外重新映射到 0..1，推到边缘仍然是满速
  const scaled = (mag - STICK_DEAD) / (1 - STICK_DEAD);
  const k = scaled / mag;
  touchX.value = dx * k;
  touchY.value = dy * k;
  stickKnobX.value = dx * k * 30;
  stickKnobY.value = dy * k * 30;
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
  if (tiltOn.value && !stickActive.value) {
    x += tiltX.value;
    y += tiltY.value;
  }
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

function applyCurrentFeel(
  input: { x: number; y: number },
  dt: number,
  speedPct: number,
  accelPct: number,
) {
  const accel = 650 * (accelPct / 100);
  frameVX.value += input.x * accel * dt;
  frameVY.value += input.y * accel * dt;
  const drag = Math.pow(0.88, dt * 60);
  frameVX.value *= drag;
  frameVY.value *= drag;
  const maxSpeed = 90 * (speedPct / 100);
  frameVX.value = Math.max(-maxSpeed, Math.min(maxSpeed, frameVX.value));
  frameVY.value = Math.max(-maxSpeed, Math.min(maxSpeed, frameVY.value));
}

function applyBadFeel(
  input: { x: number; y: number },
  dt: number,
  speedPct: number,
  worn: boolean,
) {
  const moving = input.x !== 0 || input.y !== 0;
  const dirX = Math.sign(input.x);
  const dirY = Math.sign(input.y);
  const reversalAfter = worn ? 350 : 260;
  if (moving) {
    const reversedX = badDirX !== 0 && dirX !== 0 && dirX !== badDirX;
    const reversedY = badDirY !== 0 && dirY !== 0 && dirY !== badDirY;
    if ((reversedX || reversedY) && badHoldMs > reversalAfter) {
      // 坏杆漂移更大；正常杆只留一小段。
      badReversalMs = worn ? 450 : 220;
    }
    if (dirX !== 0) badDirX = dirX;
    if (dirY !== 0) badDirY = dirY;
    badHoldMs += dt * 1000;
  } else {
    badHoldMs = Math.max(0, badHoldMs - dt * 2000);
  }

  // 长按追鱼加成：坏杆最多 +40%，正常杆 +35%；短按基本吃不到。
  const boost = 1 + Math.min(worn ? 0.4 : 0.35, (badHoldMs / 1000) * (worn ? 0.35 : 0.3));
  const speed = 90 * (speedPct / 100) * boost;
  const targetX = input.x * speed;
  const targetY = input.y * speed;

  let response: number;
  if (worn) {
    // 坏杆＝原来那套滑雪手感：换向 1.1、常驻 2.6、松手 1.6。
    response = moving ? (badReversalMs > 0 ? 1.1 : 2.6) : 1.6;
  } else if (!moving) {
    // 正常杆短按松手就停，不滑出去。
    response = 8;
  } else if (badReversalMs > 0) {
    // 长按换向只漂一小段，幅度小于坏杆。
    response = 3.4;
  } else {
    const hold = badHoldMs / 1000;
    // 前 0.25s 跟手（短按没惯性），之后才慢慢变“重”。
    response = hold < 0.25 ? 14 : 14 - Math.min(9, (hold - 0.25) * 9);
  }

  const alpha = 1 - Math.exp(-response * dt);
  frameVX.value += (targetX - frameVX.value) * alpha;
  frameVY.value += (targetY - frameVY.value) * alpha;
  badReversalMs = Math.max(0, badReversalMs - dt * 1000);
}

function tick(now: number) {
  if (done.value || phase.value !== "playing") return;
  const session = props.session;
  if (!session) return;
  if (!last) last = now;
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  elapsed += dt * 1000;
  if (session.isKing) {
    const ratio = elapsed / (session.params.catchMs * fightMultiplier.value);
    kingPhase.value = ratio < 0.3 ? "probe" : ratio < 0.7 ? "power" : "sprint";
  } else {
    kingPhase.value = "none";
  }

  if (Date.now() > session.expiresAt) {
    finish(false, "这一竿放太久了");
    return;
  }
  if (elapsed >= session.params.catchMs * fightMultiplier.value) {
    finish(false, "时间到，鱼跑了");
    return;
  }

  const input = axis();
  if (feel.value === "bad") {
    applyBadFeel(input, dt, session.params.frameSpeedPct, isWornRod.value);
  } else {
    applyCurrentFeel(
      input,
      dt,
      session.params.frameSpeedPct,
      session.params.frameAccelPct,
    );
  }
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
    const minMs = session.params.targetMinMs;
    const maxMs = session.params.targetMaxMs;
    targetTimer = (minMs + Math.random() * (maxMs - minMs)) / 1000;
    const jerk = session.params.fishJerkPct / 100;
    const range = 13 + jerk * 8;
    let angle: number;
    if (frameDist < session.params.fleeRadiusPct) {
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
    // 行为偏好：贴底/上层往固定水深靠，洄游更爱横穿。
    const [minY, maxY] = preferredDepth();
    const bias = depthBias();
    targetY = keepInPond(
      targetY * (1 - bias) + (minY + Math.random() * (maxY - minY)) * bias,
    );
    if (behavior.value === "migratory" && frameDist >= session.params.fleeRadiusPct) {
      const farX = fishX.value < 50 ? 70 + Math.random() * 13 : 17 + Math.random() * 13;
      targetX = keepInPond(targetX * 0.3 + farX * 0.7);
    }
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
  // 行为：贴底/上层垂直慢，洄游更平；爆发鱼周期性冲一下。
  // 逃逸时不再叠满爆发倍率，否则“逃逸×爆发”会把江面稀有鱼放大成疯狗。
  const burstSpeed = escaping ? 1.15 : 1.5;
  let behaviorSpeed = 1;
  if (behavior.value === "burst") {
    if (burstLeft > 0) {
      burstLeft = Math.max(0, burstLeft - dt);
      behaviorSpeed = burstSpeed;
    } else {
      burstCooldown -= dt;
      if (burstCooldown <= 0) {
        burstLeft = 0.55;
        burstCooldown = 2.6 + Math.random() * 1.2;
        behaviorSpeed = burstSpeed;
      } else {
        behaviorSpeed = 0.9;
      }
    }
  }
  const kingSpeed =
    kingPhase.value === "probe"
      ? 0.9
      : kingPhase.value === "power"
        ? 1.15
        : kingPhase.value === "sprint"
          ? 1.35
          : 1;
  const tensionSpeed = tension.value > 75 ? 1.15 : tension.value < 35 ? 0.9 : 1;
  const speed =
    (session.params.fishSpeedPct / 100) *
    70 *
    (escaping ? session.params.fishEscapePct / 100 : 1) *
    behaviorSpeed *
    kingSpeed *
    tensionSpeed;
  const verticalFactor =
    behavior.value === "bottom"
      ? 0.65
      : behavior.value === "surface"
        ? 0.85
        : behavior.value === "migratory"
          ? 0.75
          : 1;
  if (dist > 0.01) {
    const desiredX = (dx / dist) * speed;
    const desiredY = (dy / dist) * speed * verticalFactor;
    const turn = Math.min(1, dt * 2.4 * (session.params.turnPct / 100));
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
  // 第二维：张力。鱼在框内涨，框外回落；太高会绷断鱼线。
  const kingTension =
    kingPhase.value === "probe"
      ? 0.7
      : kingPhase.value === "power"
        ? 1
        : kingPhase.value === "sprint"
          ? 1.25
          : 1;
  const tensionGain =
    12 * ((session.params.tensionGainPct ?? 100) / 100) * kingTension;
  tension.value = Math.max(
    0,
    Math.min(100, tension.value + (inside ? tensionGain : -10) * dt),
  );
  if (tension.value >= 100) {
    finish(false, "鱼线绷断了，鱼跑了");
    return;
  }
  const tensionFactor =
    tension.value < 35 ? 0.8 : tension.value > 75 ? 0.85 : 1.15;
  trailAt += dt * 1000;
  if (trailAt >= 110) {
    trailAt = 0;
    trail.value = [
      { x: fishX.value, y: fishY.value, facing: fishFacing.value },
      ...trail.value,
    ].slice(0, 2);
  }
  if (inside) {
    progress.value +=
      (session.params.progressGain * dt * tensionFactor) / fightMultiplier.value;
    if (progress.value > peakProgress.value) {
      peakProgress.value = progress.value;
    }
  } else if (graceLeft.value <= 0) {
    progress.value -= (session.params.progressLoss * dt) / fightMultiplier.value;
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
  if (!props.active) return;
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

function onContextMenu(event: MouseEvent) {
  if (!props.active || !props.session) return;
  event.preventDefault();
  keys.clear();
  focusGame();
}

function onWindowFocus() {
  keys.clear();
  if (props.active && phase.value === "playing") {
    focusGame();
  }
}

function resetRound() {
  cancelAnimationFrame(raf);
  clearTimeout(biteTimer);
  clearBiteWindow();
  raf = 0;
  biteTimer = 0;
  keys.clear();
  done.value = false;
  progress.value = 35;
  peakProgress.value = 35;
  tension.value = 30;
  kingPhase.value = "none";
  inZone.value = false;
  graceLeft.value = 0;
  biteLeft.value = 0;
  badHoldMs = 0;
  badDirX = 0;
  badDirY = 0;
  badReversalMs = 0;
  burstLeft = 0;
  burstCooldown = 3;
  trail.value = [];
  frameX.value = 50;
  frameY.value = 50;
  frameVX.value = 0;
  frameVY.value = 0;
  fishX.value = 32;
  fishY.value = 38;
  fishTargetX.value = 32;
  fishTargetY.value = 38;
  fishVX.value = 0;
  fishVY.value = 0;
  fishFacing.value = -1;
  elapsed = 0;
  last = 0;
  stuckMs = 0;
  facingLockAt = 0;
  targetTimer = 0;
  trailAt = 0;
  stickActive.value = false;
  stickKnobX.value = 0;
  stickKnobY.value = 0;
}

function beginSession() {
  resetRound();
  phase.value = "waiting";
  message.value = "浮标落水了，等鱼咬钩……";
  const id = props.session?.id ?? 0;
  const delay = 1600 + (Number(id) % 3) * 700;
  biteTimer = window.setTimeout(() => {
    if (!props.session || done.value) return;
    phase.value = "bite";
    beginBiteWindow();
  }, delay);
}

function goIdle() {
  resetRound();
  phase.value = "idle";
  message.value = "按 F 抛竿，等浮标落水。";
}

watch(phase, () => {
  void nextTick(watchPond);
  // 只藏系统栏，不动方向：想横玩自己转，CSS 的 landscape 布局会自动接管。
  void setImmersive(phase.value !== "idle");
  // 横屏后页面还停在钓点/鱼饵那一屏，等旋转落定再把池塘顶到视野里
  if (phase.value !== "idle" && typeof matchMedia === "function" && matchMedia("(orientation: landscape)").matches) {
    window.setTimeout(() => {
      pond.value?.scrollIntoView({ block: "start", behavior: "smooth" });
    }, 450);
  }
});

watch(
  () => props.session?.id ?? null,
  (id, previous) => {
    if (id === previous) return;
    if (id === null) goIdle();
    else beginSession();
  },
);

onMounted(() => {
  if (SHOW_TILT && readTilt()) setTilt(true);
  // 开发模式：用 window.__tfTilt(beta, gamma) 灌一个假的设备姿态，
  // 这样在桌面浏览器里就能验重力感应，不用上真机/模拟器。
  if (import.meta.env.DEV) {
    (window as { __tfTilt?: (beta: number, gamma: number) => void }).__tfTilt = (
      beta: number,
      gamma: number,
    ) => {
      window.dispatchEvent(
        new DeviceOrientationEvent("deviceorientation", { alpha: 0, beta, gamma, absolute: false }),
      );
    };
  }
  window.addEventListener("keydown", onKey);
  window.addEventListener("keyup", onKeyUp);
  window.addEventListener("contextmenu", onContextMenu, true);
  window.addEventListener("focus", onWindowFocus);
  pond.value?.focus();
  if (props.session) beginSession();
  else goIdle();
});
onUnmounted(() => {
  cancelAnimationFrame(raf);
  clearTimeout(biteTimer);
  clearBiteWindow();
  pondObserver?.disconnect();
  pondObserver = null;
  window.removeEventListener("deviceorientation", onTilt);
  window.removeEventListener("keydown", onKey);
  window.removeEventListener("keyup", onKeyUp);
  window.removeEventListener("contextmenu", onContextMenu, true);
  window.removeEventListener("focus", onWindowFocus);
  onStickUp();
  // 离开小游戏：系统栏还回来
  void setImmersive(false);
});
</script>

<template>
  <div class="fishing-game" :data-phase="phase">
    <div class="fg-head">
      <strong>{{ session?.fishName ?? "还没抛竿" }}</strong>
      <span v-if="session" class="muted">
        <b v-if="session.isGolden" class="fg-golden">金色变异 · </b>
        {{ session.spotLabel }} · {{ session.params.personalityLabel }} ·
        {{ session.isKing ? "鱼王" : `难度 ${session.difficulty}` }}
      </span>
      <span v-else class="muted">按 F 抛竿，浮标落水后会等鱼咬钩</span>
      <span v-if="session" class="fg-behavior muted" :title="behaviorHint">
        行为：{{ behaviorLabel }}（{{ behaviorHint }}）
        <template v-if="kingPhaseLabel"> · {{ kingPhaseLabel }}</template>
      </span>
    </div>
    <div v-if="SHOW_TUNING" class="fg-feel">
      <span class="muted">手感</span>
      <button
        class="link"
        :class="{ active: feel === 'current' }"
        type="button"
        @click="setFeel('current')"
      >
        [1 旧]
      </button>
      <button
        class="link"
        :class="{ active: feel === 'bad' }"
        type="button"
        @click="setFeel('bad')"
      >
        [2 追鱼（默认）]
      </button>
      <button
        v-if="TOUCH_UI && SHOW_TILT"
        class="link"
        :class="{ active: tiltOn }"
        type="button"
        @click="setTilt(!tiltOn)"
      >
        [重力感应]
      </button>
      <span v-if="feel === 'current'" class="muted">旧手感（保留）</span>
      <span v-else-if="session" class="muted">
        {{ isWornRod ? "坏杆：长按加速，换向大漂移" : "正常杆：短按跟手，长按小漂移" }}
      </span>
      <span v-else class="muted">正常杆短按跟手；坏杆长按大漂移</span>
    </div>
    <div v-if="SHOW_TUNING" class="fg-feel">
      <span class="muted">打架</span>
      <button
        class="link"
        :class="{ active: fightMode === 'd2' }"
        type="button"
        @click="setFight('d2')"
      >
        [D2 中]
      </button>
      <button
        class="link"
        :class="{ active: fightMode === 'd3' }"
        type="button"
        @click="setFight('d3')"
      >
        [D3 长]
      </button>
      <span class="muted">
        {{ fightMode === "d3"
          ? "活泼 1.5× · 凶猛 1.8× · 鱼王/金色 2.2×"
          : "活泼 1.3× · 凶猛 1.5× · 鱼王/金色 1.8×" }}
      </span>
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
        <span class="fg-kicker">
          {{ phase === "bite" ? "浮标猛沉" : phase === "waiting" ? "浮标落水" : "准备抛竿" }}
        </span>
        <strong>
          {{ phase === "bite" ? "按 F 开始收线" : phase === "waiting" ? "等几秒，别惊了鱼……" : "按 F 抛竿" }}
        </strong>
      </div>
    </div>
    <div
      v-else
      ref="pond"
      class="fg-pond"
      :class="{ 'in-zone': inZone, 'is-king': session?.isKing }"
      :style="progressVars"
      tabindex="0"
      @click="focusGame"
      @pointerdown="focusGame"
    >
      <img class="fg-bg" :src="bgSrc" alt="" draggable="false" />
      <div class="fg-bg-shade"></div>
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
      <div class="fg-progress" :class="{ losing: isLosing }">
        <i class="fg-progress-ghost" :style="progressGhostStyle"></i>
        <span :style="progressStyle"></span>
      </div>
      <div
        class="fg-tension"
        :class="tensionState"
        title="张力 35–75 最佳；到 100 会绷断鱼线"
      >
        <span class="fg-tension-label">张力</span>
        <i class="fg-tension-track"><b :style="tensionStyle"></b></i>
        <span class="fg-tension-value">{{ Math.round(tension) }}%</span>
      </div>
      <div class="fg-trail">
        <img
          v-for="(point, index) in trail"
          :key="index"
          :src="session ? asset(`/fish/${session.fishId}.png`) : ''"
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
      </div>
      <div class="fg-fish-shadow" :style="fishStyle"></div>
      <img
        class="fg-fish"
        :src="session ? asset(`/fish/${session.fishId}.png`) : ''"
        :style="fishStyle"
        alt=""
      />
      <div class="fg-pond-vignette"></div>
    </div>

    <div class="fg-status">
      <span>{{ message }}</span>
      <span class="fg-zone-tag" :class="{ active: inZone }">{{ zoneText }}</span>
    </div>
    <div class="row">
      <button v-if="phase === 'bite'" class="link" type="button" :disabled="done" @click="startGame">
        [开始收线]
      </button>
      <button v-if="session" class="link" type="button" :disabled="done" @click="fail">
        [收竿]
      </button>
      <span v-if="phase === 'idle'" class="muted">按 F 或下方 [抛竿]</span>
      <span v-else-if="phase === 'waiting'" class="muted">F 抛竿后等几秒，咬钩再按 F</span>
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

    <!-- 手游键位：触屏时右下角浮一个大圆键，按阶段换内容 -->
    <div v-if="phase === 'bite' || phase === 'playing'" class="fg-pad">
      <button
        v-if="phase === 'bite'"
        class="fg-pad-btn primary"
        type="button"
        :disabled="done"
        @click="startGame"
      >
        收线
      </button>
      <button v-else class="fg-pad-btn ghost" type="button" :disabled="done" @click="fail">
        收竿
      </button>
    </div>
  </div>
</template>
