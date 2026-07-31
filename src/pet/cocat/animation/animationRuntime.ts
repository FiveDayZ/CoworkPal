import type {
  CoCatAnimationContext,
  CoCatAnimationState,
  CoCatBoneId,
  CoCatPose,
} from "./animationTypes";
import {
  clamp,
  dampedOscillation,
  easeInOutSine,
  easeOutCubic,
  lerp,
  sine01,
} from "./animationCurves";
import { CORE_CAT_ANIMATION_CONFIG } from "./animationConfig";

export function sampleCoCatPose(
  state: CoCatAnimationState,
  ctx: CoCatAnimationContext,
): CoCatPose {
  if (ctx.staticMode) {
    return {};
  }

  if (state === "sleep" || state === "lowPowerStatic") {
    return sampleSleepPose(ctx);
  }

  if (state === "click") {
    return sampleIdlePose(ctx);
  }

  if (state === "bootWake") {
    return sampleBootWakePose(ctx);
  }

  if (state === "hover") {
    return mergePose(sampleIdlePose(ctx), sampleHoverPose(ctx));
  }

  if (state === "dragging") {
    return mergePose(sampleIdlePose(ctx), sampleDraggingPose(ctx));
  }

  if (state === "dropLanding") {
    return mergePose(sampleIdlePose(ctx), sampleDropLandingPose(ctx));
  }

  if (state === "panelOpen") {
    return mergePose(sampleIdlePose(ctx), samplePanelOpenPose(ctx));
  }

  if (state === "panelClose") {
    return mergePose(sampleIdlePose(ctx), samplePanelClosePose(ctx));
  }

  if (state === "errorGlitch") {
    return mergePose(sampleIdlePose(ctx), sampleErrorGlitchPose(ctx));
  }

  if (state === "updateInstalling") {
    return mergePose(sampleIdlePose(ctx), sampleUpdateInstallingPose(ctx));
  }

  if (state === "achievementPop") {
    return mergePose(sampleIdlePose(ctx), sampleAchievementPopPose(ctx));
  }

  if (state === "temperatureCheck") {
    return mergePose(sampleIdlePose(ctx), sampleTemperatureCheckPose(ctx));
  }

  if (state === "memoryCrowded") {
    return mergePose(sampleIdlePose(ctx), sampleMemoryCrowdedPose(ctx));
  }

  if (state === "repairing") {
    return mergePose(sampleIdlePose(ctx), sampleRepairingPose(ctx));
  }

  if (state === "dataSorting") {
    return mergePose(sampleIdlePose(ctx), sampleDataSortingPose(ctx));
  }

  if (state === "celebrate") {
    return mergePose(sampleIdlePose(ctx), sampleCelebratePose(ctx));
  }

  if (state === "workshopUpgrade") {
    return mergePose(sampleIdlePose(ctx), sampleWorkshopUpgradePose(ctx));
  }

  if (state === "moduleUpgrade") {
    return mergePose(sampleIdlePose(ctx), sampleModuleUpgradePose(ctx));
  }

  return sampleIdlePose(ctx);
}

export function applyCoCatActionOverlay(
  state: CoCatAnimationState,
  pose: CoCatPose,
  ctx: CoCatAnimationContext,
): CoCatPose {
  if (ctx.staticMode) {
    return pose;
  }

  if (state === "click") {
    return mergePose(pose, sampleClickPose(ctx));
  }

  return pose;
}

function sampleIdlePose(ctx: CoCatAnimationContext): CoCatPose {
  const config = CORE_CAT_ANIMATION_CONFIG.idle;
  const motionScale = ctx.reducedMotion ? 0.35 : 1;
  const breath = sine01((ctx.now % config.breathMs) / config.breathMs);
  const headBreath = sine01(
    ((ctx.now - config.headDelayMs) % config.breathMs) / config.breathMs,
  );
  const headSwing = Math.sin(
    ((ctx.now - config.headDelayMs) / config.headSwayMs) * Math.PI * 2,
  );
  const tail = Math.sin((ctx.now / config.tailSwingMs) * Math.PI * 2);
  const earTwitch = ctx.earTwitch
    ? dampedOscillation(
        (ctx.now % config.earTwitchDurationMs) / config.earTwitchDurationMs,
        2,
        4,
      )
    : 0;

  return {
    body_base: {
      scaleY: 1 + config.bodyScaleYAmplitude * breath * motionScale,
      scaleX: 1 - config.bodyScaleXAmplitude * breath * motionScale,
    },
    head_base: {
      y: config.headMoveY * headBreath * motionScale,
      rotate: config.headSwayDeg * headSwing * motionScale,
    },
    tail_base: {
      rotate: tail * config.tailBaseSwingDeg * motionScale,
    },
    tail_mid: {
      rotate: tail * config.tailMidSwingDeg * motionScale,
    },
    tail_tip: {
      rotate: tail * config.tailTipSwingDeg * motionScale,
    },
    shadow: {
      opacity: lerp(
        config.shadowMaxOpacity,
        config.shadowMinOpacity,
        breath * motionScale,
      ),
      scaleX: 1 - config.shadowScaleXAmplitude * breath * motionScale,
    },
    ears_left: {
      rotate: ctx.earTwitch === "left" ? -3 * earTwitch * motionScale : 0,
    },
    ears_right: {
      rotate: ctx.earTwitch === "right" ? 3 * earTwitch * motionScale : 0,
    },
    eyes: {
      scaleY: ctx.blinkActive ? 0.16 : 1,
    },
  };
}

