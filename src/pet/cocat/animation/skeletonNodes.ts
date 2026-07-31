import type { CoCatSkeletonNode } from "./animationTypes";

export const COCAT_CANVAS_SIZE = {
  width: 160,
  height: 160,
};

const identityTransform = {
  x: 0,
  y: 0,
  scaleX: 1,
  scaleY: 1,
  rotate: 0,
  opacity: 1,
};

export const coCatSkeletonNodes: CoCatSkeletonNode[] = [
  {
    id: "root",
    label: "Root",
    zIndex: 1,
    pivot: [80, 120],
    defaultTransform: identityTransform,
  },
  {
    id: "shadow",
    parentId: "root",
    label: "Shadow",
    zIndex: 0,
    pivot: [80, 136],
    defaultTransform: identityTransform,
  },
  {
    id: "tail_base",
    parentId: "root",
    label: "Tail Base",
    zIndex: 10,
    pivot: [54, 104],
    defaultTransform: identityTransform,
  },
  {
    id: "tail_mid",
    parentId: "tail_base",
    label: "Tail Mid",
    zIndex: 11,
    pivot: [48, 103],
    defaultTransform: identityTransform,
  },
  {
    id: "tail_tip",
    parentId: "tail_mid",
    label: "Tail Tip",
    zIndex: 12,
    pivot: [36, 99],
    defaultTransform: identityTransform,
  },
  {
    id: "body_base",
    parentId: "root",
    label: "Body Base",
    zIndex: 20,
    pivot: [80, 105],
    defaultTransform: identityTransform,
  },
  {
    id: "arm_left",
    parentId: "body_base",
    label: "Left Arm",
    zIndex: 30,
    pivot: [58, 92],
    defaultTransform: identityTransform,
  },
  {
    id: "arm_right_wrench",
    parentId: "body_base",
    label: "Right Arm Wrench",
    zIndex: 35,
    pivot: [102, 92],
    defaultTransform: identityTransform,
  },
  {
    id: "pouch",
    parentId: "body_base",
    label: "Tool Pouch",
    zIndex: 36,
    pivot: [106, 112],
    defaultTransform: identityTransform,
  },
  {
    id: "head_base",
    parentId: "root",
    label: "Head Base",
    zIndex: 40,
    pivot: [80, 68],
    defaultTransform: identityTransform,
  },
  {
    id: "ears_left",
    parentId: "head_base",
    label: "Left Ear",
    zIndex: 42,
    pivot: [57, 40],
    defaultTransform: identityTransform,
  },
  {
    id: "ears_right",
    parentId: "head_base",
    label: "Right Ear",
    zIndex: 42,
    pivot: [103, 40],
    defaultTransform: identityTransform,
  },
  {
    id: "eyes",
    parentId: "head_base",
    label: "Eyes",
    zIndex: 45,
    pivot: [80, 66],
    defaultTransform: identityTransform,
  },
  {
    id: "goggles",
    parentId: "head_base",
    label: "Goggles",
    zIndex: 50,
    pivot: [80, 62],
    defaultTransform: identityTransform,
  },
  {
    id: "vfx_anchor",
    parentId: "root",
    label: "VFX Anchor",
    zIndex: 80,
    pivot: [118, 72],
    defaultTransform: {
      ...identityTransform,
      opacity: 0.55,
    },
  },
];

export const coCatSkeletonNodeMap = Object.fromEntries(
  coCatSkeletonNodes.map((node) => [node.id, node]),
);
