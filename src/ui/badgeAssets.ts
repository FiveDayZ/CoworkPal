const badgeImageModules = import.meta.glob(
  "../assets/achievements/*.webp",
  { eager: true, import: "default", query: "?url&no-inline" },
) as Record<string, string>;

export const badgeAssets = Object.fromEntries(
  Object.entries(badgeImageModules).map(([path, url]) => [
    path.split("/").pop()!.replace(/\.webp$/i, ""), url,
  ]),
) as Record<string, string>;
