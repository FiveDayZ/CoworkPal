import { Component, type ErrorInfo, type ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Optional override for the fallback UI. Note: when set, the default UI
   *  (including the "重试" button that triggers `onReset`) is NOT rendered —
   *  use `compact` instead if you want a smaller UI that still has a retry. */
  fallback?: ReactNode;
  /**
   * Render a compact recovery UI (sized for small overlay windows like the
   * taskbar monitor) instead of the full-page fallback. Unlike `fallback`,
   * the compact UI still includes a "重试" button that triggers `onReset`, so
   * recovery remains reachable in tiny windows.
   */
  compact?: boolean;
  /**
   * Optional recovery hook invoked when the user clicks "重试". If the render
   * error originated from stale/invalid store data (rather than a transient
   * render condition), merely clearing the error state would re-render the same
   * bad data and re-throw, making "重试" a no-op death-loop. Providing this
   * lets the caller re-fetch authoritative data (e.g. `loadNotes()`) before the
   * subtree re-renders, giving recovery a real chance.
   */
  onReset?: () => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  message: string;
}

/**
 * Catches render-time errors anywhere in its subtree and shows a recovery UI
 * instead of letting the whole window go blank.
 *
 * Without this, a single thrown error (e.g. reading a field off `null` while
 * data is still loading) crashes the entire Tauri webview with no way back
 * short of restarting the window. Wrap each window's page content so a failure
 * in one page is contained.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false, message: "" };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, message: error.message ?? "未知错误" };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("ErrorBoundary caught a render error:", error, info);
  }

  private handleReset = (): void => {
    // Give the caller a chance to repair the underlying data before we re-render
    // the subtree (see onReset docs). Wrapped so a throw in the hook doesn't
    // prevent the state clear that lets the user see the (hopefully repaired) UI.
    try {
      this.props.onReset?.();
    } catch (error) {
      console.error("ErrorBoundary onReset hook threw:", error);
    }
    this.setState({ hasError: false, message: "" });
  };

  render(): ReactNode {
    if (!this.state.hasError) {
      return this.props.children;
    }

    if (this.props.fallback !== undefined) {
      return this.props.fallback;
    }

    // Compact mode for small overlay windows: minimal UI but still has a retry
    // button (unlike a bare `fallback`, which would make onReset unreachable).
    if (this.props.compact) {
      return (
        <div
          className="cwp-error-boundary cwp-error-boundary-compact"
          role="alert"
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            height: "100%",
            width: "100%",
            gap: 4,
            padding: 4,
            textAlign: "center",
            overflow: "hidden",
          }}
        >
          <div style={{ fontSize: 11, color: "var(--color-danger, #e55757)" }}>渲染异常</div>
          {import.meta.env.DEV && this.state.message ? (
            <div
              title={this.state.message}
              style={{
                fontSize: 9,
                color: "var(--color-text-muted)",
                maxWidth: "90%",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {this.state.message}
            </div>
          ) : null}
          <button
            type="button"
            onClick={this.handleReset}
            style={{
              padding: "2px 8px",
              border: "1px solid var(--color-border)",
              borderRadius: 4,
              background: "var(--color-surface-800)",
              color: "var(--color-text)",
              cursor: "pointer",
              fontSize: 10,
            }}
          >
            重试
          </button>
        </div>
      );
    }

    return (
      <div
        className="cwp-error-boundary"
        role="alert"
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 12,
          padding: 32,
          height: "100%",
          textAlign: "center",
          color: "var(--color-text)",
        }}
      >
        <div style={{ fontSize: 40, lineHeight: 1 }}>:(</div>
        <h2 style={{ margin: 0, fontSize: 18 }}>这一块出了点问题</h2>
        <p style={{ margin: 0, opacity: 0.7, fontSize: 13, maxWidth: 360 }}>
          页面渲染时发生了错误。可以尝试重新加载，或返回其他页面继续使用。
        </p>
        {import.meta.env.DEV && this.state.message ? (
          <pre
            style={{
              margin: 0,
              maxWidth: 480,
              padding: 8,
              background: "var(--color-surface-900, rgba(0,0,0,0.3))",
              borderRadius: 6,
              fontSize: 11,
              textAlign: "left",
              overflow: "auto",
              maxHeight: 120,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              color: "var(--color-danger, #e55757)",
            }}
          >
            {this.state.message}
          </pre>
        ) : null}
        <button
          type="button"
          onClick={this.handleReset}
          style={{
            marginTop: 4,
            padding: "8px 18px",
            border: "1px solid var(--color-border)",
            borderRadius: 8,
            background: "var(--color-surface-800)",
            color: "var(--color-text)",
            cursor: "pointer",
            fontSize: 13,
          }}
        >
          重试
        </button>
      </div>
    );
  }
}
