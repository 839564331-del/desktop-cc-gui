const DEFAULT_WORKSPACE_SUFFIXES = [
  "/.ccgui/workspace",
  "/.mossx/workspace",
  "/.codemoss/workspace",
  "/com.zhukunpenglinyutong.ccgui/workspace",
  "/com.zhukunpenglinyutong.mossx/workspace",
  "/com.zhukunpenglinyutong.codemoss/workspace",
];

function normalizeWorkspaceHomePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

export function normalizeWorkspacePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

export function isDefaultWorkspacePath(path: string): boolean {
  const normalized = normalizeWorkspacePath(path);
  return DEFAULT_WORKSPACE_SUFFIXES.some((suffix) => normalized.endsWith(suffix));
}

export function getDefaultWorkspaceCandidatePaths(homePath: string): string[] {
  const normalizedHomePath = normalizeWorkspaceHomePath(homePath);
  if (!normalizedHomePath) {
    return [];
  }
  return DEFAULT_WORKSPACE_SUFFIXES.map((suffix) => `${normalizedHomePath}${suffix}`);
}

// Office (white-collar) mode uses a visible directory under the user's Documents,
// not the hidden .ccgui/workspace used by developer mode.
const OFFICE_DEFAULT_WORKSPACE_SUFFIXES = ["/Documents/AI助手"];

export function isOfficeDefaultWorkspacePath(path: string): boolean {
  const normalized = normalizeWorkspacePath(path);
  return OFFICE_DEFAULT_WORKSPACE_SUFFIXES.some((suffix) =>
    normalized.endsWith(suffix.toLowerCase()),
  );
}

export function getOfficeDefaultWorkspaceCandidatePaths(
  homePath: string,
): string[] {
  const normalizedHomePath = normalizeWorkspaceHomePath(homePath);
  if (!normalizedHomePath) {
    return [];
  }
  return OFFICE_DEFAULT_WORKSPACE_SUFFIXES.map(
    (suffix) => `${normalizedHomePath}${suffix}`,
  );
}
