import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import type { MainRoute } from "./routeTypes";

export type { MainRoute } from "./routeTypes";

type MainRouteComponent = ComponentType | LazyExoticComponent<ComponentType>;

export const mainRoutes: Array<{
  key: MainRoute;
  label: string;
  element: MainRouteComponent;
}> = [
  {
    key: "dashboard",
    label: "Dashboard",
    element: lazy(() =>
      import("./pages/dashboard/DashboardPage").then((module) => ({
        default: module.DashboardPage,
      })),
    ),
  },
  {
    key: "focus",
    label: "Focus",
    element: lazy(() =>
      import("./pages/focus/FocusPage").then((module) => ({
        default: module.FocusPage,
      })),
    ),
  },
  {
    key: "devices",
    label: "Devices",
    element: lazy(() =>
      import("./pages/devices/DevicesPage").then((module) => ({
        default: module.DevicesPage,
      })),
    ),
  },
  {
    key: "workshop",
    label: "Workshop",
    element: lazy(() =>
      import("./pages/workshop/WorkshopPage").then((module) => ({
        default: module.WorkshopPage,
      })),
    ),
  },
  {
    key: "workLog",
    label: "Work Log",
    element: lazy(() =>
      import("./pages/work-log/WorkLogPage").then((module) => ({
        default: module.WorkLogPage,
      })),
    ),
  },
  {
    key: "health",
    label: "Health",
    element: lazy(() =>
      import("./pages/health/HealthPage").then((module) => ({
        default: module.HealthPage,
      })),
    ),
  },
  {
    key: "notes",
    label: "Notes",
    element: lazy(() =>
      import("./pages/notes/NotesPage").then((module) => ({
        default: module.NotesPage,
      })),
    ),
  },
  {
    key: "settings",
    label: "Settings",
    element: lazy(() =>
      import("./pages/settings/SettingsPage").then((module) => ({
        default: module.SettingsPage,
      })),
    ),
  },
  {
    key: "achievements",
    label: "Achievements",
    element: lazy(() =>
      import("./pages/achievements/AchievementsPage").then((module) => ({
        default: module.AchievementsPage,
      })),
    ),
  },
  {
    key: "about",
    label: "About",
    element: lazy(() =>
      import("./pages/about/AboutPage").then((module) => ({
        default: module.AboutPage,
      })),
    ),
  },
];
