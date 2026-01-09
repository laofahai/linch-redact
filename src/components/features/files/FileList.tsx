import { FilePlus, X, FileText } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useFileStore } from "@/stores"
import { open } from "@tauri-apps/plugin-dialog"
import { cn } from "@/lib/utils"

interface FileListProps {
  hideTitle?: boolean
}

export function FileList({ hideTitle = false }: FileListProps) {
  const files = useFileStore((s) => s.files)
  const selectedFileId = useFileStore((s) => s.selectedFileId)
  const addFiles = useFileStore((s) => s.addFiles)
  const removeFile = useFileStore((s) => s.removeFile)
  const selectFile = useFileStore((s) => s.selectFile)

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
    <div className="space-y-2">
      {/* 标题和操作按钮 */}
      {!hideTitle && (
        <div className="flex items-center justify-between">
          <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            文件列表
          </h3>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={handleAddFile}
          >
            <FilePlus className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}

      {/* 文件列表 */}
      {files.length === 0 ? (
        <div
          className="cursor-pointer rounded-md border border-dashed p-4 text-center transition-colors hover:border-primary hover:bg-muted/50"
          onClick={handleAddFile}
        >
          <p className="text-xs text-muted-foreground">
            点击选择或拖入 PDF 文件
          </p>
        </div>
      ) : (
        <div className="space-y-1">
          {files.map((file) => (
            <div
              key={file.id}
              className={cn(
                "group flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
                selectedFileId === file.id
                  ? "bg-primary/15 text-foreground ring-1 ring-primary/30 ring-inset"
                  : "hover:bg-muted"
              )}
              onClick={() => selectFile(file.id)}
            >
              <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
              <span className="flex-1 truncate">{file.name}</span>
              <Button
                variant="ghost"
                size="icon"
                className="h-5 w-5 opacity-0 transition-opacity group-hover:opacity-100"
                onClick={(e) => {
                  e.stopPropagation()
                  removeFile(file.id)
                }}
              >
                <X className="h-3 w-3" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
