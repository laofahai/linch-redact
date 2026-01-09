import { useRef, useState } from "react"
import { X } from "lucide-react"
import type { Mask } from "@/types"

type ResizeHandle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw"

interface MaskOverlayProps {
  mask: Mask
  isSelected: boolean
  onSelect: () => void
  onDelete: () => void
  onResize: (newBounds: Partial<Mask>) => void
}

export function MaskOverlay({
  mask,
  isSelected,
  onSelect,
  onDelete,
  onResize,
}: MaskOverlayProps) {
  const [dragging, setDragging] = useState(false)
  const startPosRef = useRef<{ x: number; y: number; mask: Mask } | null>(null)

  const handlePointerDown = (e: React.PointerEvent) => {
    e.stopPropagation()
    e.preventDefault()

    if (!isSelected) {
      onSelect()
      return
    }

    // 已选中状态，开始拖动
    setDragging(true)
    startPosRef.current = {
      x: e.clientX,
      y: e.clientY,
      mask: { ...mask },
    }
    ;(e.target as HTMLElement).setPointerCapture(e.pointerId)

    const handlePointerMove = (moveEvent: PointerEvent) => {
      if (!startPosRef.current) return

      const parentRect = (e.target as HTMLElement).closest(".mask-container")?.getBoundingClientRect()
      if (!parentRect) return

      const deltaX = (moveEvent.clientX - startPosRef.current.x) / parentRect.width
      const deltaY = (moveEvent.clientY - startPosRef.current.y) / parentRect.height

      const { mask: startMask } = startPosRef.current

      // 计算新位置，确保不超出边界
      let newX = Math.max(0, Math.min(1 - startMask.width, startMask.x + deltaX))
      let newY = Math.max(0, Math.min(1 - startMask.height, startMask.y + deltaY))

      onResize({ x: newX, y: newY })
    }

    const handlePointerUp = () => {
      setDragging(false)
      startPosRef.current = null
      document.removeEventListener("pointermove", handlePointerMove)
      document.removeEventListener("pointerup", handlePointerUp)
    }

    document.addEventListener("pointermove", handlePointerMove)
    document.addEventListener("pointerup", handlePointerUp)
  }

  const handleResizeStart = (e: React.PointerEvent, handle: ResizeHandle) => {
    e.stopPropagation()
    e.preventDefault()
    startPosRef.current = {
      x: e.clientX,
      y: e.clientY,
      mask: { ...mask },
    }
    ;(e.target as HTMLElement).setPointerCapture(e.pointerId)

    const handlePointerMove = (moveEvent: PointerEvent) => {
      if (!startPosRef.current) return

      const parentRect = (e.target as HTMLElement).closest(".mask-container")?.getBoundingClientRect()
      if (!parentRect) return

      const deltaX = (moveEvent.clientX - startPosRef.current.x) / parentRect.width
      const deltaY = (moveEvent.clientY - startPosRef.current.y) / parentRect.height

      const { mask: startMask } = startPosRef.current
      let newX = startMask.x
      let newY = startMask.y
      let newWidth = startMask.width
      let newHeight = startMask.height

      // 根据手柄方向调整
      if (handle.includes("w")) {
        newX = Math.max(0, Math.min(startMask.x + startMask.width - 0.02, startMask.x + deltaX))
        newWidth = startMask.width - (newX - startMask.x)
      }
      if (handle.includes("e")) {
        newWidth = Math.max(0.02, Math.min(1 - startMask.x, startMask.width + deltaX))
      }
      if (handle.includes("n")) {
        newY = Math.max(0, Math.min(startMask.y + startMask.height - 0.02, startMask.y + deltaY))
        newHeight = startMask.height - (newY - startMask.y)
      }
      if (handle.includes("s")) {
        newHeight = Math.max(0.02, Math.min(1 - startMask.y, startMask.height + deltaY))
      }

      onResize({ x: newX, y: newY, width: newWidth, height: newHeight })
    }

    const handlePointerUp = () => {
      startPosRef.current = null
      document.removeEventListener("pointermove", handlePointerMove)
      document.removeEventListener("pointerup", handlePointerUp)
    }

    document.addEventListener("pointermove", handlePointerMove)
    document.addEventListener("pointerup", handlePointerUp)
  }

  const handleDeleteClick = (e: React.PointerEvent) => {
    e.stopPropagation()
    e.preventDefault()
    console.log("[MaskOverlay] Delete clicked for mask:", mask.id)
    onDelete()
  }

  // 手柄位置配置
  const handles: { handle: ResizeHandle; style: React.CSSProperties; cursor: string }[] = [
    { handle: "nw", style: { top: -3, left: -3 }, cursor: "nwse-resize" },
    { handle: "n", style: { top: -3, left: "50%", transform: "translateX(-50%)" }, cursor: "ns-resize" },
    { handle: "ne", style: { top: -3, right: -3 }, cursor: "nesw-resize" },
    { handle: "w", style: { top: "50%", left: -3, transform: "translateY(-50%)" }, cursor: "ew-resize" },
    { handle: "e", style: { top: "50%", right: -3, transform: "translateY(-50%)" }, cursor: "ew-resize" },
    { handle: "sw", style: { bottom: -3, left: -3 }, cursor: "nesw-resize" },
    { handle: "s", style: { bottom: -3, left: "50%", transform: "translateX(-50%)" }, cursor: "ns-resize" },
    { handle: "se", style: { bottom: -3, right: -3 }, cursor: "nwse-resize" },
  ]

  return (
    <div
      className="absolute pointer-events-auto"
      style={{
        left: `${mask.x * 100}%`,
        top: `${mask.y * 100}%`,
        width: `${mask.width * 100}%`,
        height: `${mask.height * 100}%`,
        cursor: dragging ? "grabbing" : isSelected ? "grab" : "pointer",
      }}
      onPointerDown={handlePointerDown}
    >
      {/* 黑色遮盖框 */}
      <div
        className="absolute inset-0 bg-black/80"
        style={{
          outline: isSelected ? "1px solid #3b82f6" : "none",
          outlineOffset: "1px",
        }}
      />

      {/* 选中时显示调整手柄和删除按钮 */}
      {isSelected && (
        <>
          {/* 删除按钮 */}
          <div
            className="absolute -top-2.5 -right-2.5 z-20 flex h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white shadow cursor-pointer hover:bg-red-600"
            onPointerDown={handleDeleteClick}
          >
            <X className="h-3 w-3" />
          </div>

          {/* 8 个调整手柄 */}
          {handles.map(({ handle, style, cursor }) => (
            <div
              key={handle}
              className="absolute h-1.5 w-1.5 rounded-full bg-blue-500 border border-white shadow-sm z-10"
              style={{ ...style, cursor }}
              onPointerDown={(e) => handleResizeStart(e, handle)}
            />
          ))}
        </>
      )}
    </div>
  )
}
