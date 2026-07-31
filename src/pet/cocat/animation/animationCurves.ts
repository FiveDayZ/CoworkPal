export function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

export function lerp(from: number, to: number, weight: number) {
  return from + (to - from) * weight;
}

export function sine01(phase: number) {
  return 0.5 - Math.cos(phase * Math.PI * 2) / 2;
}

export function easeOutCubic(t: number) {
  const clamped = clamp(t, 0, 1);
  return 1 - Math.pow(1 - clamped, 3);
}

export function easeInOutSine(t: number) {
  const clamped = clamp(t, 0, 1);
  return 0.5 - Math.cos(Math.PI * clamped) / 2;
}

export function dampedOscillation(t: number, cycles = 3, decay = 4.8) {
  const clamped = clamp(t, 0, 1);
  return Math.exp(-decay * clamped) * Math.cos(Math.PI * 2 * cycles * clamped);
}