function sampleHoverPose(ctx: CoCatAnimationContext): CoCatPose {
  const config = CORE_CAT_ANIMATION_CONFIG.hover;
  const nx = clamp(ctx.pointer.x / config.pointerInputMax, -1, 1);
  const ny = clamp(ctx.pointer.y / config.pointerInputMax, -1, 1);
  const headX = nx * config.headMaxX;
  const headY = ny * config.headMaxY;

  return {
    head_base: {
      x: headX,
      y: headY,
      rotate: nx * config.headRotateDeg,
    },
    goggles: {
      x: headX * config.gogglesCoefficient,
      y: headY * config.gogglesCoefficient,
    },
    eyes: {
      x: headX * config.eyesCoefficient,
      y: headY * config.eyesCoefficient,
    },
    ears_left: {
      rotate: nx * -config.earRotateDeg,
    },
    ears_right: {
      rotate: nx * -config.earRotateDeg,
    },
  };
}

function sampleClickPose(ctx: CoCatAnimationContext): CoCatPose {
  const config = CORE_CAT_ANIMATION_CONFIG.click;
  const elapsed = clamp(ctx.stateElapsedMs, 0, config.totalMs);

  if (elapsed <= config.compressEndMs) {
    const p = easeOutCubic(elapsed / config.compressEndMs);
    return {
      body_base: {
        y: lerp(0, 4, p),
        scaleX: lerp(1, 1.04, p),
        scaleY: lerp(1, 0.92, p),
      },
      head_base: {
        y: lerp(0, 1.5, p),
      },
      arm_right_wrench: {
        rotate: lerp(0, 30, p),
      },
    };
  }

  if (elapsed <= config.stretchEndMs) {
    const p = easeOutCubic(
      (elapsed - config.compressEndMs) /
        (config.stretchEndMs - config.compressEndMs),
    );
    return {
      body_base: {
        y: lerp(4, -3, p),
        scaleX: lerp(1.04, 0.97, p),
        scaleY: lerp(0.92, 1.06, p),
      },
      head_base: {
        y: lerp(1.5, -1.5, p),
      },
      arm_right_wrench: {
        rotate: lerp(30, 220, p),
      },
    };
  }

  const p =
    (elapsed - config.stretchEndMs) / (config.totalMs - config.stretchEndMs);
  const bounce = dampedOscillation(
    p,
    config.reboundCycles,
    config.reboundDecay,
  );
  const wrenchProgress = clamp(
    (elapsed - config.wrenchStartMs) /
      (config.wrenchEndMs - config.wrenchStartMs),
    0,
    1,
  );

  return {
    body_base: {
      y: 1.2 * bounce,
      scaleX: 1 - 0.012 * bounce,
      scaleY: 1 + 0.025 * bounce,
    },
    head_base: {
      y: 0.7 * bounce,
    },
    arm_right_wrench: {
      rotate: lerp(220, 360, easeOutCubic(wrenchProgress)),
    },
  };
}

