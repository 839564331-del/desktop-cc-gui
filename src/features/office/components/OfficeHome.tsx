import { type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import PenLine from "lucide-react/dist/esm/icons/pen-line";
import AlignLeft from "lucide-react/dist/esm/icons/align-left";
import Table2 from "lucide-react/dist/esm/icons/table-2";
import Languages from "lucide-react/dist/esm/icons/languages";
import Lightbulb from "lucide-react/dist/esm/icons/lightbulb";
import TrendingUp from "lucide-react/dist/esm/icons/trending-up";
import MessageSquare from "lucide-react/dist/esm/icons/message-square";

type LatestAgentRun = {
  message: string;
  timestamp: number;
  projectName: string;
  workspaceId: string;
  threadId: string;
  isProcessing: boolean;
};

// P2 placeholder experts. P3 will wire each id to a role system-prompt
// (Collaboration Modes + Custom Prompts + Skills). For now every card just
// focuses the composer so the user starts a generic conversation.
const OFFICE_EXPERTS = [
  { id: "writing", nameKey: "office.expert.writing", icon: PenLine },
  { id: "summary", nameKey: "office.expert.summary", icon: AlignLeft },
  { id: "spreadsheet", nameKey: "office.expert.spreadsheet", icon: Table2 },
  { id: "translate", nameKey: "office.expert.translate", icon: Languages },
  { id: "productDesign", nameKey: "office.expert.productDesign", icon: Lightbulb },
  { id: "finance", nameKey: "office.expert.finance", icon: TrendingUp },
] as const;

type OfficeHomeProps = {
  latestAgentRuns: LatestAgentRun[];
  onSelectThread: (workspaceId: string, threadId: string) => void;
  composerNode?: ReactNode;
};

export function OfficeHome({
  latestAgentRuns,
  onSelectThread,
  composerNode,
}: OfficeHomeProps) {
  const { t } = useTranslation();

  function focusComposer() {
    const textarea = document.querySelector<HTMLTextAreaElement>(
      ".office-home-composer-host textarea",
    );
    textarea?.focus();
  }

  return (
    <div
      className="office-home"
      style={{
        display: "flex",
        justifyContent: "center",
        padding: "48px 24px",
        minHeight: "100%",
      }}
    >
      <div
        className="office-home-shell"
        style={{ width: "100%", maxWidth: 720, display: "flex", flexDirection: "column", gap: 32 }}
      >
        <header className="office-home-hero" style={{ textAlign: "center" }}>
          <h1 className="office-home-title" style={{ fontSize: 28, fontWeight: 600, margin: 0 }}>
            {t("office.home.dailyWork")}
          </h1>
          <p className="office-home-subtitle" style={{ margin: "8px 0 0", opacity: 0.7 }}>
            {t("office.home.dailyWorkDesc")}
          </p>
        </header>

        <section
          className="office-home-composer-host"
          aria-label={t("office.home.dailyWork")}
        >
          {composerNode}
        </section>

        <section className="office-home-experts">
          <h2 className="office-home-section-title" style={{ fontSize: 14, fontWeight: 600, opacity: 0.7, margin: "0 0 12px" }}>
            {t("office.home.experts")}
          </h2>
          <div
            className="office-home-expert-grid"
            style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 }}
          >
            {OFFICE_EXPERTS.map((expert) => {
              const Icon = expert.icon;
              return (
                <button
                  key={expert.id}
                  type="button"
                  className="office-home-expert-card"
                  onClick={focusComposer}
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: 8,
                    padding: "20px 12px",
                    borderRadius: 12,
                    border: "1px solid var(--border, rgba(128,128,128,0.2))",
                    background: "var(--surface, rgba(128,128,128,0.05))",
                    cursor: "pointer",
                  }}
                >
                  <span className="office-home-expert-icon" aria-hidden>
                    <Icon size={24} />
                  </span>
                  <span className="office-home-expert-name" style={{ fontSize: 13 }}>
                    {t(expert.nameKey)}
                  </span>
                </button>
              );
            })}
          </div>
        </section>

        <section className="office-home-recent">
          <h2 className="office-home-section-title" style={{ fontSize: 14, fontWeight: 600, opacity: 0.7, margin: "0 0 12px" }}>
            {t("office.home.recentSessions")}
          </h2>
          {latestAgentRuns.length > 0 ? (
            <ul className="office-home-recent-list" style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 6 }}>
              {latestAgentRuns.slice(0, 8).map((run) => (
                <li key={run.threadId}>
                  <button
                    type="button"
                    className="office-home-recent-item"
                    onClick={() => onSelectThread(run.workspaceId, run.threadId)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      width: "100%",
                      padding: "10px 12px",
                      borderRadius: 8,
                      border: "1px solid var(--border, rgba(128,128,128,0.15))",
                      background: "transparent",
                      cursor: "pointer",
                      textAlign: "left",
                    }}
                  >
                    <span className="office-home-recent-icon" aria-hidden style={{ opacity: 0.6 }}>
                      <MessageSquare size={16} />
                    </span>
                    <span className="office-home-recent-message" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
                      {run.message || run.projectName}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="office-home-recent-empty" style={{ opacity: 0.5, fontSize: 13 }}>
              {t("office.home.noRecentSessions")}
            </p>
          )}
        </section>
      </div>
    </div>
  );
}
