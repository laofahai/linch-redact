import { ZoomIn, ZoomOut } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useEditorStore } from "@/stores"

export function ZoomControls() {
  const zoom = useEditorStore((s) => s.zoom)
  const zoomIn = useEditorStore((s) => s.zoomIn)
  const zoomOut = useEditorStore((s) => s.zoomOut)
  const resetZoom = useEditorStore((s) => s.resetZoom)

  return (
    <div className="flex items-center gap-1">
      <Button
        variant="ghost"
        size="icon"
        className="h-7 w-7"
        onClick={zoomOut}
        disabled={zoom <= 0.25}
      >
        <ZoomOut className="h-4 w-4" />
      </Button>
      <button
        className="w-14 rounded px-2 py-0.5 text-center text-sm hover:bg-muted"
        onClick={resetZoom}
      >
        {Math.round(zoom * 100)}%
      </button>
      <Button
        variant="ghost"
        size="icon"
        className="h-7 w-7"
        onClick={zoomIn}
        disabled={zoom >= 3}
      >
        <ZoomIn className="h-4 w-4" />
      </Button>
    </div>
  )
}
