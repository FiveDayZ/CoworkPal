export const mainRouteKeys = [
  "dashboard",
  "workshop",
  "devices",
  "settings",
  "workLog",
  "health",
  "notes",
  "achievements",
  "about",
] as const;

export type MainRoute = (typeof mainRouteKeys)[number];

export function isMainRoute(route: string): route is MainRoute {
  return mainRouteKeys.includes(route as MainRoute);
}
