import { useState } from "react"
import { useTranslation } from "react-i18next"
import { nanoid } from "nanoid"
import { invoke } from "@tauri-apps/api/core"
import {
  Search,
  CreditCard,
  Phone,
  Mail,
  User,
  Plus,
  Check,
  ChevronDown,
  ChevronRight,
  AlertCircle,
  ShieldAlert,
  FileText,
  Files,
  Settings,
} from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { cn } from "@/lib/utils"
import { useDetectionRulesStore, useFileStore, useEditorStore, useOcrStore } from "@/stores"
import { Trash2 } from "lucide-react"
import type { Rule, DetectionHit, Mask, PdfAnalysis } from "@/types"

const ruleIcons: Record<string, React.ComponentType<{ className?: string }>> = {
  id_card_cn: User,
  phone_cn: Phone,
  email: Mail,
  bank_card: CreditCard,
}

async function analyzePdf(path: string): Promise<PdfAnalysis> {
  return await invoke<PdfAnalysis>("analyze_pdf", { pdfPath: path })
}

async function detectSensitiveContent(
  path: string,
  rules: Rule[],
  useOcr: boolean = false,
  pageIndices?: number[]
): Promise<DetectionHit[]> {
  return await invoke<DetectionHit[]>("detect_sensitive_content", {
    pdfPath: path,
    rules,
    useOcr,
    pageIndices,
  })
}

