import appIcon from "../assets/icons/app_icon.png";
import cocatAvatar from "../assets/icons/cocat_avatar.png";
import trayIcon from "../assets/icons/tray_icon.png";

// Themed Icons imports
import appIconOrange from "../assets/icons/app_icon_orange.png";
import cocatAvatarOrange from "../assets/icons/cocat_avatar_orange.png";
import trayIconOrange from "../assets/icons/tray_icon_orange.png";

import appIconBlue from "../assets/icons/app_icon_blue.png";
import cocatAvatarBlue from "../assets/icons/cocat_avatar_blue.png";
import trayIconBlue from "../assets/icons/tray_icon_blue.png";

import appIconGold from "../assets/icons/app_icon_gold.png";
import cocatAvatarGold from "../assets/icons/cocat_avatar_gold.png";
import trayIconGold from "../assets/icons/tray_icon_gold.png";

import moduleCpu from "../assets/modules/module_cpu_core_workbench.svg";
import moduleDisk from "../assets/modules/module_disk_archive_cabinet.svg";
import moduleGpu from "../assets/modules/module_gpu_graphic_bench.svg";
import moduleNet from "../assets/modules/module_net_transfer_station.svg";
import moduleRam from "../assets/modules/module_ram_parts_warehouse.svg";
import moduleTemp from "../assets/modules/module_temp_cooling_wall.svg";

import { useSettingsStore } from "../stores/settingsStore";

export const iconAssets = {
  app: appIcon,
  cocatAvatar,
  tray: trayIcon,
};

export const themeIconAssets = {
  coworkpal: {
    app: appIconOrange,
    cocatAvatar: cocatAvatarOrange,
    tray: trayIconOrange,
  },
  classic: {
    app: appIconOrange,
    cocatAvatar: cocatAvatarOrange,
    tray: trayIconOrange,
  },
  cyber: {
    app: appIconBlue,
    cocatAvatar: cocatAvatarBlue,
    tray: trayIconBlue,
  },
  steampunk: {
    app: appIconGold,
    cocatAvatar: cocatAvatarGold,
    tray: trayIconGold,
  },
};

export function useThemedIcons() {
  const settings = useSettingsStore((state) => state.settings);
  const theme = settings?.themeName || "coworkpal";
  return themeIconAssets[theme as keyof typeof themeIconAssets] || themeIconAssets.coworkpal;
}

export const moduleAssets = {
  cpu: moduleCpu,
  gpu: moduleGpu,
  ram: moduleRam,
  network: moduleNet,
  temperature: moduleTemp,
  disk: moduleDisk,
};

// Eagerly load all achievement badges for bundler compilation
const badgeImageModules = import.meta.glob(
  "../assets/achievements/*.webp",
  { eager: true, import: "default", query: "?url" },
) as Record<string, string>;

export const badgeAssets = Object.fromEntries(
  Object.entries(badgeImageModules).map(([path, url]) => {
    const filename = path.split("/").pop() ?? "";
    const badgeKey = filename.replace(/\.webp$/i, "");
    return [badgeKey, url];
  })
) as Record<string, string>;

