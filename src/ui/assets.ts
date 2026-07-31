import appIcon from "../assets/icons/app_icon.png";
import corecatAvatar from "../assets/icons/corecat_avatar.png";
import trayIcon from "../assets/icons/tray_icon.png";

// Themed Icons imports
import appIconOrange from "../assets/icons/app_icon_orange.png";
import corecatAvatarOrange from "../assets/icons/corecat_avatar_orange.png";
import trayIconOrange from "../assets/icons/tray_icon_orange.png";

import appIconBlue from "../assets/icons/app_icon_blue.png";
import corecatAvatarBlue from "../assets/icons/corecat_avatar_blue.png";
import trayIconBlue from "../assets/icons/tray_icon_blue.png";

import appIconGold from "../assets/icons/app_icon_gold.png";
import corecatAvatarGold from "../assets/icons/corecat_avatar_gold.png";
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
  corecatAvatar,
  tray: trayIcon,
};

export const themeIconAssets = {
  coworkpal: {
    app: appIconOrange,
    corecatAvatar: corecatAvatarOrange,
    tray: trayIconOrange,
  },
  classic: {
    app: appIconOrange,
    corecatAvatar: corecatAvatarOrange,
    tray: trayIconOrange,
  },
  cyber: {
    app: appIconBlue,
    corecatAvatar: corecatAvatarBlue,
    tray: trayIconBlue,
  },
  steampunk: {
    app: appIconGold,
    corecatAvatar: corecatAvatarGold,
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

