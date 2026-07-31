import { useState } from "react";
import { iconAssets } from "../../ui/assets";
import { PixelIcon } from "../../ui/PixelIcon";
import { UpdateModal } from "../../components/UpdateModal";

export function AboutPage() {
  const [isUpdateOpen, setIsUpdateOpen] = useState(false);
  return (
    <div className="cwp-page">
      <div className="page-title-row">
        <h2 className="page-title">关于CoworkPal</h2>
      </div>

      <div className="cwp-about-content">
        {/* Hero Welcome Banner */}
        <div className="cwp-about-hero">
          <div className="cwp-about-hero-base" />
          <div>
            <h1 className="cwp-about-hero-title">CoworkPal 桌面伙伴</h1>
            <p className="cwp-about-hero-subtitle">
              让工作更有序，让创意更自由。这是一款纯本地运行的轻量级桌面伙伴，把枯燥的硬件数据变成一只像素猫咪 CoreCat 的活体表演。实时监控 CPU / GPU / 内存 / 温度等指标，自动生成每日工况日报与健康趋势体检，还能随手记录笔记与备忘；CoreCat 会根据你的负载和进程讲出专属小故事，并在工坊养成与成就收集中陪你走过每一个工作日——是贴心的工作伙伴，也是不打扰的观察者。
            </p>
            <div style={{ display: "flex", alignItems: "center", gap: "8px", marginTop: "6px" }}>
              <span className="cwp-about-version-tag">Version {typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "0.1.0"} (Release-Build)</span>
              <button
                onClick={() => setIsUpdateOpen(true)}
                className="cwp-custom-btn"
                style={{
                  background: "var(--color-brand-orange)",
                  border: "1px solid var(--color-border-strong)",
                  color: "var(--color-bg-950)",
                  fontSize: "10px",
                  padding: "2px 8px",
                  cursor: "pointer",
                  fontWeight: "bold",
                  fontFamily: "var(--font-pixel-title)",
                  lineHeight: 1
                }}
                type="button"
              >
                检查更新
              </button>
            </div>
          </div>
          <img
            src={iconAssets.corecatAvatar}
            alt="Mascot Celebrate"
            className="cwp-about-hero-mascot"
          />
        </div>

        {/* Git Repository Info */}
        <div className="cwp-about-card" style={{ height: "auto", padding: "12px 16px", gap: "8px" }}>
          <span className="cwp-about-card-title" style={{ fontSize: "13px", display: "flex", alignItems: "center" }}>
            <PixelIcon name="energy" size={14} style={{ marginRight: "8px" }} />
            开源项目仓库 (Git Repository)
          </span>
          <p className="cwp-about-card-p" style={{ fontSize: "11px", marginBottom: "4px" }}>
            欢迎访问项目主页提交 Issue 或 PR。让我们一起打磨出更好玩的硬件诊断工坊与桌面伴侣！
          </p>
          <div style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            background: "var(--color-bg-950)",
            border: "1px solid var(--color-border-soft)",
            padding: "6px 10px",
            borderRadius: "0px",
            marginTop: "6px"
          }}>
            <code style={{
              fontFamily: "var(--font-mono)",
              fontSize: "11px",
              color: "var(--color-brand-orange-strong)",
              userSelect: "all",
              wordBreak: "break-all"
            }}>
              https://github.com/FiveDayZ/CoworkPal.git
            </code>
            <button
              onClick={() => {
                void navigator.clipboard.writeText("https://github.com/FiveDayZ/CoworkPal.git");
                alert("Git 仓库地址已复制到剪贴板！");
              }}
              style={{
                background: "var(--color-bg-800)",
                border: "1px solid var(--color-border-soft)",
                color: "var(--color-text-primary)",
                fontSize: "11px",
                padding: "4px 10px",
                cursor: "pointer",
                marginLeft: "10px",
                fontFamily: "var(--font-pixel-title)",
                transition: "all 0.15s ease"
              }}
              className="cwp-custom-btn"
              type="button"
            >
              复制地址
            </button>
          </div>
        </div>

        {/* Safety & Privacy — migrated from the settings page */}
        <div className="cwp-about-card cwp-about-safety" style={{ height: "auto", padding: "12px 16px", gap: "8px" }}>
          <span className="cwp-about-card-title" style={{ fontSize: "13px", display: "flex", alignItems: "center" }}>
            <PixelIcon name="shield" size={14} style={{ marginRight: "8px" }} />
            安全与隐私 (Safety &amp; Privacy)
          </span>
          <p className="cwp-about-card-p" style={{ fontSize: "11px", marginBottom: "4px" }}>
            CoworkPal 是一款纯本地运行的桌面伴侣，您的信任是它存在的根基。以下是它对您的承诺：
          </p>
          <ul className="cwp-about-safety-list">
            <li>
              <strong>轻量绿色易安装</strong>
              <span>提供极简安装包，不写注册表、不捆绑任何第三方软件。</span>
            </li>
            <li>
              <strong>本地离线不联网</strong>
              <span>核心功能完全离线运行，除更新检查外不主动连接任何服务器。</span>
            </li>
            <li>
              <strong>后台无静默占用</strong>
              <span>仅在可见时采样硬件数据，闲置时自动进入低功耗，绝不挖矿或驻留后台任务。</span>
            </li>
            <li>
              <strong>用户数据不上传</strong>
              <span>工坊进度、成就、硬件数据全部存于本机，不收集、不上传任何个人信息。</span>
            </li>
          </ul>
        </div>
      </div>
      <UpdateModal isOpen={isUpdateOpen} onClose={() => setIsUpdateOpen(false)} />
    </div>
  );
}
