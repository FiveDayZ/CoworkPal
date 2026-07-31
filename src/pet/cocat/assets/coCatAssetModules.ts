import {
  coCatAssetManifest,
  coCatAssetMetaById,
  type CoCatAssetId,
} from "./coCatAssetManifest";
import {
  getCoCatAssetCandidatePaths,
  normalizeCoCatAssetPath,
  validateCoCatAssets,
} from "./coCatAssetValidator";

const assetModules = import.meta.glob("./cocat_skeleton/**/*.{png,svg}", {
  eager: true,
  import: "default",
  query: "?url",
}) as Record<string, string>;

const runtimeAssetUrlsByPath = Object.fromEntries(
  Object.entries(assetModules).map(([path, url]) => [
    normalizeCoCatAssetPath(path),
    url,
  ]),
) as Record<string, string>;

export const coCatRuntimeAssetReport = validateCoCatAssets(
  coCatAssetManifest,
  Object.keys(runtimeAssetUrlsByPath),
);

export function getCoCatRuntimeAssetUrl(assetId: CoCatAssetId) {
  const asset = coCatAssetMetaById[assetId];
  const resolvedPath = getCoCatAssetCandidatePaths(asset.path).find(
    (candidatePath) => runtimeAssetUrlsByPath[candidatePath],
  );

  return resolvedPath ? runtimeAssetUrlsByPath[resolvedPath] : null;
}
