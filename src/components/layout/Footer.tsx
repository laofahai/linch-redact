import { useState } from "react"
import { useTranslation } from "react-i18next"
import { FolderOpen, Play, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useFileStore, useSettingsStore, useDetectionRulesStore } from "@/stores"
import { open } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"
import { applyRedaction } from "@/lib/tauri/document"

export function Footer() {
  const { t } = useTranslation()
  const documents = useFileStore((s) => s.documents)
  const settings = useSettingsStore((s) => s.settings)
  const setOutputDirectory = useSettingsStore((s) => s.setOutputDirectory)
  const rules = useDetectionRulesStore((s) => s.rules)
  const [processing, setProcessing] = useState(false)

  const hasDocuments = documents.length > 0
  const outputDir = settings.output.directory || t("processing.selectOutputDir")

  const handleSelectOutput = async () => {
    const selected = await open({
      directory: true,
      title: t("processing.selectOutputDir"),
    })
    if (selected) {
      setOutputDirectory(selected as string)
    }
  }

  const handleStartProcessing = async () => {
    if (!settings.output.directory) {
      toast.error(t("processing.selectOutputDirFirst"))
      return
    }

    const enabledRules = rules.filter((r) => r.enabled)
    if (enabledRules.length === 0) {
      toast.error(t("detection.selectAtLeastOneRule"))
      return
    }

    setProcessing(true)

    try {
      let successCount = 0
      let errorCount = 0

      for (const doc of documents) {
        if (doc.status !== "ready") continue

        // 生成输出文件名
        const fileName = doc.name
        const ext = fileName.split(".").pop() || ""
        const baseName = fileName.slice(0, -(ext.length + 1))
        const outputPath = `${settings.output.directory}/redacted_${baseName}.${ext}`

        try {
          const result = await applyRedaction(doc.path, enabledRules, outputPath)
          if (result.success) {
            successCount++
            console.log(`[OK] ${doc.name}: ${result.message}`)
          } else {
            errorCount++
            console.error(`[FAIL] ${doc.name}: ${result.message}`)
          }
        } catch (e) {
          errorCount++
          console.error(`[ERROR] ${doc.name}:`, e)
        }
      }

      if (errorCount === 0) {
        toast.success(t("processing.processComplete", { count: successCount }))
      } else {
        toast.error(t("processing.processFailedMultiple", { count: errorCount }))
      }
    } catch (e) {
      toast.error(`${t("processing.processFailed")}: ${e}`)
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
        <Button size="sm" disabled={!hasDocuments || processing} onClick={handleStartProcessing}>
          {processing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
          {processing ? t("common.processing") : t("processing.startProcessing")}
        </Button>
      </div>
    </footer>
  )
}
