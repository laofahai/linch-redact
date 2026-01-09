import { useSettingsStore } from "@/stores"
import { Zap, FileText, Shield } from "lucide-react"
import { cn } from "@/lib/utils"
import type { RedactionMode } from "@/types"

interface ModeOption {
  value: RedactionMode
  label: string
  icon: React.ComponentType<{ className?: string }>
}

const modeOptions: ModeOption[] = [
  { value: "auto", label: "自动", icon: Zap },
  { value: "text_replace", label: "高清", icon: FileText },
  { value: "safe_render", label: "安全", icon: Shield },
]

export function ModeSelector() {
  const redactionMode = useSettingsStore((s) => s.settings.redactionMode)
  const setRedactionMode = useSettingsStore((s) => s.setRedactionMode)

  return (
    <div className="space-y-2">
      <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
        脱敏模式
      </h3>
      <div className="flex gap-1">
        {modeOptions.map((option) => {
          const Icon = option.icon
          const isSelected = redactionMode === option.value
          return (
            <button
              key={option.value}
              className={cn(
                "flex-1 flex items-center justify-center gap-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors",
                isSelected
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted/50 text-muted-foreground hover:bg-muted hover:text-foreground"
              )}
              onClick={() => setRedactionMode(option.value)}
            >
              <Icon className="h-3.5 w-3.5" />
              {option.label}
            </button>
          )
        })}
      </div>
    </div>
  )
}
