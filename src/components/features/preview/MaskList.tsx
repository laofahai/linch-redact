import { useTranslation } from "react-i18next"
import { X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useEditorStore } from "@/stores"

export function MaskList() {
  const { t } = useTranslation()
  const currentPage = useEditorStore((s) => s.currentPage)
  const currentFileId = useEditorStore((s) => s.currentFileId)
  const masksByFile = useEditorStore((s) => s.masksByFile)
  const removeMask = useEditorStore((s) => s.removeMask)
  const clearPageMasks = useEditorStore((s) => s.clearPageMasks)

  // 直接从 state 计算 masks，确保状态变化时组件重新渲染
  const masks = currentFileId ? (masksByFile[currentFileId]?.[currentPage] ?? []) : []

  if (masks.length === 0) {
    return <div className="text-sm text-muted-foreground">{t("preview.noMasks")}</div>
  }

  return (
    <div className="flex items-center gap-2">
      <span className="text-sm text-muted-foreground">
        {t("preview.maskCount", { count: masks.length })}
      </span>
      <div className="flex items-center gap-1">
        {masks.map((mask, index) => (
          <Button
            key={mask.id}
            variant="outline"
            size="sm"
            className="h-6 gap-1 px-2 text-xs"
            onClick={() => removeMask(currentPage, mask.id)}
          >
            #{index + 1}
            <X className="h-3 w-3" />
          </Button>
        ))}
      </div>
      {masks.length > 1 && (
        <Button
          variant="ghost"
          size="sm"
          className="h-6 text-xs text-destructive hover:text-destructive"
          onClick={() => clearPageMasks(currentPage)}
        >
          {t("common.clearAll")}
        </Button>
      )}
    </div>
  )
}
