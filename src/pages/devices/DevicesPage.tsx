import { useEffect, useState } from "react";
import { formatBytes } from "../../services/formatters";
import { getHardwareSnapshot } from "../../services/tauriCommands";
import { useHardwareStore } from "../../stores/hardwareStore";
import type {
  HardwareDeviceInfo,
  HardwareSnapshot,
  MemoryModuleInfo,
} from "../../types/hardware";
import { PixelIcon, type PixelIconName } from "../../ui/PixelIcon";

interface DeviceSection {
  key: string;
  label: string;
  icon: PixelIconName;
  items: string[];
}

export function DevicesPage() {
  const snapshot = useHardwareStore((state) => state.snapshot);
  const setSnapshot = useHardwareStore((state) => state.setSnapshot);
  const sections = buildDeviceSections(snapshot);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true); setError(null);
    void getHardwareSnapshot()
      .then((snapshot) => { if (!cancelled) setSnapshot(snapshot); })
      .catch((error) => { if (!cancelled) setError(String(error)); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [setSnapshot, retry]);

  return (
    <div className="cwp-page cwp-device-page">
      <div className="page-title-row">
        <h2 className="page-title">设备</h2>
      </div>

      <section className="cwp-device-panel">
        <div className="cwp-device-panel-head">
          <div className="cwp-device-title-wrap">
            <span className="cwp-device-head-icon">
              <PixelIcon name="devices" size={16} />
            </span>
            <div>
              <h3>本机硬件清单</h3>
              <p>软件启动时采样一次</p>
            </div>
          </div>
          <span className="cwp-device-refresh-tag">
            {snapshot ? "BOOT INVENTORY" : "WAITING"}
          </span>
        </div>

        {loading ? <p role="status">正在后台识别硬件，其他页面可正常使用…</p> : null}
        {error ? <p role="alert">设备读取失败：{error} <button type="button" className="cwp-compact-button" onClick={() => setRetry((n) => n + 1)}>重试</button></p> : null}
        <div className="cwp-device-list">
          {sections.map((section) => (
            <div className="cwp-device-row" key={section.key}>
              <div className="cwp-device-label">
                <span className="cwp-device-label-icon">
                  <PixelIcon name={section.icon} size={14} />
                </span>
                <span>{section.label}</span>
              </div>
              <div className="cwp-device-values">
                {section.items.length > 0 ? (
                  section.items.map((item, index) => (
                    <span key={`${section.key}-${index}`}>{item}</span>
                  ))
                ) : (
                  <span className="is-muted">{loading ? "等待系统返回设备信息" : "系统未提供此项信息"}</span>
                )}
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function buildDeviceSections(snapshot: HardwareSnapshot | null): DeviceSection[] {
  const inventory = snapshot?.deviceInventory;

  return [
    {
      key: "cpu",
      label: "处理器",
      icon: "cpu",
      items: compact([formatCpuDevice(snapshot)]),
    },
    {
      key: "motherboard",
      label: "主板",
      icon: "motherboard",
      items: formatDeviceList(inventory?.motherboard ?? []),
    },
    {
      key: "memory",
      label: "内存",
      icon: "ram",
      items: formatMemoryDevices(snapshot),
    },
    {
      key: "gpu",
      label: "显卡",
      icon: "gpu",
      items: formatDeviceList(inventory?.gpus ?? [], snapshot?.gpuName ?? null),
    },
    {
      key: "display",
      label: "显示器",
      icon: "monitor",
      items: formatDeviceList(inventory?.displays ?? []),
    },
    {
      key: "disk",
      label: "磁盘",
      icon: "disk",
      items: formatDeviceList(inventory?.disks ?? []),
    },
    {
      key: "audio",
      label: "声卡",
      icon: "audio",
      items: formatDeviceList(inventory?.audioDevices ?? []),
    },
    {
      key: "network",
      label: "网卡",
      icon: "network",
      items: formatDeviceList(inventory?.networkAdapters ?? []),
    },
  ];
}

function formatCpuDevice(snapshot: HardwareSnapshot | null) {
  if (!snapshot?.cpuName) {
    return null;
  }

  const coreText = compact([
    snapshot.cpuPhysicalCoreCount != null ? `${snapshot.cpuPhysicalCoreCount} 核` : null,
    snapshot.cpuLogicalCoreCount != null ? `${snapshot.cpuLogicalCoreCount} 线程` : null,
  ]).join(" / ");

  return coreText ? `${snapshot.cpuName}（${coreText}）` : snapshot.cpuName;
}

function formatMemoryDevices(snapshot: HardwareSnapshot | null) {
  const inventory = snapshot?.deviceInventory;
  const modules = inventory?.memoryModules ?? [];
  const moduleSizeText = modules
    .map((module) => formatBytes(module.capacityBytes))
    .filter((value) => value !== "N/A");
  const summary = snapshot?.totalMemoryBytes
    ? `总计 ${formatBytes(snapshot.totalMemoryBytes)}${
        moduleSizeText.length > 0 ? `（${moduleSizeText.join(" + ")}）` : ""
      }`
    : null;

  return compact([summary, ...modules.map(formatMemoryModule)]);
}

function formatMemoryModule(module: MemoryModuleInfo) {
  const title = compact([module.manufacturer, module.partNumber]).join(" ");
  const specs = compact([
    formatBytes(module.capacityBytes),
    module.speedMhz != null ? `${module.speedMhz}MHz` : null,
  ]).filter((value) => value !== "N/A");

  if (!title && specs.length === 0) {
    return null;
  }

  if (title && specs.length > 0) {
    return `${title}（${specs.join(" / ")}）`;
  }

  return title || specs.join(" / ");
}

function formatDeviceList(devices: HardwareDeviceInfo[], fallbackName?: string | null) {
  const items = devices.map(formatDeviceInfo).filter((item): item is string => Boolean(item));
  if (items.length > 0) {
    return items;
  }

  return fallbackName ? [fallbackName] : [];
}

function formatDeviceInfo(device: HardwareDeviceInfo) {
  if (!device.name) {
    return null;
  }

  const detailParts = compact([
    formatBytes(device.capacityBytes),
    device.vendor,
    device.detail,
  ]).filter((value) => value !== "N/A");

  if (detailParts.length === 0) {
    return device.name;
  }

  return `${device.name}（${detailParts.join(" / ")}）`;
}

function compact<T>(values: Array<T | null | undefined | false | "">): T[] {
  return values.filter(Boolean) as T[];
}
