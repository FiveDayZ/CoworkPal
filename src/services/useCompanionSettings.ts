import { useMemo } from "react";
import { useSettingsStore } from "../stores/settingsStore";
import { effectiveCompanionSettings } from "./companionSettings";
export function useCompanionSettings() {
  const settings = useSettingsStore((state) => state.settings);
  return useMemo(() => effectiveCompanionSettings(settings), [settings]);
}