function sampleBootWakePose(ctx: CoCatAnimationContext): CoCatPose {
  const totalMs = CORE_CAT_ANIMATION_CONFIG.oneShot.bootWakeMs;
  const elapsed = clamp(ctx.stateElapsedMs, 0, totalMs);
  const p = easeOutCubic(elapsed / totalMs);
  const wakeTwitch =
    Math.sin((ctx.now / 46) * Math.PI * 2) * (1 - p) * 1.2;

  return {
    root: {
      y: lerp(10, 0, p),
      scaleX: lerp(0.9, 1, p),
      scaleY: lerp(0.84, 1, p),
      opacity: lerp(0.32, 1, p),
    },
    body_base: {
      y: lerp(9, 0, p),
      scaleX: lerp(0.96, 1, p),
      scaleY: lerp(0.88, 1, p),
    },
    head_base: {
      y: lerp(14, 0, p),
      rotate: lerp(-5, 0, p) + wakeTwitch,
    },
    goggles: {
      y: lerp(-8, 0, p),
      opacity: lerp(0.72, 1, p),
    },
    eyes: {
      scaleY: elapsed < 280 ? 0.16 : lerp(0.16, 1, p),
      opacity: lerp(0.66, 1, p),
    },
    ears_left: {
      rotate: lerp(-10, 0, p) - wakeTwitch,
    },
    ears_right: {
      rotate: lerp(10, 0, p) + wakeTwitch,
    },
    tail_base: {
      x: lerp(16, 0, p),
      y: lerp(14, 0, p),
      rotate: lerp(42, 0, p),
    },
    tail_mid: {
      x: lerp(18, 0, p),
      y: lerp(14, 0, p),
      rotate: lerp(58, 0, p),
    },
    tail_tip: {
      x: lerp(20, 0, p),
      y: lerp(12, 0, p),
      rotate: lerp(76, 0, p),
    },
    shadow: {
      opacity: lerp(0.2, 0.42, p),
      scaleX: lerp(0.62, 1, p),
    },
  };
}

function sampleDraggingPose(ctx: CoCatAnimationContext): CoCatPose {
  const config = CORE_CAT_ANIMATION_CONFIG.hover;
  const nx = clamp(ctx.pointer.x / config.pointerInputMax, -1, 1);
  const ny = clamp(ctx.pointer.y / config.pointerInputMax, -1, 1);
  const sway = Math.sin((ctx.now / 210) * Math.PI * 2);

  return {
    root: {
      x: nx * 3,
      y: -2 + ny * 2,
      rotate: nx * 8,
    },
    body_base: {
      rotate: nx * 2,
      scaleX: 1.025,
      scaleY: 0.975,
    },
    head_base: {
      x: nx * 4,
      y: ny * 2,
      rotate: nx * 4 + sway * 0.7,
    },
    goggles: {
      x: nx * 5,
      y: ny * 2.4,
    },
    eyes: {
      x: nx * 6,
      y: ny * 2.8,
      scaleY: 0.76,
    },
    arm_left: {
      x: -nx * 2,
      y: -2,
      rotate: -18 + nx * 12,
    },
    arm_right_wrench: {
      x: nx * 3,
      y: -4,
      rotate: 22 + nx * 18,
    },
    tail_base: {
      x: -nx * 4,
      rotate: -nx * 10 + sway * 3,
    },
    tail_mid: {
      x: -nx * 5,
      rotate: -nx * 18 + sway * 5,
    },
    tail_tip: {
      x: -nx * 6,
      rotate: -nx * 26 + sway * 7,
    },
    shadow: {
      opacity: 0.29,
      scaleX: 0.82,
    },
  };
}

function sampleDropLandingPose(ctx: CoCatAnimationContext): CoCatPose {
  const totalMs = CORE_CAT_ANIMATION_CONFIG.oneShot.dropLandingMs;
  const elapsed = clamp(ctx.stateElapsedMs, 0, totalMs);

  if (elapsed <= 80) {
    const p = easeOutCubic(elapsed / 80);
    return {
      root: {
        y: lerp(-2, 5, p),
      },
      body_base: {
        scaleX: lerp(1, 1.07, p),
        scaleY: lerp(1, 0.88, p),
      },
      head_base: {
        y: lerp(0, 2, p),
      },
      shadow: {
        opacity: lerp(0.34, 0.5, p),
        scaleX: lerp(0.9, 1.1, p),
      },
    };
  }

  if (elapsed <= 180) {
    const p = easeOutCubic((elapsed - 80) / 100);
    return {
      root: {
        y: lerp(5, -3, p),
      },
      body_base: {
        scaleX: lerp(1.07, 0.97, p),
        scaleY: lerp(0.88, 1.06, p),
      },
      head_base: {
        y: lerp(2, -2, p),
      },
      tail_base: {
        rotate: lerp(-4, 6, p),
      },
      tail_mid: {
        rotate: lerp(-8, 10, p),
      },
      tail_tip: {
        rotate: lerp(-12, 15, p),
      },
    };
  }

  const p = (elapsed - 180) / (totalMs - 180);
  const bounce = dampedOscillation(p, 3, 5.2);

  return {
    root: {
      y: bounce * 1.6,
    },
    body_base: {
      scaleX: 1 - bounce * 0.012,
      scaleY: 1 + bounce * 0.02,
    },
    head_base: {
      y: bounce * 0.8,
      rotate: bounce * 1.2,
    },
    tail_base: {
      rotate: bounce * 4,
    },
    tail_mid: {
      rotate: bounce * 7,
    },
    tail_tip: {
      rotate: bounce * 10,
    },
  };
}

