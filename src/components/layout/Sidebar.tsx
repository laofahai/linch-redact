import { FilePlus, Files, Layers, Settings } from "lucide-react"
import { FileList } from "@/components/features/files/FileList"
import { PageList } from "@/components/features/pages/PageList"
import { useFileStore, useSettingsDialogStore } from "@/stores"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { Button } from "@/components/ui/button"
import { open } from "@tauri-apps/plugin-dialog"

export function Sidebar() {
  const files = useFileStore((s) => s.files)
  const selectedFileId = useFileStore((s) => s.selectedFileId)
  const addFiles = useFileStore((s) => s.addFiles)
  const openSettingsDialog = useSettingsDialogStore((s) => s.openDialog)
  const hasSelectedFile = !!selectedFileId

  const selectedFile = files.find((f) => f.id === selectedFileId)
  const pageCount = selectedFile?.pages.length ?? 0

  const handleAddFile = async () => {
    const selected = await open({
      multiple: true,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
      title: "选择 PDF 文件",
    })
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected]
      addFiles(paths)
    }
  }

  return (
    <aside className="flex w-80 shrink-0 flex-col border-r bg-card overflow-hidden">
      <div className="flex-1 min-h-0 flex flex-col p-3 pb-0">
        <div className="flex items-center justify-between mb-2 shrink-0">
          <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
            <Files className="h-3 w-3" />
            文件列表
          </h3>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleAddFile}>
            <FilePlus className="h-3.5 w-3.5" />
          </Button>
        </div>
        <ScrollArea className="flex-1 file-list-scroll">
          <FileList hideTitle />
        </ScrollArea>
      </div>

      {hasSelectedFile && (
        <>
          <Separator />

          <div className="flex-1 min-h-0 flex flex-col p-3 py-2">
            <div className="flex items-center justify-between mb-2 shrink-0">
              <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
                <Layers className="h-3 w-3" />
                页面列表
              </h3>
              <span className="text-xs text-muted-foreground">{pageCount} 页</span>
            </div>
            <ScrollArea className="flex-1">
              <PageList hideTitle />
            </ScrollArea>
          </div>
        </>
      )}

      <Separator />
      <div className="shrink-0 p-2">
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-start gap-2 text-muted-foreground hover:text-foreground"
          onClick={openSettingsDialog}
        >
          <Settings className="h-4 w-4" />
          设置
        </Button>
      </div>
    </aside>
  )
}
