import type { AppSettings } from "../types/settings";
export function effectiveCompanionSettings(settings: AppSettings | null): AppSettings | null {
  if (!settings || !settings.companionMode || settings.companionMode === "normal") return settings;
  return { ...settings, enableSound: false, enableNotifications: false, enablePetBubble: false,
    enableStaticCatMode: true, enableLowPowerMode: settings.companionMode === "economy" || settings.enableLowPowerMode };
}
