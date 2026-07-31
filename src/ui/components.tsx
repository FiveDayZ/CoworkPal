import type React from "react";
import { useThemedIcons } from "./assets";
import { RollingValue } from "./RollingValue";

type Tone = "cyan" | "orange" | "gold" | "green" | "red" | "muted";

function joinClassNames(
  ...classNames: Array<string | false | null | undefined>
) {
  return classNames.filter(Boolean).join(" ");
}

export interface GlassPanelProps extends React.HTMLAttributes<HTMLElement> {
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

export function GlassPanel({ children, className, style, ...props }: GlassPanelProps) {
  return (
    <section className={joinClassNames("cwp-glass-panel", className)} style={style} {...props}>
      {children}
    </section>
  );
}

export interface CompactButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "ghost" | "primary" | "danger";
}

export function CompactButton({
  className,
  variant = "ghost",
  type = "button",
  ...props
}: CompactButtonProps) {
  return (
    <button
      className={joinClassNames("cwp-compact-button", `is-${variant}`, className)}
      type={type}
      {...props}
    />
  );
}

export function PetAvatar({ className }: { className?: string }) {
  const icons = useThemedIcons();
  return (
    <span className={joinClassNames("cwp-pet-avatar", className)}>
      <img alt="CoCat" src={icons.cocatAvatar} />
    </span>
  );
}

export function ResourcePill({
  label,
  value,
  tone = "muted",
}: {
  label: string;
  value: string;
  tone?: Tone;
}) {
  return (
    <span className={joinClassNames("cwp-resource-pill", `tone-${tone}`)}>
      <span>{label}</span>
      <strong>{value}</strong>
    </span>
  );
}

export function StatusBadge({
  children,
  tone = "cyan",
}: {
  children: React.ReactNode;
  tone?: Tone;
}) {
  return (
    <span className={joinClassNames("cwp-status-badge", `tone-${tone}`)}>
      {children}
    </span>
  );
}

export function MetricCard({
  label,
  value,
  detail,
  level,
  tone = "cyan",
}: {
  label: string;
  value: string;
  detail?: string;
  level?: number | null;
  tone?: Tone;
}) {
  const safeLevel = Math.max(0, Math.min(100, level ?? 0));

  return (
    <div className={joinClassNames("cwp-metric-card", `tone-${tone}`)}>
      <div className="cwp-metric-card-head">
        <span>{label}</span>
        {detail ? <em>{detail}</em> : null}
      </div>
      <strong>{value}</strong>
      <span
        className="cwp-metric-bar"
        style={{ "--metric-level": `${safeLevel}%` } as React.CSSProperties}
      />
    </div>
  );
}

export function MonitorChip({
  label,
  value,
  detail,
  tone = "cyan",
}: {
  label: string;
  value: string;
  detail?: string;
  tone?: Tone;
}) {
  return (
    <span className={joinClassNames("cwp-monitor-chip", `tone-${tone}`)}>
      <span>{label}</span>
      <strong>
        <RollingValue value={value} />
      </strong>
      {detail ? <em>{detail}</em> : null}
    </span>
  );
}

export function PetSpeechBubble({
  children,
  visible,
}: {
  children: React.ReactNode;
  visible: boolean;
}) {
  return (
    <div className={joinClassNames("cwp-speech-bubble", visible && "is-visible")}>
      {children}
    </div>
  );
}

export function MiniSlider({
  label,
  max,
  min,
  step,
  value,
  valueLabel,
  onChange,
}: {
  label: string;
  max: number;
  min: number;
  step: number;
  value: number;
  valueLabel: string;
  onChange: (value: number) => void;
}) {
  return (
    <label className="cwp-mini-slider">
      <span>{label}</span>
      <input
        max={max}
        min={min}
        onChange={(event) => onChange(Number(event.target.value))}
        step={step}
        type="range"
        value={value}
      />
      <strong>{valueLabel}</strong>
    </label>
  );
}

export function ToggleSwitch({
  checked,
  label,
  description,
  onChange,
}: {
  checked: boolean;
  label: string;
  description?: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="cwp-toggle-switch">
      <span className="cwp-toggle-copy">
        <strong>{label}</strong>
        {description ? <em>{description}</em> : null}
      </span>
      <input
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        type="checkbox"
      />
      <span className="cwp-toggle-track" />
    </label>
  );
}

export interface ContextMenuItem {
  key: string;
  label: string;
  danger?: boolean;
  onClick: () => void;
}

export function ContextMenu({
  items,
  open,
}: {
  items: ContextMenuItem[];
  open: boolean;
}) {
  if (!open) {
    return null;
  }

  return (
    <div className="cwp-context-menu" onContextMenu={(event) => event.preventDefault()}>
      {items.map((item) => (
        <button
          className={joinClassNames(
            "cwp-context-menu-item",
            item.danger && "is-danger",
          )}
          key={item.key}
          onClick={item.onClick}
          type="button"
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}

export function WorkshopModuleCard({
  detail,
  image,
  label,
  level,
  status,
}: {
  detail: string;
  image: string;
  label: string;
  level: string;
  status: string;
}) {
  return (
    <article className="cwp-workshop-module-card">
      <img alt="" src={image} />
      <div>
        <span>{label}</span>
        <strong>{level}</strong>
        <em>{detail}</em>
      </div>
      <StatusBadge tone="orange">{status}</StatusBadge>
    </article>
  );
}

export function TitleBar({
  actions,
  center,
  stats,
  title = "CoworkPal",
  onDragStart,
}: {
  actions: React.ReactNode;
  center?: React.ReactNode;
  stats?: React.ReactNode;
  title?: string;
  onDragStart?: () => void;
}) {
  function handleMouseDown(event: React.MouseEvent<HTMLElement>) {
    if ((event.target as HTMLElement).closest("button")) {
      return;
    }

    onDragStart?.();
  }

  return (
    <header className="cwp-titlebar" onMouseDown={handleMouseDown}>
      <div className="cwp-titlebar-left">
        <PetAvatar />
        <strong>{title}</strong>
        <span
          className="cwp-titlebar-version"
          style={{
            fontFamily: "var(--font-pixel-title)",
            fontSize: "9px",
            color: "var(--color-text-muted)",
            letterSpacing: "0.5px",
            marginLeft: "4px",
            opacity: 0.7,
            userSelect: "none",
          }}
        >
          v{typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "0.1.0"}
        </span>
      </div>
      {center ? <div className="cwp-titlebar-center">{center}</div> : null}
      <div className="cwp-titlebar-right">
        <div className="cwp-titlebar-stats">{stats}</div>
        <div className="cwp-window-actions">{actions}</div>
      </div>
    </header>
  );
}

export function Sidebar({
  footer,
  items,
}: {
  footer?: React.ReactNode;
  items: Array<{
    key: string;
    label: React.ReactNode;
    active: boolean;
    onClick: () => void;
  }>;
}) {
  return (
    <aside className="cwp-sidebar">
      <nav className="cwp-sidebar-nav">
        {items.map((item) => (
          <button
            className={joinClassNames("cwp-sidebar-item", item.active && "is-active")}
            key={item.key}
            onClick={item.onClick}
            type="button"
          >
            {item.label}
          </button>
        ))}
      </nav>
      {footer ? <div className="cwp-sidebar-footer">{footer}</div> : null}
    </aside>
  );
}