function samplePanelOpenPose(ctx: CoCatAnimationContext): CoCatPose {
  const elapsed = clamp(
    ctx.stateElapsedMs,
    0,
    CORE_CAT_ANIMATION_CONFIG.oneShot.panelOpenMs,
  );
  const p = easeOutCubic(
    elapsed / CORE_CAT_ANIMATION_CONFIG.oneShot.panelOpenMs,
  );

  return {
    body_base: {
      x: lerp(0, -1, p),
      rotate: lerp(0, -2, p),
    },
    head_base: {
      x: lerp(0, 2, p),
      rotate: lerp(0, 2.5, p),
    },
    eyes: {
      x: lerp(0, 2.2, p),
      scaleY: lerp(1, 0.75, p),
    },
    arm_right_wrench: {
      x: lerp(0, 15, p),
      y: lerp(0, -12, p),
      rotate: lerp(0, 64, p),
    },
    pouch: {
      y: lerp(0, -3, p),
      rotate: lerp(0, -5, p),
    },
    vfx_anchor: {
      x: lerp(0, 8, p),
      opacity: lerp(0.6, 1, p),
    },
  };
}

function samplePanelClosePose(ctx: CoCatAnimationContext): CoCatPose {
  const elapsed = clamp(
    ctx.stateElapsedMs,
    0,
    CORE_CAT_ANIMATION_CONFIG.oneShot.panelCloseMs,
  );
  const p = easeOutCubic(
    elapsed / CORE_CAT_ANIMATION_CONFIG.oneShot.panelCloseMs,
  );

  return {
    body_base: {
      rotate: lerp(-2, 0, p),
      scaleX: lerp(1.02, 1, p),
    },
    head_base: {
      x: lerp(2, 0, p),
      rotate: lerp(2.5, 0, p),
    },
    eyes: {
      x: lerp(2.2, 0, p),
      scaleY: lerp(0.75, 1, p),
    },
    arm_right_wrench: {
      x: lerp(15, 0, p),
      y: lerp(-12, 0, p),
      rotate: lerp(64, 0, p),
    },
    pouch: {
      y: lerp(-3, 0, p),
      rotate: lerp(-5, 0, p),
    },
    vfx_anchor: {
      x: lerp(8, 0, p),
      opacity: lerp(1, 0.65, p),
    },
  };
}

function sampleErrorGlitchPose(ctx: CoCatAnimationContext): CoCatPose {
  const elapsed = clamp(
    ctx.stateElapsedMs,
    0,
    CORE_CAT_ANIMATION_CONFIG.oneShot.errorGlitchMs,
  );
  const fade = 1 - elapsed / CORE_CAT_ANIMATION_CONFIG.oneShot.errorGlitchMs;
  const jitter = Math.sin((ctx.now / 43) * Math.PI * 2) * 2.4 * fade;
  const scan = Math.sin((ctx.now / 86) * Math.PI * 2);

  return {
    root: {
      x: jitter,
      y: scan * 0.8 * fade,
      opacity: 0.9 + Math.max(0, scan) * 0.1,
    },
    body_base: {
      scaleX: 1 + Math.abs(jitter) * 0.006,
      scaleY: 1 - Math.abs(jitter) * 0.005,
    },
    head_base: {
      x: -jitter * 0.7,
      rotate: jitter * 1.2,
    },
    goggles: {
      x: jitter * 1.4,
      opacity: 0.58 + Math.abs(scan) * 0.36,
    },
    eyes: {
      x: -jitter * 1.8,
      scaleY: 0.38 + Math.abs(scan) * 0.22,
    },
    ears_left: {
      rotate: -8 + jitter,
    },
    ears_right: {
      rotate: 8 + jitter,
    },
    arm_right_wrench: {
      rotate: 18 + jitter * 6,
    },
  };
}

