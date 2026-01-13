import { useState } from "react"
import { FolderOpen, Play, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useFileStore, useSettingsStore, useOcrStore, useEditorStore } from "@/stores"
import { open } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"
import { processPdfs } from "@/lib/tauri"

export function Footer() {
  const files = useFileStore((s) => s.files)
  const settings = useSettingsStore((s) => s.settings)
  const setOutputDirectory = useSettingsStore((s) => s.setOutputDirectory)
  const engineStatus = useOcrStore((s) => s.engineStatus)
  const currentEngine = useOcrStore((s) => s.currentEngine)
  const openOcrDialog = useOcrStore((s) => s.openDialog)

  const isOcrReady =
    currentEngine === "paddle"
      ? (engineStatus?.paddle.installed ?? false)
      : (engineStatus?.tesseract.installed ?? false)
  const getMasksByPage = useEditorStore((s) => s.getMasksByPage)
  const [processing, setProcessing] = useState(false)

  const hasFiles = files.length > 0
  const outputDir = settings.output.directory || "请选择输出目录"

  const handleSelectOutput = async () => {
    const selected = await open({
      directory: true,
      title: "选择输出目录",
    })
    if (selected) {
      setOutputDirectory(selected as string)
    }
  }

  const handleStartProcessing = async () => {
    const needsOcr = settings.mode === "searchable"

    if (needsOcr && !isOcrReady) {
      openOcrDialog()
      return
    }

    if (!settings.output.directory) {
      toast.error("请先选择输出目录")
      return
    }

    setProcessing(true)

    try {
      const request = {
        files: files.map((file) => {
          const fileMasks = getMasksByPage(file.id)
          return {
            path: file.path,
            pages: file.pages.map((p) => ({
              index: p.index,
              action: p.action,
            })),
            masks_by_page: Object.fromEntries(
              Object.entries(fileMasks).map(([pageIdx, masks]) => [
                parseInt(pageIdx),
                masks.map((m: { x: number; y: number; width: number; height: number }) => ({
                  x: m.x,
                  y: m.y,
                  width: m.width,
                  height: m.height,
                })),
              ])
            ),
          }
        }),
        output_directory: settings.output.directory,
        prefix: "redacted_",
        mode: settings.redactionMode,
        cleaning: settings.cleaning,
      }

      console.log("[DEBUG] Processing request:", JSON.stringify(request, null, 2))

      const result = await processPdfs(request)

      if (result.success) {
        toast.success(`处理完成，共处理 ${result.processed_files.length} 个文件`)
      } else {
        // 显示详细的错误信息给用户
        const errorCount = result.errors.length
        if (errorCount === 1) {
          toast.error(result.errors[0])
        } else {
          toast.error(`处理失败，${errorCount} 个文件出错`)
          // 逐个显示错误详情
          result.errors.forEach((err) => {
            toast.error(err, { duration: 8000 })
          })
        }
        result.errors.forEach((err) => console.error(err))
      }
    } catch (e) {
      toast.error(`处理失败: ${e}`)
      console.error(e)
    } finally {
      setProcessing(false)
    }
  }

  return (
    <footer className="flex h-14 shrink-0 items-center justify-between border-t bg-card px-4">
      <button
        onClick={handleSelectOutput}
        className="flex items-center gap-2 rounded-md px-2 py-1 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      >
        <FolderOpen className="h-4 w-4" />
        <span className="max-w-[300px] truncate">{outputDir}</span>
      </button>

      <div className="flex items-center gap-2">
        <Button size="sm" disabled={!hasFiles || processing} onClick={handleStartProcessing}>
          {processing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
          {processing ? "处理中..." : "开始处理"}
        </Button>
      </div>
    </footer>
  )
}
