import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";

export type ExperimentalToggleRowProps = {
  title: string;
  description: string;
  markerLabel: string;
  markerTone: "success" | "info" | "warning";
  markerDetail: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  highlighted?: boolean;
};

export function ExperimentalToggleRow({
  title,
  description,
  markerLabel,
  markerTone,
  markerDetail,
  checked,
  onCheckedChange,
  highlighted = false,
}: ExperimentalToggleRowProps) {
  return (
    <div className={`settings-toggle-row${highlighted ? " is-highlighted" : ""}`}>
      <div>
        <div className="settings-toggle-title flex items-center gap-2">
          <span>{title}</span>
          <Badge variant={markerTone} size="sm">
            {markerLabel}
          </Badge>
        </div>
        <div className="settings-toggle-subtitle">
          {description}
        </div>
        <div className="settings-help">{markerDetail}</div>
      </div>
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </div>
  );
}