function sampleUpdateInstallingPose(ctx: CoCatAnimationContext): CoCatPose {
  const progress = clamp(ctx.updateProgress, 0, 1);
  const pulse = Math.sin((ctx.now / 760) * Math.PI * 2);
  const scan = Math.sin((ctx.now / 1200) * Math.PI * 2);

  return {
    head_base: {
      x: scan * 1.2,
      rotate: scan * 0.8,
    },
    eyes: {
      x: scan * 2,
      scaleY: 0.72,
    },
    goggles: {
      opacity: 0.82 + Math.max(0, pulse) * 0.1,
    },
    arm_right_wrench: {
      x: lerp(-3, 11, progress),
      y: -7 + pulse * 0.6,
      rotate: lerp(-24, 32, progress) + pulse * 2,
    },
    pouch: {
      y: -2,
      rotate: -2 + pulse * 1.2,
    },
    vfx_anchor: {
      x: lerp(-12, 14, progress),
      y: -4,
      opacity: 0.75 + progress * 0.25,
    },
  };
}

function sampleAchievementPopPose(ctx: CoCatAnimationContext): CoCatPose {
  const totalMs = CORE_CAT_ANIMATION_CONFIG.oneShot.achievementPopMs;
  const elapsed = clamp(ctx.stateElapsedMs, 0, totalMs);
  const p = elapsed / totalMs;
  const pop = Math.sin(clamp(p * 1.4, 0, 1) * Math.PI);
  const wave = Math.sin((ctx.now / 520) * Math.PI * 2);

  return {
    root: {
      y: -pop * 4,
    },
    body_base: {
      scaleX: 1 + pop * 0.025,
      scaleY: 1 + pop * 0.035,
      rotate: wave * 1.5 * (1 - p),
    },
    head_base: {
      y: -pop * 3,
      rotate: wave * 2.4,
    },
    eyes: {
      scaleY: 0.86,
    },
    arm_left: {
      y: -pop * 8,
      rotate: -18 - pop * 20,
    },
    arm_right_wrench: {
      y: -pop * 10,
      rotate: 20 + pop * 42,
    },
    tail_base: {
      rotate: 9 + pop * 10 + wave * 2,
    },
    tail_mid: {
      rotate: 16 + pop * 18 + wave * 3,
    },
    tail_tip: {
      rotate: 24 + pop * 26 + wave * 4,
    },
    vfx_anchor: {
      y: -pop * 12,
      opacity: 1,
    },
  };
}

function sampleTemperatureCheckPose(ctx: CoCatAnimationContext): CoCatPose {
  const pulse = Math.sin((ctx.now / 680) * Math.PI * 2);
  const shake = Math.sin((ctx.now / (1000 / 15)) * Math.PI * 2);
  const fan = Math.sin((ctx.now / 80) * Math.PI * 2);

  return {
    root: {
      x: shake * 0.8,
    },
    body_base: {
      scaleX: 1.01 + Math.abs(shake) * 0.005,
      scaleY: 0.99,
    },
    head_base: {
      x: -1,
      y: -1,
      rotate: -3 + pulse * 0.8,
    },
    ears_left: {
      rotate: -7,
    },
    ears_right: {
      rotate: 7,
    },
    goggles: {
      y: -1,
      opacity: 0.92 + Math.max(0, pulse) * 0.08,
    },
    eyes: {
      scaleY: 0.68,
    },
    arm_right_wrench: {
      rotate: lerp(-15, 35, (fan + 1) / 2),
      x: 2,
      y: -3,
    },
    tail_base: {
      rotate: -4 + pulse,
    },
    tail_mid: {
      rotate: -7 + pulse * 1.5,
    },
    tail_tip: {
      rotate: -11 + pulse * 2,
    },
  };
}

function sampleMemoryCrowdedPose(ctx: CoCatAnimationContext): CoCatPose {
  const wobble = Math.sin((ctx.now / 520) * Math.PI * 2);
  const tremble = Math.sin((ctx.now / (1000 / 25)) * Math.PI * 2) * 0.5;
  const slip = Math.sin((ctx.now / 3000) * Math.PI * 2);
  const slipStep = slip > 0.92 ? (1 - slip) * 25 : 0;

  return {
    root: {
      x: slipStep,
      y: 4,
    },
    body_base: {
      y: 4,
      scaleX: 1.05,
      scaleY: 0.96,
    },
    head_base: {
      x: wobble * 1.2,
      y: 2,
      rotate: wobble * 1.8,
    },
    eyes: {
      scaleY: 0.62,
      x: wobble * 0.8,
    },
    pouch: {
      y: 2,
      rotate: -4 + wobble * 1.4,
    },
    arm_left: {
      x: -3,
      y: -14 + tremble,
      rotate: -55,
    },
    arm_right_wrench: {
      x: 3,
      y: -14 - tremble,
      rotate: 55,
    },
    tail_base: {
      rotate: 6 + wobble,
    },
    tail_mid: {
      rotate: 10 + wobble * 1.5,
    },
    tail_tip: {
      rotate: 14 + wobble * 2,
    },
  };
}

