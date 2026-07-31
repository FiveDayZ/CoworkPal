import type {
  CoCatBoneId,
  CoCatEyeState,
} from "../animation/animationTypes";

export type CoCatAssetId =
  | "shadow"
  | "tail_base"
  | "tail_mid"
  | "tail_tip"
  | "body_base"
  | "arm_left"
  | "arm_right_wrench"
  | "arm_right_fan"
  | "pouch"
  | "head_base"
  | "ears_left"
  | "ears_right"
  | "goggles"
  | "eye_normal"
  | "eye_blink"
  | "eye_focused"
  | "eye_dizzy"
  | "eye_sleepy"
  | "eye_glowing"
  | "ram_box"
  | "wrench_clone"
  | "badge_star"
  | "sleep_bubble";

export interface CoCatAssetMeta {
  id: CoCatAssetId;
  path: string;
  fallbackPath: string;
  width: number;
  height: number;
  pivotX: number;
  pivotY: number;
  anchorNode: CoCatBoneId;
  required: boolean;
}

const canvasAssetSize = {
  height: 160,
  width: 160,
};

function asset(
  id: CoCatAssetId,
  path: string,
  anchorNode: CoCatBoneId,
  pivot: [number, number],
): CoCatAssetMeta {
  return {
    ...canvasAssetSize,
    anchorNode,
    fallbackPath: `placeholder:${id}`,
    id,
    path,
    pivotX: pivot[0],
    pivotY: pivot[1],
    required: true,
  };
}

export const coCatAssetManifest: CoCatAssetMeta[] = [
  asset("shadow", "cocat_skeleton/shadow.png", "shadow", [80, 136]),
  asset("tail_base", "cocat_skeleton/tail/tail_base.png", "tail_base", [54, 104]),
  asset("tail_mid", "cocat_skeleton/tail/tail_mid.png", "tail_mid", [48, 103]),
  asset("tail_tip", "cocat_skeleton/tail/tail_tip.png", "tail_tip", [36, 99]),
  asset("body_base", "cocat_skeleton/body/body_base.png", "body_base", [80, 105]),
  asset("arm_left", "cocat_skeleton/body/arm_left.png", "arm_left", [58, 92]),
  asset(
    "arm_right_wrench",
    "cocat_skeleton/body/arm_right_wrench.png",
    "arm_right_wrench",
    [102, 92],
  ),
  asset(
    "arm_right_fan",
    "cocat_skeleton/body/arm_right_fan.png",
    "arm_right_wrench",
    [102, 92],
  ),
  asset("pouch", "cocat_skeleton/body/pouch.png", "pouch", [106, 112]),
  asset("head_base", "cocat_skeleton/head/head_base.png", "head_base", [80, 68]),
  asset("ears_left", "cocat_skeleton/head/ears_left.png", "ears_left", [57, 40]),
  asset("ears_right", "cocat_skeleton/head/ears_right.png", "ears_right", [103, 40]),
  asset("goggles", "cocat_skeleton/head/goggles.png", "goggles", [80, 62]),
  asset("eye_normal", "cocat_skeleton/eyes/eye_normal.png", "eyes", [80, 66]),
  asset("eye_blink", "cocat_skeleton/eyes/eye_blink.png", "eyes", [80, 66]),
  asset("eye_focused", "cocat_skeleton/eyes/eye_focused.png", "eyes", [80, 66]),
  asset("eye_dizzy", "cocat_skeleton/eyes/eye_dizzy.png", "eyes", [80, 66]),
  asset("eye_sleepy", "cocat_skeleton/eyes/eye_sleepy.png", "eyes", [80, 66]),
  asset("eye_glowing", "cocat_skeleton/eyes/eye_glowing.png", "eyes", [80, 66]),
  asset("ram_box", "cocat_skeleton/props/ram_box.png", "vfx_anchor", [80, 48]),
  asset(
    "wrench_clone",
    "cocat_skeleton/props/wrench_clone.png",
    "arm_right_wrench",
    [116, 74],
  ),
  asset("badge_star", "cocat_skeleton/props/badge_star.png", "vfx_anchor", [104, 34]),
  asset("sleep_bubble", "cocat_skeleton/props/sleep_bubble.png", "vfx_anchor", [
    99,
    64,
  ]),
];

export const coCatRequiredAssetIds = coCatAssetManifest
  .filter((assetMeta) => assetMeta.required)
  .map((assetMeta) => assetMeta.id);

export const coCatAssetMetaById = Object.fromEntries(
  coCatAssetManifest.map((assetMeta) => [assetMeta.id, assetMeta]),
) as Record<CoCatAssetId, CoCatAssetMeta>;

export const coCatBoneAssetMap: Partial<Record<CoCatBoneId, CoCatAssetId>> = {
  arm_left: "arm_left",
  arm_right_wrench: "arm_right_wrench",
  body_base: "body_base",
  ears_left: "ears_left",
  ears_right: "ears_right",
  goggles: "goggles",
  head_base: "head_base",
  pouch: "pouch",
  shadow: "shadow",
  tail_base: "tail_base",
  tail_mid: "tail_mid",
  tail_tip: "tail_tip",
};

export const coCatEyeAssetMap: Record<CoCatEyeState, CoCatAssetId> = {
  blink: "eye_blink",
  dizzy: "eye_dizzy",
  focused: "eye_focused",
  glowing: "eye_glowing",
  normal: "eye_normal",
  sleepy: "eye_sleepy",
};

export function getCoCatAssetIdForBone(
  boneId: CoCatBoneId,
  eyeState: CoCatEyeState,
): CoCatAssetId | null {
  if (boneId === "eyes") {
    return coCatEyeAssetMap[eyeState];
  }

  return coCatBoneAssetMap[boneId] ?? null;
}
