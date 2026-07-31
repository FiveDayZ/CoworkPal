import { Suspense, useEffect, useState } from "react";
import { AchievementUnlockToast } from "../../components/AchievementUnlockToast";
import { ErrorBoundary } from "../../components/ErrorBoundary";
import { mainRoutes, type MainRoute } from "../../routes";
import { formatParts } from "../../services/formatters";
import {
  showPetPanel,
  trackAchievementEvent,
} from "../../services/tauriCommands";
import {
  startDraggingCurrentWindow,
  minimizeMainWindow,
  isMainWindowZoomed,
  toggleMainWindowZoom,
  closeMainWindow,
} from "../../services/windowApi";
import { useUiStore } from "../../stores/uiStore";
import { useWorkshopStore } from "../../stores/workshopStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useAchievementStore } from "../../stores/achievementStore";
import {
  GlassPanel,
  Sidebar,
  TitleBar,
  PetAvatar,
} from "../../ui/components";
import { PixelIcon, type PixelIconName } from "../../ui/PixelIcon";

const routeLabels: Record<MainRoute, { icon: PixelIconName; text: string }> = {
  dashboard: { icon: "dashboard", text: "控制台" },
  devices: { icon: "devices", text: "设备" },
  workshop: { icon: "tools", text: "工坊" },
  settings: { icon: "settings", text: "设置" },
  workLog: { icon: "log", text: "日报" },
  health: { icon: "shield", text: "体检" },
  notes: { icon: "info", text: "笔记" },
  achievements: { icon: "achievement", text: "成就" },
  about: { icon: "info", text: "关于" },
};

/**
 * Reload the data stores the main-window pages render from, so a "重试" after a
 * data-driven render crash can repair the underlying data instead of looping on
 * the same broken snapshot. Dynamic imports keep this out of the initial bundle.
 */
function reloadPageStores() {
  void Promise.all([
    import("../../stores/notesStore"),
    import("../../stores/workshopStore"),
    import("../../stores/achievementStore"),
    import("../../stores/settingsStore"),
  ]).then(
    ([{ useNotesStore }, { useWorkshopStore }, { useAchievementStore }, { useSettingsStore }]) => {
      void useNotesStore.getState().loadNotes();
      void useWorkshopStore.getState().loadWorkshopState();
      void useAchievementStore.getState().loadSummary();
      void useSettingsStore.getState().loadSettings();
    },
  );
}