function sampleRepairingPose(ctx: CoCatAnimationContext): CoCatPose {
  const hit = Math.sin((ctx.now / 160) * Math.PI * 2);
  const tap = Math.sin((ctx.now / 320) * Math.PI * 2);

  return {
    root: {
      x: -2,
    },
    body_base: {
      x: -2,
      rotate: -6 + tap * 0.8,
      scaleX: 0.96,
    },
    head_base: {
      x: 1,
      y: -0.8,
      rotate: -2,
    },
    eyes: {
      scaleY: 0.74,
    },
    arm_left: {
      rotate: 18,
      y: -1,
    },
    arm_right_wrench: {
      rotate: lerp(-20, 45, (hit + 1) / 2),
      x: 6,
      y: -2,
    },
    tail_base: {
      rotate: -2,
    },
    tail_mid: {
      rotate: -4,
    },
    tail_tip: {
      rotate: -7,
    },
  };
}

function sampleDataSortingPose(ctx: CoCatAnimationContext): CoCatPose {
  const scan = Math.sin((ctx.now / 980) * Math.PI * 2);
  const t = (ctx.now % 1800) / 1800;
  const hand = cubicBezierPoint(
    t,
    { x: 6, y: -2 },
    { x: 16, y: -10 },
    { x: 22, y: 6 },
    { x: 10, y: 10 },
  );
  const nextHand = cubicBezierPoint(
    Math.min(1, t + 0.01),
    { x: 6, y: -2 },
    { x: 16, y: -10 },
    { x: 22, y: 6 },
    { x: 10, y: 10 },
  );
  const tangentDeg =
    (Math.atan2(nextHand.y - hand.y, nextHand.x - hand.x) * 180) / Math.PI;

  return {
    head_base: {
      x: scan * 1.6,
      rotate: scan * 0.7,
    },
    goggles: {
      x: scan * 2.2,
    },
    eyes: {
      x: scan * 2.8 + hand.x * 0.25,
      y: hand.y * 0.25,
      scaleY: 0.78,
    },
    arm_right_wrench: {
      x: hand.x,
      y: hand.y,
      rotate: tangentDeg * 0.45,
    },
    pouch: {
      y: -1,
      rotate: scan * 2,
    },
    vfx_anchor: {
      opacity: 0.7 + Math.max(0, scan) * 0.3,
    },
  };
}