export function DetectionPanel() {
  const { t } = useTranslation()
  const selectedFile = useFileStore((s) => s.getSelectedFile())
  const currentPage = useEditorStore((s) => s.currentPage)
  const setCurrentPage = useEditorStore((s) => s.setCurrentPage)
  const addMask = useEditorStore((s) => s.addMask)
  const removeMask = useEditorStore((s) => s.removeMask)
  const engineStatus = useOcrStore((s) => s.engineStatus)
  const currentEngine = useOcrStore((s) => s.currentEngine)
  const openOcrDialog = useOcrStore((s) => s.openDialog)

  const ocrReady =
    currentEngine === "paddle"
      ? (engineStatus?.paddle.installed ?? false)
      : (engineStatus?.tesseract.installed ?? false)

  const rules = useDetectionRulesStore((s) => s.rules)
  const toggleRule = useDetectionRulesStore((s) => s.toggleRule)
  const getHits = useDetectionRulesStore((s) => s.getHits)
  const getAddedHits = useDetectionRulesStore((s) => s.getAddedHits)
  const setHits = useDetectionRulesStore((s) => s.setHits)
  const markHitAdded = useDetectionRulesStore((s) => s.markHitAdded)
  const unmarkHitAdded = useDetectionRulesStore((s) => s.unmarkHitAdded)
  const markAllHitsAdded = useDetectionRulesStore((s) => s.markAllHitsAdded)

  const [scanning, setScanning] = useState(false)
  const [expanded, setExpanded] = useState(true)
  const [needsOcr, setNeedsOcr] = useState(false)
  const [scanScope, setScanScope] = useState<"current" | "all">("all")

  const fileId = selectedFile?.id ?? ""
  const hits = getHits(fileId)
  const addedHits = getAddedHits(fileId)

  const runDetection = async () => {
    if (!selectedFile?.path || !fileId) return

    const enabledRules = rules.filter((r) => r.enabled)
    if (enabledRules.length === 0) {
      toast.warning(t("detection.selectAtLeastOneRule"))
      return
    }

    setScanning(true)
    setNeedsOcr(false)

    try {
      const analysis = await analyzePdf(selectedFile.path)
      const imagePageCount = analysis.pageTypes.filter((t) => t === "image_based").length

      if (imagePageCount > 0 && !ocrReady) {
        setNeedsOcr(true)
        setScanning(false)
        toast.warning(t("detection.needsOcrWarning", { count: imagePageCount }), {
          duration: 5000,
        })
        return
      }

      const useOcr = imagePageCount > 0 && ocrReady
      const pageIndices = scanScope === "current" ? [currentPage] : undefined
      const results = await detectSensitiveContent(
        selectedFile.path,
        enabledRules,
        useOcr,
        pageIndices
      )
      setHits(fileId, results)

      if (results.length === 0) {
        toast.info(t("detection.noResults"))
      } else {
        toast.success(t("detection.foundResults", { count: results.length }))
      }
    } catch (e) {
      console.error("Detection failed:", e)
      toast.error(t("detection.scanFailed"))
    } finally {
      setScanning(false)
    }
  }

  // 存储每个 hit 对应的 mask id，用于删除
  const [hitMaskIds, setHitMaskIds] = useState<Map<number, string>>(new Map())

  const addHitAsMask = (hit: DetectionHit, hitIndex: number, e: React.MouseEvent) => {
    e.stopPropagation()
    if (addedHits.has(hitIndex)) return

    const maskId = nanoid()
    const mask: Mask = {
      id: maskId,
      x: hit.bbox.x,
      y: hit.bbox.y,
      width: hit.bbox.width,
      height: hit.bbox.height,
    }
    addMask(hit.page, mask, fileId)
    markHitAdded(fileId, hitIndex)
    setHitMaskIds((prev) => new Map(prev).set(hitIndex, maskId))
    // 跳转到对应页面以显示添加的遮罩
    if (hit.page !== currentPage) {
      setCurrentPage(hit.page)
    }
    toast.success(t("detection.maskAdded"))
  }

  const removeHitMask = (hit: DetectionHit, hitIndex: number, e: React.MouseEvent) => {
    e.stopPropagation()
    const maskId = hitMaskIds.get(hitIndex)
    if (maskId) {
      removeMask(hit.page, maskId, fileId)
      setHitMaskIds((prev) => {
        const newMap = new Map(prev)
        newMap.delete(hitIndex)
        return newMap
      })
    }
    // 更新状态为未添加
    unmarkHitAdded(fileId, hitIndex)
  }

  const addAllHitsAsMasks = () => {
    let added = 0
    const newMaskIds = new Map(hitMaskIds)
    hits.forEach((hit, idx) => {
      if (addedHits.has(idx)) return
      const maskId = nanoid()
      const mask: Mask = {
        id: maskId,
        x: hit.bbox.x,
        y: hit.bbox.y,
        width: hit.bbox.width,
        height: hit.bbox.height,
      }
      addMask(hit.page, mask, fileId)
      newMaskIds.set(idx, maskId)
      added++
    })
    setHitMaskIds(newMaskIds)
    markAllHitsAdded(fileId)
    if (added > 0) {
      toast.success(t("detection.masksAdded", { count: added }))
    }
  }

  const handleHitClick = (hit: DetectionHit) => {
    // 只跳转页面，不添加遮罩
    if (hit.page !== currentPage) {
      setCurrentPage(hit.page)
    }
  }

  if (!selectedFile) {
    return null
  }

  // 按页分组统计
  const currentPageHitsCount = hits.filter((h) => h.page === currentPage).length

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between shrink-0">
        <div
          className="flex items-center gap-1.5 cursor-pointer flex-1"
          onClick={() => setExpanded(!expanded)}
        >
          <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
            <ShieldAlert className="h-3 w-3" />
            {t("detection.title")}
          </h3>
        </div>
        <button
          onClick={openOcrDialog}
          className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] bg-muted hover:bg-muted/80 transition-colors"
          title="配置 OCR"
        >
          <span
            className={`w-1.5 h-1.5 rounded-full ${ocrReady ? "bg-green-500" : "bg-red-500"}`}
          />
          <span className="text-muted-foreground">
            {currentEngine === "paddle" ? "Paddle" : "Tesseract"}
          </span>
          <Settings className="h-2.5 w-2.5 text-muted-foreground" />
        </button>
        {expanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
      </div>

      {expanded && (
        <div className="flex-1 min-h-0 flex flex-col mt-3 space-y-3">
          <div className="flex flex-wrap gap-1.5 shrink-0">
            {rules.map((rule) => {
              const Icon = ruleIcons[rule.id] || Search
              return (
                <button
                  key={rule.id}
                  onClick={() => toggleRule(rule.id)}
                  className={cn(
                    "inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium transition-all",
                    "border cursor-pointer select-none",
                    rule.enabled
                      ? "bg-primary/10 border-primary/30 text-primary hover:bg-primary/20"
                      : "bg-muted/50 border-transparent text-muted-foreground hover:bg-muted"
                  )}
                >
                  <Icon className="h-3 w-3" />
                  {rule.name}
                </button>
              )
            })}
          </div>

          <div className="flex items-center gap-1.5 shrink-0">
            <Button
              variant="outline"
              size="sm"
              className="flex-1"
              onClick={runDetection}
              disabled={scanning}
            >
              <Search className={`h-3.5 w-3.5 mr-1.5 ${scanning ? "animate-pulse" : ""}`} />
              {scanning ? t("common.scanning") : t("detection.scan")}
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm" className="px-2">
                  {scanScope === "current" ? (
                    <FileText className="h-3.5 w-3.5" />
                  ) : (
                    <Files className="h-3.5 w-3.5" />
                  )}
                  <ChevronDown className="h-3 w-3 ml-1" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => setScanScope("current")}>
                  <FileText className="h-3.5 w-3.5 mr-2" />
                  {t("common.currentPage")}
                  {scanScope === "current" && <Check className="h-3 w-3 ml-auto" />}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => setScanScope("all")}>
                  <Files className="h-3.5 w-3.5 mr-2" />
                  {t("common.allPages")}
                  {scanScope === "all" && <Check className="h-3 w-3 ml-auto" />}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {needsOcr && (
            <div className="rounded-md bg-amber-50 dark:bg-amber-950/30 p-2 space-y-2 shrink-0">
              <div className="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-400">
                <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
                <span>{t("detection.needsOcr")}</span>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="w-full h-7 text-xs"
                onClick={openOcrDialog}
              >
                {t("detection.configureOcr")}
              </Button>
            </div>
          )}

          {hits.length > 0 && (
            <div className="flex-1 min-h-0 flex flex-col">
              <div className="flex items-center justify-between shrink-0 mb-2">
                <p className="text-sm text-muted-foreground">
                  {t("detection.currentPageCount", {
                    current: currentPageHitsCount,
                    total: hits.length,
                  })}
                </p>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 px-2 text-sm"
                  onClick={addAllHitsAsMasks}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  {t("common.addAll")}
                </Button>
              </div>

              <ScrollArea className="flex-1">
                <div className="space-y-1 pr-2">
                  {hits.map((hit, idx) => {
                    const isAdded = addedHits.has(idx)
                    const isCurrentPage = hit.page === currentPage
                    return (
                      <div
                        key={idx}
                        onClick={() => handleHitClick(hit)}
                        className={cn(
                          "flex items-center justify-between rounded-md px-2 py-1.5 text-sm transition-colors cursor-pointer",
                          isAdded
                            ? "bg-green-50 dark:bg-green-950/30 hover:bg-green-100 dark:hover:bg-green-950/50"
                            : isCurrentPage
                              ? "bg-primary/10 hover:bg-primary/15"
                              : "bg-muted/50 hover:bg-muted"
                        )}
                      >
                        <div className="flex-1 truncate">
                          <span
                            className={cn(
                              "text-xs mr-1.5",
                              isCurrentPage ? "text-primary font-medium" : "text-muted-foreground"
                            )}
                          >
                            P{hit.page + 1}
                          </span>
                          <span className="font-medium">{hit.ruleName}</span>
                          <span className="text-muted-foreground ml-1.5">{hit.snippet}</span>
                        </div>
                        {isAdded ? (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6 shrink-0 text-destructive hover:text-destructive hover:bg-destructive/10"
                            onClick={(e) => removeHitMask(hit, idx, e)}
                            title={t("detection.removeMask")}
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        ) : (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6 shrink-0"
                            onClick={(e) => addHitAsMask(hit, idx, e)}
                            title={t("detection.addMask")}
                          >
                            <Plus className="h-4 w-4" />
                          </Button>
                        )}
                      </div>
                    )
                  })}
                </div>
              </ScrollArea>
            </div>
          )}

          {hits.length === 0 && !needsOcr && (
            <div className="flex-1 flex items-center justify-center text-xs text-muted-foreground">
              {t("detection.scanHint")}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
