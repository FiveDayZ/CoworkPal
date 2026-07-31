import { CORE_CAT_ANIMATION_CONFIG } from "./animationConfig";
import { clamp } from "./animationCurves";

export interface CoCatHoverRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface CoCatHoverHitArea {
  isInside: boolean;
  x: number;
  y: number;
}

export function resolveCoCatHoverHitArea(
  clientX: number,
  clientY: number,
  rect: CoCatHoverRect | null,
): CoCatHoverHitArea {
  if (!rect) {
    return { isInside: false, x: 0, y: 0 };
  }

  const margin = CORE_CAT_ANIMATION_CONFIG.hover.hitAreaMarginPx;
  const right = rect.left + rect.width;
  const bottom = rect.top + rect.height;
  const isInside =
    clientX >= rect.left - margin &&
    clientX <= right + margin &&
    clientY >= rect.top - margin &&
    clientY <= bottom + margin;

  if (!isInside) {
    return { isInside: false, x: 0, y: 0 };
  }

  const centerX = rect.left + rect.width / 2;
  const centerY = rect.top + rect.height / 2;
  const radiusX = rect.width / 2 + margin;
  const radiusY = rect.height / 2 + margin;

  return {
    isInside,
    x: clamp((clientX - centerX) / radiusX, -1, 1),
    y: clamp((clientY - centerY) / radiusY, -1, 1),
  };
}
