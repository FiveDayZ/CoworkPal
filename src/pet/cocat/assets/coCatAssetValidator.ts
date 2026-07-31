import type {
  CoCatAssetId,
  CoCatAssetMeta,
} from "./coCatAssetManifest";

export type CoCatAssetSource = "formal" | "placeholder";
export type CoCatAssetRenderMode = CoCatAssetSource;

export interface CoCatAssetValidationItem {
  asset: CoCatAssetMeta;
  exists: boolean;
  issues: string[];
  resolvedPath: string;
  source: CoCatAssetSource;
}

export interface CoCatAssetValidationReport {
  allRequiredSatisfied: boolean;
  availablePaths: string[];
  formalCount: number;
  items: CoCatAssetValidationItem[];
  missingRequired: CoCatAssetValidationItem[];
  placeholderCount: number;
}

export function normalizeCoCatAssetPath(path: string) {
  return path
    .replace(/\\/g, "/")
    .replace(/^\/+/, "")
    .replace(/^\.\//, "")
    .replace(/^src\/pet\/cocat\/assets\//, "");
}

export function getCoCatAssetCandidatePaths(path: string) {
  const normalizedPath = normalizeCoCatAssetPath(path);
  const extensionSwap =
    normalizedPath.endsWith(".png")
      ? normalizedPath.replace(/\.png$/, ".svg")
      : normalizedPath.endsWith(".svg")
        ? normalizedPath.replace(/\.svg$/, ".png")
        : null;

  return extensionSwap ? [normalizedPath, extensionSwap] : [normalizedPath];
}

export function validateCoCatAssets(
  manifest: CoCatAssetMeta[],
  availablePaths: string[],
): CoCatAssetValidationReport {
  const normalizedAvailablePaths = availablePaths.map(normalizeCoCatAssetPath);
  const availablePathSet = new Set(normalizedAvailablePaths);
  const items = manifest.map((asset) =>
    validateCoCatAsset(asset, availablePathSet),
  );
  const missingRequired = items.filter(
    (item) => item.asset.required && !item.exists,
  );

  return {
    allRequiredSatisfied: missingRequired.length === 0,
    availablePaths: normalizedAvailablePaths,
    formalCount: items.filter((item) => item.source === "formal").length,
    items,
    missingRequired,
    placeholderCount: items.filter((item) => item.source === "placeholder").length,
  };
}

export function getCoCatAssetValidationItem(
  report: CoCatAssetValidationReport,
  id: CoCatAssetId,
) {
  return report.items.find((item) => item.asset.id === id) ?? null;
}

export function resolveCoCatAssetRenderMode(
  _assetId: CoCatAssetId,
  assetUrl: string | null | undefined,
): CoCatAssetRenderMode {
  return assetUrl ? "formal" : "placeholder";
}

export function shouldShowCoCatAssetPanel(isDev: boolean) {
  return isDev;
}

function validateCoCatAsset(
  asset: CoCatAssetMeta,
  availablePathSet: Set<string>,
): CoCatAssetValidationItem {
  const candidatePaths = getCoCatAssetCandidatePaths(asset.path);
  const resolvedAssetPath =
    candidatePaths.find((candidatePath) => availablePathSet.has(candidatePath)) ??
    null;
  const exists = resolvedAssetPath != null;
  const issues: string[] = [];

  if (asset.required && !exists) {
    issues.push("missing required asset");
  }

  if (!asset.fallbackPath.trim()) {
    issues.push("empty fallback");
  }

  if (asset.width <= 0 || asset.height <= 0) {
    issues.push("invalid size");
  }

  if (
    asset.pivotX < 0 ||
    asset.pivotY < 0 ||
    asset.pivotX > asset.width ||
    asset.pivotY > asset.height
  ) {
    issues.push("pivot outside asset bounds");
  }

  return {
    asset,
    exists,
    issues,
    resolvedPath: resolvedAssetPath ?? asset.fallbackPath,
    source: exists ? "formal" : "placeholder",
  };
}
