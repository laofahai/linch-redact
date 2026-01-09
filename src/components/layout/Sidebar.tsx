import { useState } from "react"
import { ChevronDown, ChevronRight, FilePlus, Files, Layers, Settings } from "lucide-react"
import { FileList } from "@/components/features/files/FileList"
import { PageList } from "@/components/features/pages/PageList"
import { DetectionPanel } from "@/components/features/detection/DetectionPanel"
import { ModeSelector } from "@/components/features/settings/ModeSelector"
import { useFileStore, useSettingsStore } from "@/stores"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { Button } from "@/components/ui/button"
import { open } from "@tauri-apps/plugin-dialog"

function CleaningOptions() {
  const [expanded, setExpanded] = useState(false)
  const settings = useSettingsStore((s) => s.settings)
  const toggleCleaning = useSettingsStore((s) => s.toggleCleaning)

  const cleaningOptions = [
    { key: "documentInfo" as const, label: "文档信息" },
    { key: "xmpMetadata" as const, label: "元数据" },
    { key: "annotations" as const, label: "批注" },
    { key: "forms" as const, label: "表单" },
    { key: "attachments" as const, label: "附件" },
    { key: "javascript" as const, label: "脚本" },
  ]

  // 统计启用的选项数量
  const enabledCount = cleaningOptions.filter(
    (opt) => settings.cleaning[opt.key]
  ).length

  return (
    <div className="space-y-2">
      <div
        className="flex items-center justify-between cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
          <Settings className="h-3 w-3" />
          高级选项
        </h3>
        <div className="flex items-center gap-1">
          {enabledCount > 0 && (
            <span className="text-xs text-muted-foreground">
              {enabledCount} 项
            </span>
          )}
          {expanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
        </div>
      </div>

      {expanded && (
        <div className="grid grid-cols-2 gap-x-2 gap-y-1.5 pl-1">
          {cleaningOptions.map((option) => (
            <div key={option.key} className="flex items-center gap-1.5">
              <Checkbox
                id={option.key}
                checked={settings.cleaning[option.key]}
                onCheckedChange={() => toggleCleaning(option.key)}
                className="h-3.5 w-3.5"
              />
              <Label
                htmlFor={option.key}
                className="text-xs font-normal cursor-pointer"
              >
                {option.label}
              </Label>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export function Sidebar() {
  const files = useFileStore((s) => s.files)
  const selectedFileId = useFileStore((s) => s.selectedFileId)
  const addFiles = useFileStore((s) => s.addFiles)
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
      {/* 文件列表 - 可滚动区域 */}
      <div className="flex-1 min-h-0 flex flex-col p-3 pb-0">
        <div className="flex items-center justify-between mb-2 shrink-0">
          <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
            <Files className="h-3 w-3" />
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
        <ScrollArea className="flex-1">
          <FileList hideTitle />
        </ScrollArea>
      </div>

      {/* 以下内容仅在选中文件时显示 */}
      {hasSelectedFile && (
        <>
          <Separator />

          {/* 脱敏模式选择 - 固定高度 */}
          <div className="shrink-0 p-3 py-2">
            <ModeSelector />
          </div>

          <Separator />

          {/* 页面列表 - 可滚动区域 */}
          <div className="flex-1 min-h-0 flex flex-col p-3 py-2">
            <div className="flex items-center justify-between mb-2 shrink-0">
              <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
                <Layers className="h-3 w-3" />
                页面列表
              </h3>
              <span className="text-xs text-muted-foreground">
                {pageCount} 页
              </span>
            </div>
            <ScrollArea className="flex-1">
              <PageList hideTitle />
            </ScrollArea>
          </div>

          <Separator />

          {/* 敏感信息检测 - 固定高度 */}
          <div className="shrink-0 p-3 py-2">
            <DetectionPanel />
          </div>

          <Separator />

          {/* 高级选项 - 固定高度 */}
          <div className="shrink-0 p-3 pt-2">
            <CleaningOptions />
          </div>
        </>
      )}
    </aside>
  )
}
