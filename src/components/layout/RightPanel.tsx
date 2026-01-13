import { Separator } from "@/components/ui/separator"
import { DetectionPanel } from "@/components/features/detection/DetectionPanel"
import { ModeSelector } from "@/components/features/settings/ModeSelector"
import { CleaningOptionsPanel } from "@/components/features/settings/CleaningOptionsPanel"
import { useFileStore } from "@/stores"

export function RightPanel() {
  const hasSelectedFile = !!useFileStore((s) => s.selectedFileId)

  return (
    <aside className="flex w-80 shrink-0 flex-col border-l bg-card">
      {hasSelectedFile ? (
        <>
          {/* 顶部区域：模式选择 + 检测面板（占满剩余空间） */}
          <div className="flex-1 min-h-0 flex flex-col p-3 pb-0">
            <div className="space-y-3 shrink-0">
              <ModeSelector />
              <Separator />
            </div>
            <div className="flex-1 min-h-0 mt-3">
              <DetectionPanel />
            </div>
          </div>
          {/* 底部区域：高级选项（固定在底部，高度与中间缩放区域对齐） */}
          <div className="shrink-0 border-t px-3 py-2">
            <CleaningOptionsPanel />
          </div>
        </>
      ) : (
        <div className="flex flex-1 items-center justify-center text-xs text-muted-foreground p-3">
          添加 PDF 后可配置检测与选项。
        </div>
      )}
    </aside>
  )
}
