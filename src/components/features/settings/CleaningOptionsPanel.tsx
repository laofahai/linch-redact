import { useState } from "react"
import { ChevronDown, ChevronRight, Settings2 } from "lucide-react"
import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { useSettingsStore } from "@/stores"

export function CleaningOptionsPanel() {
  const [expanded, setExpanded] = useState(false)
  const settings = useSettingsStore((s) => s.settings)
  const toggleCleaning = useSettingsStore((s) => s.toggleCleaning)

  const cleaningOptions = [
    { key: "documentInfo" as const, label: "文档信息" },
    { key: "xmpMetadata" as const, label: "XMP 元数据" },
    { key: "annotations" as const, label: "注释" },
    { key: "forms" as const, label: "表单" },
    { key: "attachments" as const, label: "附件" },
    { key: "javascript" as const, label: "JavaScript" },
  ]

  const enabledCount = cleaningOptions.filter((opt) => settings.cleaning[opt.key]).length

  return (
    <div className="space-y-2">
      <div
        className="flex items-center justify-between cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
          <Settings2 className="h-3 w-3" />
          高级选项
          {expanded ? (
            <ChevronDown className="h-3 w-3 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-3 w-3 text-muted-foreground" />
          )}
        </h3>
        {enabledCount > 0 && (
          <span className="text-xs text-muted-foreground">{enabledCount} 项</span>
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
                {option.label}
              </Label>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
