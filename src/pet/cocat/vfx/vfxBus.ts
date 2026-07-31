import type { CoCatVfxEvent } from "./vfxTypes";

export type CoCatVfxListener = (event: CoCatVfxEvent) => void;

export class CoCatVfxBus {
  private readonly listeners = new Set<CoCatVfxListener>();

  emit(event: CoCatVfxEvent) {
    this.listeners.forEach((listener) => listener(event));
  }

  subscribe(listener: CoCatVfxListener) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  clear() {
    this.listeners.clear();
  }
}

export function createCoCatVfxBus() {
  return new CoCatVfxBus();
}

export const coCatVfxBus = createCoCatVfxBus();