export function MainWindow() {
  const route = useUiStore((state) => state.mainRoute);
  const setRoute = useUiStore((state) => state.setMainRoute);
  const workshop = useWorkshopStore((state) => state.state);
  const settings = useSettingsStore((state) => state.settings);
  const achievementSummary = useAchievementStore((state) => state.summary);
  const pendingAchievementUnlockCount = useAchievementStore(
    (state) => state.unlockQueue.length,
  );
  const [mainWindowZoomed, setMainWindowZoomed] = useState(isMainWindowZoomed);
  const routeConfig = mainRoutes.find((item) => item.key === route);
  const CurrentPage = routeConfig?.element ?? mainRoutes[0].element;
  const achievementUnlockedCount = achievementSummary?.unlockedCount ?? 0;
  const unreadAchievementCount = Math.max(
    achievementSummary?.pendingNotificationCount ?? 0,
    pendingAchievementUnlockCount,
  );

  useEffect(() => {
    const pageKey = route === "workLog" ? "worklog" : route;
    void trackAchievementEvent({
      eventName: "page.view",
      occurredAt: Date.now(),
      idempotencyKey: `page.view:${pageKey}:${Date.now()}`,
      payload: { pageKey },
      source: "main-window",
    }).catch((error) => {
      console.error("Failed to track achievement page view", error);
    });
  }, [route]);

  async function handleMainWindowZoomToggle() {
    try {
      setMainWindowZoomed(await toggleMainWindowZoom());
    } catch (error) {
      console.error("Failed to toggle main window zoom", error);
    }
  }

  return (
    <main className={`cwp-main-root${mainWindowZoomed ? " is-window-zoomed" : ""}`}>
      <GlassPanel className="cwp-main-shell">
        <TitleBar
          actions={
            <>
              <button
                aria-label="Minimize"
                className="cwp-window-action"
                onClick={() => void minimizeMainWindow()}
                type="button"
                title="最小化"
              >
                <PixelIcon name="minimize" size={10} />
              </button>
              <button
                aria-label={mainWindowZoomed ? "Restore Window Size" : "Double Window Size"}
                className="cwp-window-action"
                onClick={() => void handleMainWindowZoomToggle()}
                type="button"
                title={mainWindowZoomed ? "还原窗口尺寸" : "窗口放大 2 倍"}
              >
                <PixelIcon name={mainWindowZoomed ? "windowShrink" : "windowExpand"} size={10} />
              </button>
              <button
                aria-label="Toggle Panel"
                className="cwp-window-action"
                onClick={() => void showPetPanel()}
                type="button"
                title="显示快捷面板"
              >
                <PixelIcon name="restore" size={10} />
              </button>
              <button
                aria-label="Close"
                className="cwp-window-action is-danger"
                onClick={() => void closeMainWindow()}
                type="button"
                title="关闭窗口"
              >
                <PixelIcon name="close" size={10} />
              </button>
            </>
          }
          onDragStart={() => void startDraggingCurrentWindow()}
          stats={
            <div className="cwp-window-stats">
              <div className="cwp-stat-pill" title="工坊等级">
                <span className="cwp-stat-pill-label">LV.</span>
                <span style={{ color: "var(--color-brand-orange-strong)", fontWeight: 700 }}>
                  {workshop?.workshopLevel ?? 1}
                </span>
              </div>
              <div className="cwp-stat-pill" title="攒下来的零件">
                <span className="cwp-stat-pill-label" style={{ display: "inline-flex", alignItems: "center" }}>
                  <PixelIcon name="wrench" size={12} style={{ color: "var(--color-brand-orange-strong)" }} />
                </span>
                <span style={{ color: "var(--color-text-primary)", marginLeft: "4px" }}>
                  {formatParts(workshop?.parts ?? 0)}
                </span>
              </div>
              <div className="cwp-stat-pill" title="获得灵感">
                <span className="cwp-stat-pill-label" style={{ display: "inline-flex", alignItems: "center" }}>
                  <PixelIcon name="lightbulb" size={12} style={{ color: "var(--color-insight-gold)" }} />
                </span>
                <span style={{ color: "var(--color-insight-gold)", marginLeft: "4px" }}>
                  {formatParts(workshop?.insight ?? 0)}
                </span>
              </div>
              <div className="cwp-stat-pill" title="成就点数">
                <span className="cwp-stat-pill-label" style={{ display: "inline-flex", alignItems: "center" }}>
                  <PixelIcon name="achievement" size={12} style={{ color: "#e8aa55" }} />
                </span>
                <span style={{ color: "#e8aa55", marginLeft: "4px" }}>
                  {formatParts(achievementSummary?.totalPoints ?? 0)}
                </span>
              </div>
            </div>
          }
        />

        <div className="cwp-window-body">
          <Sidebar
            footer={
              <div className="cwp-sidebar-status-card" style={{ flexDirection: "column", alignItems: "stretch", gap: "4px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <PetAvatar />
                  <div className="cwp-sidebar-status-info">
                    <span className="cwp-sidebar-status-name">
                      CoCat
                    </span>
                    <span className="cwp-sidebar-status-online">
                      ● 陪伴中
                    </span>
                  </div>
                </div>
                {settings?.catId && (
                  <div
                    style={{
                      fontFamily: "var(--font-pixel-title)",
                      fontSize: "8px",
                      color: "var(--color-text-muted)",
                      textAlign: "center",
                      borderTop: "1px dashed var(--color-border-soft)",
                      paddingTop: "2px",
                      letterSpacing: "0.5px",
                      userSelect: "text",
                    }}
                    title="您的宠物唯一识别码 CatID"
                  >
                    ID: {settings.catId}
                  </div>
                )}
              </div>
            }
            items={mainRoutes.map((item) => ({
              active: item.key === route,
              key: item.key,
              label: (
                <span className="cwp-sidebar-route-label">
                  <span className="cwp-sidebar-route-icon">
                    <PixelIcon name={routeLabels[item.key].icon} size={14} />
                  </span>
                  <span className="cwp-sidebar-route-text">
                    {routeLabels[item.key].text}
                  </span>
                  {item.key === "achievements" ? (
                    <AchievementSidebarMarker
                      count={achievementUnlockedCount}
                      hasPending={unreadAchievementCount > 0}
                    />
                  ) : null}
                </span>
              ),
              onClick: () => setRoute(item.key),
            }))}
          />

          <section className="cwp-main-content">
            <ErrorBoundary onReset={reloadPageStores}>
              <Suspense fallback={null}>
                <CurrentPage />
              </Suspense>
            </ErrorBoundary>
          </section>
        </div>
        <AchievementUnlockToast />
      </GlassPanel>
    </main>
  );
}

function AchievementSidebarMarker({
  count,
  hasPending,
}: {
  count: number;
  hasPending: boolean;
}) {
  if (count <= 0 && !hasPending) {
    return null;
  }

  return (
    <span
      className={`cwp-sidebar-achievement-marker${hasPending ? " has-pending" : ""}`}
      title={hasPending ? "有新的成就解锁" : `已解锁 ${count} 个成就`}
    >
      {count > 0 ? count : "新"}
    </span>
  );
}
