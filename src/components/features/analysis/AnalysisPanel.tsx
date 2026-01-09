import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import {
  FileText,
  Pen,
  ImageIcon,
  Layers,
  FormInput,
  MessageSquare,
  FileJson,
  Paperclip,
  Code,
  RefreshCw,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { useFileStore, useSettingsStore } from "@/stores"
import type { PdfAnalysis, PageContentType, RedactionMode } from "@/types"

// 调用 Tauri 命令分析 PDF
async function analyzePdf(path: string): Promise<PdfAnalysis> {
  return await invoke<PdfAnalysis>("analyze_pdf", { pdfPath: path })
}

const pageTypeLabels: Record<PageContentType, { label: string; icon: React.ComponentType<{ className?: string }> }> = {
  text: { label: "纯文字", icon: FileText },
  path_drawn: { label: "路径绘制", icon: Pen },
  image_based: { label: "扫描件", icon: ImageIcon },
  mixed: { label: "混合内容", icon: Layers },
  empty: { label: "空白页", icon: FileText },
}

const modeLabels: Record<RedactionMode, string> = {
  auto: "自动模式",
  text_replace: "文字替换",
  safe_render: "安全渲染",
  image_mode: "图像处理",
  black_overlay: "黑色覆盖",
}

export function AnalysisPanel() {
  const selectedFile = useFileStore((s) => s.getSelectedFile())
  const setRedactionMode = useSettingsStore((s) => s.setRedactionMode)
  const [analysis, setAnalysis] = useState<PdfAnalysis | null>(null)
  const [loading, setLoading] = useState(false)

  const runAnalysis = async () => {
    if (!selectedFile?.path) return
    setLoading(true)
    try {
      const result = await analyzePdf(selectedFile.path)
      setAnalysis(result)
    } catch (e) {
      console.error("Analysis failed:", e)
    } finally {
      setLoading(false)
    }
  }

  // 切换文件时清除分析结果
  useEffect(() => {
    setAnalysis(null)
  }, [selectedFile?.id])

  if (!selectedFile) {
    return null
  }

  // 统计页面类型
  const pageTypeStats = analysis?.pageTypes.reduce((acc, type) => {
    acc[type] = (acc[type] || 0) + 1
    return acc
  }, {} as Record<PageContentType, number>)

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          PDF 分析
        </h3>
        <Button
          variant="ghost"
          size="sm"
          onClick={runAnalysis}
          disabled={loading}
          className="h-6 px-2 text-xs"
        >
          <RefreshCw className={`h-3 w-3 mr-1 ${loading ? "animate-spin" : ""}`} />
          {loading ? "分析中" : analysis ? "重新分析" : "分析"}
        </Button>
      </div>

      {!analysis ? (
        <p className="text-xs text-muted-foreground">
          点击分析按钮检测 PDF 内容类型
        </p>
      ) : (
        <div className="space-y-3">
          {/* 页面类型统计 */}
          {pageTypeStats && Object.keys(pageTypeStats).length > 0 && (
            <div className="space-y-1.5">
              <p className="text-xs text-muted-foreground">页面类型</p>
              <div className="flex flex-wrap gap-1">
                {Object.entries(pageTypeStats).map(([type, count]) => {
                  const { label, icon: Icon } = pageTypeLabels[type as PageContentType]
                  return (
                    <span
                      key={type}
                      className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs"
                    >
                      <Icon className="h-3 w-3" />
                      {label} ({count})
                    </span>
                  )
                })}
              </div>
            </div>
          )}

          {/* 文档特征 */}
          <div className="space-y-1.5">
            <p className="text-xs text-muted-foreground">文档特征</p>
            <div className="flex flex-wrap gap-1">
              {analysis.hasForms && (
                <span className="inline-flex items-center gap-1 rounded-full bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400 px-2 py-0.5 text-xs">
                  <FormInput className="h-3 w-3" />
                  表单
                </span>
              )}
              {analysis.hasAnnotations && (
                <span className="inline-flex items-center gap-1 rounded-full bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400 px-2 py-0.5 text-xs">
                  <MessageSquare className="h-3 w-3" />
                  注释
                </span>
              )}
              {analysis.hasMetadata && (
                <span className="inline-flex items-center gap-1 rounded-full bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400 px-2 py-0.5 text-xs">
                  <FileJson className="h-3 w-3" />
                  元数据
                </span>
              )}
              {analysis.hasAttachments && (
                <span className="inline-flex items-center gap-1 rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 px-2 py-0.5 text-xs">
                  <Paperclip className="h-3 w-3" />
                  附件
                </span>
              )}
              {analysis.hasJavascript && (
                <span className="inline-flex items-center gap-1 rounded-full bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 px-2 py-0.5 text-xs">
                  <Code className="h-3 w-3" />
                  脚本
                </span>
              )}
              {!analysis.hasForms &&
                !analysis.hasAnnotations &&
                !analysis.hasMetadata &&
                !analysis.hasAttachments &&
                !analysis.hasJavascript && (
                  <span className="text-xs text-muted-foreground">无特殊内容</span>
                )}
            </div>
          </div>

          {/* 推荐模式 */}
          <div className="space-y-1.5">
            <p className="text-xs text-muted-foreground">推荐模式</p>
            <Button
              variant="outline"
              size="sm"
              className="w-full justify-start h-8 text-xs"
              onClick={() => setRedactionMode(analysis.recommendedMode)}
            >
              使用推荐: {modeLabels[analysis.recommendedMode]}
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
