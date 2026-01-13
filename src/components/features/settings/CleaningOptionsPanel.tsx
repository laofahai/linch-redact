import { useState } from "react"
import { useTranslation } from "react-i18next"
import { ChevronDown, ChevronRight, Settings2 } from "lucide-react"
import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { useSettingsStore } from "@/stores"

export function CleaningOptionsPanel() {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(false)
  const settings = useSettingsStore((s) => s.settings)
  const toggleCleaning = useSettingsStore((s) => s.toggleCleaning)

  const cleaningOptions = [
    { key: "documentInfo" as const, labelKey: "cleaning.documentInfo" },
    { key: "xmpMetadata" as const, labelKey: "cleaning.xmpMetadata" },
    { key: "annotations" as const, labelKey: "cleaning.annotations" },
    { key: "forms" as const, labelKey: "cleaning.forms" },
    { key: "attachments" as const, labelKey: "cleaning.attachments" },
    { key: "javascript" as const, labelKey: "cleaning.javascript" },
  ]

  const enabledCount = cleaningOptions.filter((opt) => settings.cleaning[opt.key]).length

  return (
    <div className={expanded ? "space-y-2" : ""}>
      <div
        className="flex items-center justify-between cursor-pointer h-7"
        onClick={() => setExpanded(!expanded)}
      >
        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
          <Settings2 className="h-3 w-3" />
          {t("cleaning.title")}
          {expanded ? (
            <ChevronDown className="h-3 w-3 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-3 w-3 text-muted-foreground" />
          )}
        </h3>
        {enabledCount > 0 && (
          <span className="text-xs text-muted-foreground">
            {enabledCount} {t("common.items")}
          </span>
        )}
      </div>

      {expanded && (
        <div className="grid grid-cols-2 gap-x-2 gap-y-2 pl-1">
          {cleaningOptions.map((option) => (
            <div key={option.key} className="flex items-center gap-2">
              <Checkbox
                id={option.key}
                checked={settings.cleaning[option.key]}
                onCheckedChange={() => toggleCleaning(option.key)}
                className="h-4 w-4"
              />
              <Label htmlFor={option.key} className="text-sm font-normal cursor-pointer">
                {t(option.labelKey)}
              </Label>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
