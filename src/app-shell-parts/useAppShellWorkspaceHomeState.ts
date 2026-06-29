import { useEffect, useMemo, useRef, useState } from "react";
import { homeDir } from "@tauri-apps/api/path";
import type { WorkspaceInfo } from "../types";
import {
  getHomeWorkspaceOptions,
  resolveHomeWorkspaceId,
} from "../features/home/utils/homeWorkspaceOptions";
import { recordStartupMilestone } from "../features/startup-orchestration/utils/startupTrace";
import { recordStartupPerfMarker } from "../services/perfBaseline/startupMarkers";
import { ensureWorkspacePathDir } from "../services/tauri";
import { getOfficeDefaultWorkspaceCandidatePaths } from "../features/workspaces/utils/defaultWorkspace";

type WorkspaceHomeStateParams = {
  activeWorkspaceId: string | null;
  addWorkspaceFromPath: (path: string) => Promise<WorkspaceInfo | null>;
  appSettingsLoading: boolean;
  groupedWorkspaces: Parameters<typeof getHomeWorkspaceOptions>[0];
  hasLoaded: boolean;
  userMode: "developer" | "office";
  workspaces: WorkspaceInfo[];
};

export function useAppShellWorkspaceHomeState({
  activeWorkspaceId,
  addWorkspaceFromPath,
  appSettingsLoading,
  groupedWorkspaces,
  hasLoaded,
  userMode,
  workspaces,
}: WorkspaceHomeStateParams) {
  const inputReadyMilestoneRecordedRef = useRef(false);
  const officeDefaultWorkspaceCreatedRef = useRef(false);

  useEffect(() => {
    if (inputReadyMilestoneRecordedRef.current || appSettingsLoading || !hasLoaded) {
      return;
    }
    inputReadyMilestoneRecordedRef.current = true;
    recordStartupMilestone("input-ready");
    recordStartupPerfMarker("first-interactive");
  }, [appSettingsLoading, hasLoaded]);

  const workspacesById = useMemo(
    () => new Map(workspaces.map((workspace) => [workspace.id, workspace])),
    [workspaces],
  );
  const workspacesByPath = useMemo(
    () => new Map(workspaces.map((workspace) => [workspace.path, workspace])),
    [workspaces],
  );
  const [homeOpen, setHomeOpen] = useState(true);
  const homeWorkspaceOptions = useMemo(
    () => getHomeWorkspaceOptions(groupedWorkspaces, workspaces),
    [groupedWorkspaces, workspaces],
  );
  const homeWorkspaceDefaultId = homeWorkspaceOptions[0]?.id ?? null;
  const homeWorkspaceSelectedId = useMemo(
    () => resolveHomeWorkspaceId(activeWorkspaceId, homeWorkspaceOptions),
    [activeWorkspaceId, homeWorkspaceOptions],
  );

  // Office mode: auto-create a visible default workspace (~/Documents/AI助手)
  // on first launch so white-collar users never see a "select project" prompt.
  useEffect(() => {
    if (
      userMode !== "office" ||
      !hasLoaded ||
      appSettingsLoading ||
      workspaces.length > 0 ||
      officeDefaultWorkspaceCreatedRef.current
    ) {
      return;
    }
    officeDefaultWorkspaceCreatedRef.current = true;
    void (async () => {
      try {
        const resolvedHome = await homeDir();
        const candidatePaths =
          getOfficeDefaultWorkspaceCandidatePaths(resolvedHome);
        for (const candidatePath of candidatePaths) {
          try {
            await ensureWorkspacePathDir(candidatePath);
            await addWorkspaceFromPath(candidatePath);
            break;
          } catch {
            // try next candidate path; stay silent for office users
          }
        }
      } catch {
        // office default workspace auto-create failed; user can add manually
      }
    })();
  }, [
    addWorkspaceFromPath,
    appSettingsLoading,
    hasLoaded,
    userMode,
    workspaces.length,
  ]);

  return {
    homeOpen,
    homeWorkspaceDefaultId,
    homeWorkspaceSelectedId,
    setHomeOpen,
    workspacesById,
    workspacesByPath,
  };
}
