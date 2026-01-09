import { X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useEditorStore } from "@/stores"

export function MaskList() {
  const currentPage = useEditorStore((s) => s.currentPage)
  const masksByPage = useEditorStore((s) => s.masksByPage)
  const removeMask = useEditorStore((s) => s.removeMask)
  const clearPageMasks = useEditorStore((s) => s.clearPageMasks)

  const masks = masksByPage[currentPage] ?? []

  if (masks.length === 0) {
    return (
      <div className="text-sm text-muted-foreground">
        当前页无遮盖标记
      </div>
    )
  }

  return (
    <div className="flex items-center gap-2">
      <span className="text-sm text-muted-foreground">
        遮盖区域: {masks.length}
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
          清除全部
        </Button>
      )}
    </div>
  )
}