function sampleCelebratePose(ctx: CoCatAnimationContext): CoCatPose {
  const totalMs = CORE_CAT_ANIMATION_CONFIG.celebrate.totalMs;
  const elapsed = clamp(ctx.stateElapsedMs, 0, totalMs);
  const wave = Math.sin((ctx.now / 640) * Math.PI * 2);

  if (elapsed <= 200) {
    const p = easeOutCubic(elapsed / 200);
    return {
      body_base: {
        y: lerp(0, 4, p),
        scaleX: lerp(1, 1.04, p),
        scaleY: lerp(1, 0.92, p),
      },
      tail_base: {
        rotate: lerp(0, -10, p),
      },
      tail_mid: {
        rotate: lerp(0, -16, p),
      },
      tail_tip: {
        rotate: lerp(0, -24, p),
      },
    };
  }

  if (elapsed <= 550) {
    const p = easeOutCubic((elapsed - 200) / 350);
    return {
      root: {
        y: lerp(4, -5, p),
      },
      body_base: {
        rotate: lerp(0, 10, p),
        scaleX: 1.02,
        scaleY: 1.04,
      },
      arm_right_wrench: {
        x: lerp(0, 8, p),
        y: lerp(0, -26, p),
        rotate: lerp(30, 260, p),
      },
      tail_base: {
        rotate: lerp(-10, 14, p),
      },
      tail_mid: {
        rotate: lerp(-16, 22, p),
      },
      tail_tip: {
        rotate: lerp(-24, 32, p),
      },
    };
  }

  if (elapsed <= 1000) {
    const p = (elapsed - 550) / 450;
    return {
      root: {
        y: -3 + Math.sin(p * Math.PI) * -2,
      },
      body_base: {
        rotate: Math.sin(p * Math.PI * 2) * 12,
        scaleX: 0.98 + Math.sin(p * Math.PI) * 0.04,
      },
      head_base: {
        rotate: wave * 3,
      },
      arm_right_wrench: {
        x: lerp(8, 1, p),
        y: lerp(-26, -2, p),
        rotate: lerp(260, 390, p),
      },
      tail_base: {
        rotate: 14 + wave * 4,
      },
      tail_mid: {
        rotate: 22 + wave * 8,
      },
      tail_tip: {
        rotate: 30 + wave * 11,
      },
    };
  }

  if (elapsed <= 1250) {
    const p = easeOutCubic((elapsed - 1000) / 250);
    return {
      head_base: {
        y: -2,
        rotate: wave * 1.6,
      },
      goggles: {
        y: lerp(0, 5, p),
      },
      eyes: {
        opacity: lerp(1, 0.72, p),
      },
      arm_right_wrench: {
        rotate: lerp(30, -8, p),
      },
    };
  }

  const p = easeInOutSine((elapsed - 1250) / 550);

  return {
    root: {
      y: lerp(-2, 0, p),
    },
    body_base: {
      x: lerp(0, -1.5, p),
      rotate: lerp(6, -3, p),
      scaleX: lerp(1.03, 1, p),
      scaleY: lerp(1.02, 1, p),
    },
    head_base: {
      y: lerp(-2, 0, p),
      rotate: wave * 1.2,
    },
    arm_left: {
      y: -4,
      rotate: -32,
    },
    arm_right_wrench: {
      y: -4,
      rotate: 34 + wave * 4,
    },
    tail_base: {
      rotate: 8 + wave * 3,
    },
    tail_mid: {
      rotate: 14 + wave * 5,
    },
    tail_tip: {
      rotate: 21 + wave * 7,
    },
    shadow: {
      opacity: 0.34,
      scaleX: 0.92,
    },
  };
}

function sampleWorkshopUpgradePose(ctx: CoCatAnimationContext): CoCatPose {
  const totalMs = CORE_CAT_ANIMATION_CONFIG.oneShot.workshopUpgradeMs;
  const elapsed = clamp(ctx.stateElapsedMs, 0, totalMs);
  const p = elapsed / totalMs;
  const lift = Math.sin(clamp(p * 1.8, 0, 1) * Math.PI);
  const wave = Math.sin((ctx.now / 520) * Math.PI * 2);
  const glow = Math.sin((ctx.now / 360) * Math.PI * 2);

  return {
    root: {
      y: -lift * 5,
    },
    body_base: {
      rotate: wave * 2.8 * (1 - p * 0.35),
      scaleX: 1 + lift * 0.03,
      scaleY: 1 + lift * 0.04,
    },
    head_base: {
      y: -lift * 4,
      rotate: wave * 2.2,
    },
    eyes: {
      scaleY: 0.9,
      opacity: 0.86 + Math.max(0, glow) * 0.14,
    },
    goggles: {
      y: -lift * 2,
      opacity: 0.88 + Math.max(0, glow) * 0.12,
    },
    arm_left: {
      y: -lift * 12,
      rotate: -22 - lift * 22 + wave * 3,
    },
    arm_right_wrench: {
      y: -lift * 16,
      rotate: 24 + lift * 78 + wave * 8,
    },
    pouch: {
      y: -lift * 3,
      rotate: -3 + glow * 2,
    },
    tail_base: {
      rotate: 8 + lift * 10 + wave * 4,
    },
    tail_mid: {
      rotate: 14 + lift * 16 + wave * 6,
    },
    tail_tip: {
      rotate: 22 + lift * 24 + wave * 8,
    },
    vfx_anchor: {
      y: -lift * 18,
      opacity: 1,
    },
  };
}

