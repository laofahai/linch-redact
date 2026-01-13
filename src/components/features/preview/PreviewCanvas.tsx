import { useRef, useCallback, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { useEditorStore, useFileStore } from "@/stores"
import { loadPdf, renderPageToCanvas } from "@/lib/pdf"
import { MaskOverlay } from "./MaskOverlay"
import type { PdfFile } from "@/types"

interface PreviewCanvasProps {
  file: PdfFile
}

export function PreviewCanvas({ file }: PreviewCanvasProps) {
  const { t } = useTranslation()
  const wrapperRef = useRef<HTMLDivElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const currentPage = useEditorStore((s) => s.currentPage)
  const currentFileId = useEditorStore((s) => s.currentFileId)
  const masksByFile = useEditorStore((s) => s.masksByFile)
  const zoom = useEditorStore((s) => s.zoom)
  const setZoom = useEditorStore((s) => s.setZoom)
  const drawing = useEditorStore((s) => s.drawing)
  const selectedMaskId = useEditorStore((s) => s.selectedMaskId)
  const startDrawing = useEditorStore((s) => s.startDrawing)
  const updateDrawing = useEditorStore((s) => s.updateDrawing)
  const finishDrawing = useEditorStore((s) => s.finishDrawing)
  const selectMask = useEditorStore((s) => s.selectMask)
  const resizeMask = useEditorStore((s) => s.resizeMask)
  const removeMask = useEditorStore((s) => s.removeMask)
  const deleteSelectedMask = useEditorStore((s) => s.deleteSelectedMask)
  const setPageCount = useFileStore((s) => s.setPageCount)

  // 直接从 state 计算 masks，确保状态变化时组件重新渲染
  const masks = currentFileId ? (masksByFile[currentFileId]?.[currentPage] ?? []) : []

  // Ctrl + 滚轮缩放（在外层 wrapper 上监听）
  useEffect(() => {
    const wrapper = wrapperRef.current
    if (!wrapper) return

    const handleWheel = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault()
        e.stopPropagation()
        const delta = e.deltaY > 0 ? -0.1 : 0.1
        setZoom(zoom + delta)
      }
    }

    wrapper.addEventListener("wheel", handleWheel, { passive: false })
    return () => wrapper.removeEventListener("wheel", handleWheel)
  }, [zoom, setZoom])

  // 键盘事件处理（Delete/Backspace 删除选中的 mask）
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.key === "Delete" || e.key === "Backspace") && selectedMaskId) {
        e.preventDefault()
        deleteSelectedMask()
      }
      // Escape 取消选中
      if (e.key === "Escape" && selectedMaskId) {
        selectMask(null)
      }
    }

    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [selectedMaskId, deleteSelectedMask, selectMask])

  // 加载并渲染 PDF 页面（只在页面切换时渲染，zoom 用 CSS 处理）
  useEffect(() => {
    let cancelled = false

    async function loadAndRender() {
      if (!file.path || !canvasRef.current) return

      setLoading(true)
      setError(null)

      try {
        const pdf = await loadPdf(file.path)

        // 更新页数
        if (file.pageCount !== pdf.numPages) {
          setPageCount(file.id, pdf.numPages)
        }

        if (cancelled) return

        // 渲染当前页 (页码从 1 开始)
        const pageNum = Math.min(currentPage + 1, pdf.numPages)
        const page = await pdf.getPage(pageNum)

        if (cancelled) return

        await renderPageToCanvas(page, canvasRef.current)
        setLoading(false)
        setError(null)
      } catch (e: unknown) {
        const err = e as { name?: string }
        if (!cancelled && err?.name !== "RenderingCancelledException") {
          console.error("Failed to load PDF:", e)
          setError(t("preview.loadFailed"))
          setLoading(false)
        }
      }
    }

    loadAndRender()

    return () => {
      cancelled = true
    }
  }, [file.path, file.id, file.pageCount, currentPage, setPageCount])

  const getRelativePosition = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      const rect = containerRef.current?.getBoundingClientRect()
      const canvasRect = canvasRef.current?.getBoundingClientRect()
      if (!rect || !canvasRect) return { x: 0, y: 0 }

      // 调试：打印各种坐标信息
      console.log("[DEBUG] getRelativePosition:", {
        clientX: e.clientX,
        clientY: e.clientY,
        containerRect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
        canvasRect: {
          left: canvasRect.left,
          top: canvasRect.top,
          width: canvasRect.width,
          height: canvasRect.height,
        },
        zoom,
      })

      // 使用 canvas 的位置和尺寸计算相对坐标
      return {
        x: (e.clientX - canvasRect.left) / canvasRect.width,
        y: (e.clientY - canvasRect.top) / canvasRect.height,
      }
    },
    [zoom]
  )

  const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return
    // 点击空白区域时取消选中
    selectMask(null)
    const pos = getRelativePosition(e)
    startDrawing(pos.x, pos.y)
    ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
  }

  const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!drawing) return
    const pos = getRelativePosition(e)
    updateDrawing(pos.x, pos.y)
  }

  const handlePointerUp = () => {
    if (drawing) {
      // 调试：记录绘制结束时的坐标
      const x = Math.min(drawing.startX, drawing.currentX)
      const y = Math.min(drawing.startY, drawing.currentY)
      const w = Math.abs(drawing.currentX - drawing.startX)
      const h = Math.abs(drawing.currentY - drawing.startY)
      const rect = containerRef.current?.getBoundingClientRect()
      console.log("[DEBUG] Drawing finished:", {
        relativeCoords: { x, y, width: w, height: h },
        containerRect: rect
          ? { width: rect.width, height: rect.height, top: rect.top, left: rect.left }
          : null,
        zoom,
      })
      finishDrawing()
    }
  }

  const getDrawingStyle = () => {
    if (!drawing) return {}
    const x = Math.min(drawing.startX, drawing.currentX) * 100
    const y = Math.min(drawing.startY, drawing.currentY) * 100
    const w = Math.abs(drawing.currentX - drawing.startX) * 100
    const h = Math.abs(drawing.currentY - drawing.startY) * 100
    return {
      left: `${x}%`,
      top: `${y}%`,
      width: `${w}%`,
      height: `${h}%`,
    }
  }

  return (
    <div ref={wrapperRef} className="flex flex-col items-center">
      {/* 预览画布 - 使用 CSS transform 处理缩放 */}
      <div
        ref={containerRef}
        className="relative cursor-crosshair select-none overflow-hidden rounded-lg border bg-white shadow-md"
        style={{
          transform: `scale(${zoom})`,
          transformOrigin: "center top",
        }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
      >
        {/* PDF Canvas */}
        <canvas ref={canvasRef} className="block" />

        {/* 加载状态 */}
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-white/80">
            <div className="text-sm text-muted-foreground">{t("common.loading")}</div>
          </div>
        )}

        {/* 错误状态 */}
        {error && !loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-white">
            <div className="text-center">
              <p className="text-sm text-destructive">{error}</p>
              <p className="text-xs text-muted-foreground mt-1">{file.name}</p>
            </div>
          </div>
        )}

        {/* 已保存的遮盖框 - 使用 MaskOverlay 组件 */}
        <div className="mask-container absolute inset-0 pointer-events-none">
          {masks.map((mask) => (
            <MaskOverlay
              key={mask.id}
              mask={mask}
              isSelected={selectedMaskId === mask.id}
              onSelect={() => selectMask(mask.id)}
              onDelete={() => removeMask(currentPage, mask.id)}
              onResize={(newBounds) => resizeMask(currentPage, mask.id, newBounds)}
            />
          ))}
        </div>

        {/* 正在绘制的遮盖框 */}
        {drawing && (
          <div
            className="absolute border-2 border-dashed border-destructive bg-destructive/20 pointer-events-none"
            style={getDrawingStyle()}
          />
        )}
      </div>
    </div>
  )
}
