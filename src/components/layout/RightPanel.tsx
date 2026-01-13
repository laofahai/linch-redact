import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { DetectionPanel } from "@/components/features/detection/DetectionPanel"
import { ModeSelector } from "@/components/features/settings/ModeSelector"
import { CleaningOptionsPanel } from "@/components/features/settings/CleaningOptionsPanel"
import { useFileStore } from "@/stores"

export function RightPanel() {
  const hasSelectedFile = !!useFileStore((s) => s.selectedFileId)

  return (
    <aside className="flex w-80 shrink-0 flex-col border-l bg-card">
      <div className="flex-1 min-h-0 flex flex-col p-3 pb-0">
        <ScrollArea className="flex-1">
          {hasSelectedFile ? (
            <div className="space-y-3 pb-3">
              <ModeSelector />
              <Separator />
              <DetectionPanel />
              <Separator />
              <CleaningOptionsPanel />
            </div>
          ) : (
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
              添加 PDF 后可配置检测与选项。
            </div>
          )}
        </ScrollArea>
      </div>
    </aside>
  )
}