function sampleModuleUpgradePose(ctx: CoCatAnimationContext): CoCatPose {
  const totalMs = CORE_CAT_ANIMATION_CONFIG.oneShot.moduleUpgradeMs;
  const elapsed = clamp(ctx.stateElapsedMs, 0, totalMs);
  const p = elapsed / totalMs;
  const pulse = Math.sin((ctx.now / 420) * Math.PI * 2);
  const inspect = Math.sin((ctx.now / 980) * Math.PI * 2);
  const settle = easeOutCubic(clamp(p * 1.25, 0, 1));

  return {
    root: {
      x: inspect * 0.9 * (1 - p * 0.25),
    },
    body_base: {
      rotate: -2 + inspect * 1.2,
      scaleX: 0.98 + Math.max(0, pulse) * 0.015,
    },
    head_base: {
      x: 1 + inspect * 1.4,
      y: -1,
      rotate: -1.5 + inspect * 1.3,
    },
    eyes: {
      x: 1 + inspect * 2,
      scaleY: 0.72,
    },
    goggles: {
      x: inspect * 1.6,
      opacity: 0.9 + Math.max(0, pulse) * 0.1,
    },
    arm_left: {
      x: lerp(-2, -4, settle),
      y: lerp(0, -5, settle),
      rotate: lerp(0, -28, settle) + pulse * 2,
    },
    arm_right_wrench: {
      x: 5 + pulse * 1.5,
      y: -5 + pulse * 1.2,
      rotate: lerp(-12, 44, (pulse + 1) / 2),
    },
    pouch: {
      y: -2,
      rotate: pulse * 1.6,
    },
    tail_base: {
      rotate: 4 + inspect * 2,
    },
    tail_mid: {
      rotate: 7 + inspect * 3,
    },
    tail_tip: {
      rotate: 11 + inspect * 5,
    },
    vfx_anchor: {
      x: 10 + inspect * 4,
      y: -8,
      opacity: 0.8 + Math.max(0, pulse) * 0.2,
    },
  };
}

function cubicBezierPoint(
  t: number,
  p0: { x: number; y: number },
  p1: { x: number; y: number },
  p2: { x: number; y: number },
  p3: { x: number; y: number },
) {
  const clampedT = clamp(t, 0, 1);
  const u = 1 - clampedT;
  const uu = u * u;
  const tt = clampedT * clampedT;

  return {
    x:
      uu * u * p0.x +
      3 * uu * clampedT * p1.x +
      3 * u * tt * p2.x +
      tt * clampedT * p3.x,
    y:
      uu * u * p0.y +
      3 * uu * clampedT * p1.y +
      3 * u * tt * p2.y +
      tt * clampedT * p3.y,
  };
}

function sampleSleepPose(ctx: CoCatAnimationContext): CoCatPose {
  const config = CORE_CAT_ANIMATION_CONFIG.sleep;
  const breath = sine01((ctx.now % config.breathMs) / config.breathMs);
  const softBreath = easeInOutSine(breath);

  return {
    root: {
      y: config.rootY,
      scaleX: config.rootScaleX,
      scaleY: config.rootScaleY,
    },
    body_base: {
      y: config.bodyY,
      scaleX: config.bodyScaleX,
      scaleY: config.bodyScaleY + config.bodyScaleYAmplitude * softBreath,
    },
    head_base: {
      y: config.headY,
      rotate: config.headRotateDeg,
    },
    goggles: {
      y: config.gogglesY,
      rotate: config.gogglesRotateDeg,
    },
    eyes: {
      scaleY: 0.16,
      opacity: 0.92,
    },
    tail_base: {
      x: config.tailBaseX,
      y: config.tailBaseY,
      rotate: config.tailBaseRotateDeg + config.tailResonanceDeg * softBreath,
      scaleX: 0.84,
      scaleY: 0.92,
    },
    tail_mid: {
      x: config.tailMidX,
      y: config.tailMidY,
      rotate:
        config.tailMidRotateDeg + config.tailResonanceDeg * softBreath * 1.25,
      scaleX: 0.78,
      scaleY: 0.88,
    },
    tail_tip: {
      x: config.tailTipX,
      y: config.tailTipY,
      rotate:
        config.tailTipRotateDeg + config.tailResonanceDeg * softBreath * 1.5,
      scaleX: 0.78,
      scaleY: 0.88,
    },
    arm_left: {
      y: 6,
      rotate: 18,
    },
    arm_right_wrench: {
      y: 6,
      rotate: -18,
      opacity: 0.78,
    },
    shadow: {
      opacity: 0.32,
      scaleX: 0.76 + 0.04 * softBreath,
    },
  };
}

function mergePose(base: CoCatPose, override: CoCatPose): CoCatPose {
  const merged: CoCatPose = { ...base };

  Object.entries(override).forEach(([boneId, transform]) => {
    const nodeId = boneId as CoCatBoneId;
    merged[nodeId] = {
      ...(merged[nodeId] ?? {}),
      ...transform,
    };
  });

  return merged;
}
