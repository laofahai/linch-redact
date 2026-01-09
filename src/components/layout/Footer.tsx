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

  // 判断当前引擎是否可用
  const isOcrReady = currentEngine === "paddle"
    ? engineStatus?.paddle.installed ?? false
    : engineStatus?.tesseract.installed ?? false
  const masksByPage = useEditorStore((s) => s.masksByPage)
  const [processing, setProcessing] = useState(false)

  const hasFiles = files.length > 0
  const outputDir = settings.output.directory || "未选择输出目录"

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
    // 只有 "searchable" 模式（OCR 识别）才需要 OCR 组件
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
      // 构建处理请求
      const request = {
        files: files.map((file) => ({
          path: file.path,
          pages: file.pages.map((p) => ({
            index: p.index,
            action: p.action,
          })),
          masks_by_page: Object.fromEntries(
            Object.entries(masksByPage).map(([pageIdx, masks]) => [
              parseInt(pageIdx),
              masks.map((m: { x: number; y: number; width: number; height: number }) => ({
                x: m.x,
                y: m.y,
                width: m.width,
                height: m.height,
              })),
            ])
          ),
        })),
        output_directory: settings.output.directory,
        suffix: settings.output.suffix || "_redacted",
        mode: settings.redactionMode,
        cleaning: settings.cleaning,
      }

      // 调试：打印发送的坐标
      console.log("[DEBUG] Processing request:", JSON.stringify(request, null, 2))

      const result = await processPdfs(request)

      if (result.success) {
        toast.success(`处理完成！已处理 ${result.processed_files.length} 个文件`)
      } else {
        toast.error(`处理完成，但有 ${result.errors.length} 个错误`)
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
      {/* 左侧：输出目录 */}
      <button
        onClick={handleSelectOutput}
        className="flex items-center gap-2 rounded-md px-2 py-1 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      >
        <FolderOpen className="h-4 w-4" />
        <span className="max-w-[300px] truncate">{outputDir}</span>
      </button>

      {/* 右侧：操作按钮 */}
      <div className="flex items-center gap-2">
        <Button size="sm" disabled={!hasFiles || processing} onClick={handleStartProcessing}>
          {processing ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Play className="h-4 w-4" />
          )}
          {processing ? "处理中..." : "开始处理"}
        </Button>
      </div>
    </footer>
  )
}
