import { formatBytes, formatPercent } from "../../services/formatters";
import { useHardwareStore } from "../../stores/hardwareStore";
import { PixelIcon } from "../../ui/PixelIcon";
import type { ProcessUsageSnapshot } from "../../types/hardware";

/**
 * Live "Top Processes" panel for the Dashboard. Renders the real-time
 * per-process samples pushed on the `hardware:metrics` event (already filtered,
 * scored and truncated to 32 by the Rust adapter).
 *
 * Pure-local data: only `pid` + `name` + instantaneous metrics. No exe path,
 * command line, or window title is ever collected (see README privacy notes).
 *
 * Visual treatment reuses the `cwp-process-*` classes established by the
 * WorkLog page's `ProcessInsightCard`, so it matches the existing pixel UI
 * without introducing new CSS.
 */
const TOP_N = 6;

/** Display label + tone for a recognized process category. */
interface CategoryTag {
  label: string;
  /** Matches the `is-*` modifier on `.cwp-process-card`. */
  tone: "" | "positive" | "warning";
}

/**
 * Client-side mirror of the Rust `classify_process` table. Used only to attach
 * a short category tag to each row for readability; the authoritative
 * classification (which drives CoreCat's bubble) lives in the backend.
 *
 * Patterns use word boundaries / delimiters so short names (go, arc, vim, tsc…)
 * don't match as substrings of unrelated words, matching the backend's
 * exact-match treatment of those names.
 */
function categoryFor(name: string): CategoryTag {
  // Normalized: lowercased, extension stripped. Keep delimiters so we can
  // anchor matches to name boundaries with `(^|[_-])`.
  const key = name.toLowerCase().replace(/\.exe$|\.bat$/, "").trim();
  // Exact-match short names (mirror of EXACT_RULES in process_classify.rs).
  const exactCompiler = ["go", "gcc", "csc", "vbc", "fsc", "swc", "tsc", "link", "make"];
  const exactIde = ["vim", "gvim", "nvim", "zed"];
  const exactMedia = ["mpv"];
  if (exactCompiler.includes(key)) {
    return { label: "编译", tone: "warning" };
  }
  if (exactIde.includes(key)) {
    return { label: "编辑器", tone: "positive" };
  }
  if (key === "arc") {
    return { label: "浏览器", tone: "" };
  }
  if (exactMedia.includes(key)) {
    return { label: "媒体", tone: "" };
  }
  // Substring rules for longer, unambiguous names.
  if (/(^|[_-])?(steam|epicgames|galaxyclient|battle\.net|riotclient|league of legends|genshinimpact|mihoyo|unity|unreal|javaw)/.test(key)) {
    return { label: "游戏", tone: "positive" };
  }
  if (/(node|npm|pnpm|yarn|esbuild|vite|webpack|rollup|msbuild|rustc|cargo|clang|dotnet|bazel|cmake|mingw32-make|gradle|maven|docker|java)/.test(key)) {
    return { label: "编译", tone: "warning" };
  }
  if (/(chrome|msedge|firefox|brave|opera|vivaldi)/.test(key)) {
    return { label: "浏览器", tone: "" };
  }
  if (/(code|code-insiders|cursor|devenv|idea|webstorm|pycharm|clion|goland|rustrover|rider|sublime_text|emacs|atom)/.test(key)) {
    return { label: "编辑器", tone: "positive" };
  }
  if (/(zoom|teams|slack|discord|wechat|dingtalk|feishu|lark|skype|tencentmeeting)/.test(key)) {
    return { label: "通讯", tone: "" };
  }
  if (/(vlc|spotify|foobar2000|netease_cloudmusic|cloudmusic|qqmusic|kugou|potplayer)/.test(key)) {
    return { label: "媒体", tone: "" };
  }
  return { label: "", tone: "" };
}

function ProcessRow({
  process,
  rank,
}: {
  process: ProcessUsageSnapshot;
  rank: number;
}) {
  const category = categoryFor(process.name);
  // CPU is the primary load signal; cap the bar at 100% for the fill width
  // (a single core-saturating process can report >100% on multi-core systems).
  const cpuWidth = Math.min(100, Math.max(0, process.cpuUsagePercent));

  return (
    <article className={`cwp-process-card${category.tone ? ` is-${category.tone}` : ""}`}>
      <div className="cwp-process-card-head">
        <span>#{rank}</span>
        <strong title={process.name}>{process.name}</strong>
        {category.label ? <em>{category.label}</em> : null}
      </div>
      <dl className="cwp-process-metrics">
        <div>
          <dt>CPU</dt>
          <dd>{formatPercent(process.cpuUsagePercent)}</dd>
        </div>
        <div>
          <dt>内存</dt>
          <dd>{formatBytes(process.memoryBytes)}</dd>
        </div>
        <div>
          <dt>PID</dt>
          <dd>{process.pid}</dd>
        </div>
      </dl>
      <div className="cwp-process-bar">
        <span style={{ width: `${cpuWidth}%` }} />
      </div>
    </article>
  );
}

export function ProcessListPanel() {
  const processes = useHardwareStore((state) => state.snapshot?.processes ?? []);
  const top = processes.slice(0, TOP_N);

  return (
    <section className="cwp-process-panel">
      <div className="cwp-process-head">
        <div className="cwp-section-title">
          <PixelIcon name="processPortrait" size={14} />
          <strong>实时进程 Top {TOP_N}</strong>
        </div>
        <div className="cwp-process-summary">
          <span>采样进程</span>
          <em>{processes.length} 个</em>
        </div>
      </div>
      {top.length === 0 ? (
        <div className="cwp-process-empty">
          CoreCat 还没有抓到活跃进程。系统产生负载后，这里会实时显示占用最高的进程。
        </div>
      ) : (
        <div className="cwp-process-grid">
          {top.map((process, index) => (
            <ProcessRow
              key={`${process.pid}-${process.name}`}
              process={process}
              rank={index + 1}
            />
          ))}
        </div>
      )}
    </section>
  );
}
