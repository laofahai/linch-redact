import { useState } from "react"
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { cn } from "@/lib/utils"
import { useDetectionRulesStore, useFileStore, useEditorStore, useOcrStore } from "@/stores"
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
  const selectedFile = useFileStore((s) => s.getSelectedFile())
  const currentPage = useEditorStore((s) => s.currentPage)
  const addMask = useEditorStore((s) => s.addMask)
  const engineStatus = useOcrStore((s) => s.engineStatus)
  const currentEngine = useOcrStore((s) => s.currentEngine)
  const openOcrDialog = useOcrStore((s) => s.openDialog)

  const ocrReady =
    currentEngine === "paddle"
      ? (engineStatus?.paddle.installed ?? false)
      : (engineStatus?.tesseract.installed ?? false)

  const rules = useDetectionRulesStore((s) => s.rules)
  const toggleRule = useDetectionRulesStore((s) => s.toggleRule)
  const [hits, setHits] = useState<DetectionHit[]>([])
  const [addedHits, setAddedHits] = useState<Set<number>>(new Set())
  const [scanning, setScanning] = useState(false)
  const [expanded, setExpanded] = useState(true)
  const [_hasImagePages, setHasImagePages] = useState(false)
  const [needsOcr, setNeedsOcr] = useState(false)
  const [scanScope, setScanScope] = useState<"current" | "all">("all")

  const runDetection = async () => {
    if (!selectedFile?.path) return

    const enabledRules = rules.filter((r) => r.enabled)
    if (enabledRules.length === 0) {
      toast.warning("请至少选择一个检测规则")
      return
    }

    setScanning(true)
    setHits([])
    setAddedHits(new Set())
    setNeedsOcr(false)

    try {
      const analysis = await analyzePdf(selectedFile.path)
      const imagePageCount = analysis.pageTypes.filter((t) => t === "image_based").length
      setHasImagePages(imagePageCount > 0)

      if (imagePageCount > 0 && !ocrReady) {
        setNeedsOcr(true)
        setScanning(false)
        toast.warning(`检测到 ${imagePageCount} 页为图片，需 OCR 才能识别。请先安装/配置 OCR。`, {
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
      setHits(results)

      if (results.length === 0) {
        toast.info("未发现敏感信息")
      } else {
        toast.success(`发现 ${results.length} 处敏感信息`)
      }
    } catch (e) {
      console.error("Detection failed:", e)
      toast.error("检测失败")
    } finally {
      setScanning(false)
    }
  }

  const addHitAsMask = (hit: DetectionHit, hitIndex: number) => {
    if (addedHits.has(hitIndex)) return

    const mask: Mask = {
      id: nanoid(),
      x: hit.bbox.x,
      y: hit.bbox.y,
      width: hit.bbox.width,
      height: hit.bbox.height,
    }
    addMask(hit.page, mask)
    setAddedHits((prev) => new Set(prev).add(hitIndex))
    toast.success("已添加遮罩")
  }

  const addAllHitsAsMasks = () => {
    let added = 0
    const newAdded = new Set(addedHits)
    hits.forEach((hit, idx) => {
      if (newAdded.has(idx)) return
      const mask: Mask = {
        id: nanoid(),
        x: hit.bbox.x,
        y: hit.bbox.y,
        width: hit.bbox.width,
        height: hit.bbox.height,
      }
      addMask(hit.page, mask)
      newAdded.add(idx)
      added++
    })
    setAddedHits(newAdded)
    if (added > 0) {
      toast.success(`已批量添加 ${added} 处遮罩`)
    }
  }

  if (!selectedFile) {
    return null
  }

  const currentPageHits = hits
    .map((hit, idx) => ({ hit, originalIndex: idx }))
    .filter(({ hit }) => hit.page === currentPage)

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <div
          className="flex items-center gap-1.5 cursor-pointer flex-1"
          onClick={() => setExpanded(!expanded)}
        >
          <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
            <ShieldAlert className="h-3 w-3" />
            敏感信息检测
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
        <>
          <div className="flex flex-wrap gap-1.5">
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

          <div className="flex items-center gap-1.5">
            <Button
              variant="outline"
              size="sm"
              className="flex-1"
              onClick={runDetection}
              disabled={scanning}
            >
              <Search className={`h-3.5 w-3.5 mr-1.5 ${scanning ? "animate-pulse" : ""}`} />
              {scanning ? "扫描中..." : "扫描"}
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
                  当前页
                  {scanScope === "current" && <Check className="h-3 w-3 ml-auto" />}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => setScanScope("all")}>
                  <Files className="h-3.5 w-3.5 mr-2" />
                  全部页
                  {scanScope === "all" && <Check className="h-3 w-3 ml-auto" />}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {needsOcr && (
            <div className="rounded-md bg-amber-50 dark:bg-amber-950/30 p-2 space-y-2">
              <div className="flex items-start gap-2 text-xs text-amber-700 dark:text-amber-400">
                <AlertCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
                <span>检测到图片页，需要 OCR 支持。请先安装/配置 OCR。</span>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="w-full h-7 text-xs"
                onClick={openOcrDialog}
              >
                配置 OCR
              </Button>
            </div>
          )}

          {hits.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <p className="text-xs text-muted-foreground">
                  当前页: {currentPageHits.length} 条 / 共 {hits.length} 条
                </p>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 px-2 text-xs"
                  onClick={addAllHitsAsMasks}
                >
                  <Plus className="h-3 w-3 mr-1" />
                  全部添加
                </Button>
              </div>

              <div className="space-y-1 max-h-32 overflow-y-auto">
                {currentPageHits.map(({ hit, originalIndex }) => {
                  const isAdded = addedHits.has(originalIndex)
                  return (
                    <div
                      key={originalIndex}
                      className={`flex items-center justify-between rounded-md px-2 py-1 text-xs ${
                        isAdded ? "bg-green-50 dark:bg-green-950/30" : "bg-muted/50"
                      }`}
                    >
                      <div className="flex-1 truncate">
                        <span className="font-medium">{hit.ruleName}</span>
                        <span className="text-muted-foreground ml-1.5">{hit.snippet}</span>
                      </div>
                      {isAdded ? (
                        <div className="h-5 w-5 shrink-0 flex items-center justify-center text-green-600">
                          <Check className="h-3 w-3" />
                        </div>
                      ) : (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-5 w-5 shrink-0"
                          onClick={() => addHitAsMask(hit, originalIndex)}
                        >
                          <Plus className="h-3 w-3" />
                        </Button>
                      )}
                    </div>
                  )
                })}
                {currentPageHits.length === 0 && hits.length > 0 && (
                  <p className="text-xs text-muted-foreground text-center py-2">当前页无命中</p>
                )}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  )
}
