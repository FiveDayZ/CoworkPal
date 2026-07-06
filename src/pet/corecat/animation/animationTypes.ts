export type CoreCatBoneId =
  | "root"
  | "shadow"
  | "tail_base"
  | "tail_mid"
  | "tail_tip"
  | "body_base"
  | "arm_left"
  | "arm_right_wrench"
  | "head_base"
  | "ears_left"
  | "ears_right"
  | "goggles"
  | "eyes"
  | "pouch"
  | "vfx_anchor";

export type CoreCatEyeState =
  | "normal"
  | "blink"
  | "focused"
  | "dizzy"
  | "sleepy"
  | "glowing";

export type CoreCatAnimationState =
  | "bootWake"
  | "idle"
  | "hover"
  | "click"
  | "dragging"
  | "dropLanding"
  | "panelOpen"
  | "panelClose"
  | "temperatureCheck"
  | "memoryCrowded"
  | "repairing"
  | "dataSorting"
  | "scaredByMouse"
  | "eatingFish"
  | "pettingHearts"
  | "sleep"
  | "celebrate"
  | "workshopUpgrade"
  | "moduleUpgrade"
  | "updateInstalling"
  | "achievementPop"
  | "errorGlitch"
  | "lowPowerStatic"
  | "fatigued"
  | "needsBreak"
  | "freeMemory";

export interface BoneTransform {
  x?: number;
  y?: number;
  scaleX?: number;
  scaleY?: number;
  rotate?: number;
  opacity?: number;
}

export type CoreCatPose = Partial<Record<CoreCatBoneId, BoneTransform>>;

export interface CoreCatStarParticle {
  id: string;
  dx: number;
  dy: number;
  delayMs: number;
}

export interface CoreCatPointerContext {
  isInside: boolean;
  x: number;
  y: number;
}

export interface CoreCatAnimationContext {
  now: number;
  state: CoreCatAnimationState;
  stateElapsedMs: number;
  pointer: CoreCatPointerContext;
  blinkActive: boolean;
  earTwitch: "left" | "right" | null;
  reducedMotion: boolean;
  staticMode: boolean;
  lowPowerMode: boolean;
  isDragging: boolean;
  isClicking: boolean;
  updateProgress: number;
}

export interface CoreCatSkeletonNode {
  id: CoreCatBoneId;
  parentId?: CoreCatBoneId;
  label: string;
  zIndex: number;
  pivot: [number, number];
  defaultTransform: Required<Pick<BoneTransform, "x" | "y" | "scaleX" | "scaleY" | "rotate" | "opacity">>;
}
