import { useEffect, useState } from "react"
import { BrowserRouter } from "react-router-dom"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { LinchDesktopProvider, TitleBar, useUpdater } from "@linch-tech/desktop-core"
import { Toaster } from "sonner"
import { config } from "./config"
import { Sidebar } from "@/components/layout/Sidebar"
import { MainPanel } from "@/components/layout/MainPanel"
import { Footer } from "@/components/layout/Footer"
import { RightPanel } from "@/components/layout/RightPanel"
import { OcrSetupDialog } from "@/components/features/ocr/OcrSetupDialog"
import { SettingsDialog } from "@/components/features/settings/SettingsDialog"
import { useOcrStore } from "@/stores/useOcrStore"
import { useFileStore } from "@/stores/useFileStore"
import { useSettingsDialogStore } from "@/stores/useSettingsDialogStore"
import { useDetectionRulesStore } from "@/stores/useDetectionRulesStore"

function AppContent() {
  const loadStatus = useOcrStore((s) => s.loadStatus)
  const loadRules = useDetectionRulesStore((s) => s.loadRules)
  const addFiles = useFileStore((s) => s.addFiles)
  const hasSelectedFile = !!useFileStore((s) => s.selectedFileId)
  const settingsDialogOpen = useSettingsDialogStore((s) => s.isOpen)
  const closeSettingsDialog = useSettingsDialogStore((s) => s.closeDialog)
  const [isDragging, setIsDragging] = useState(false)
  const { check } = useUpdater()

  useEffect(() => {
    loadStatus()
    loadRules()
  }, [loadStatus, loadRules])

  // 禁用 web 行为（复制、选中、刷新、开发者工具等）
  useEffect(() => {
    // 禁用右键菜单
    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault()
    }

    // 禁用快捷键
    const handleKeyDown = (e: KeyboardEvent) => {
      // 禁用刷新: F5, Ctrl+R, Cmd+R
      if (e.key === "F5" || ((e.ctrlKey || e.metaKey) && e.key === "r")) {
        e.preventDefault()
        return
      }

      // 禁用开发者工具: F12, Ctrl+Shift+I, Cmd+Option+I
      if (
        e.key === "F12" ||
        ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "I") ||
        ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "i")
      ) {
        e.preventDefault()
        return
      }

      // 禁用查看源代码: Ctrl+U, Cmd+U
      if ((e.ctrlKey || e.metaKey) && (e.key === "u" || e.key === "U")) {
        e.preventDefault()
        return
      }

      // 禁用保存: Ctrl+S, Cmd+S
      if ((e.ctrlKey || e.metaKey) && (e.key === "s" || e.key === "S")) {
        e.preventDefault()
        return
      }

      // 禁用打印: Ctrl+P, Cmd+P
      if ((e.ctrlKey || e.metaKey) && (e.key === "p" || e.key === "P")) {
        e.preventDefault()
        return
      }
    }

    // 禁用拖放（但保留文件拖放功能）
    const handleDragStart = (e: DragEvent) => {
      const target = e.target as HTMLElement
      // 如果不是文件输入或特定拖放区域，阻止拖动
      if (!target.closest("[data-allow-drag]")) {
        e.preventDefault()
      }
    }

    document.addEventListener("contextmenu", handleContextMenu)
    document.addEventListener("keydown", handleKeyDown)
    document.addEventListener("dragstart", handleDragStart)

    return () => {
      document.removeEventListener("contextmenu", handleContextMenu)
      document.removeEventListener("keydown", handleKeyDown)
      document.removeEventListener("dragstart", handleDragStart)
    }
  }, [])

  // 检查更新
  useEffect(() => {
    const timer = setTimeout(() => {
      check().catch(console.error)
    }, 2000)
    return () => clearTimeout(timer)
  }, [check])

  // 监听文件拖放事件
  useEffect(() => {
    const appWindow = getCurrentWindow()
    const unlisten = appWindow.onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setIsDragging(true)
      } else if (event.payload.type === "drop") {
        setIsDragging(false)
        const paths = event.payload.paths
        // 只添加 PDF 文件
        const pdfPaths = paths.filter((p) => p.toLowerCase().endsWith(".pdf"))
        if (pdfPaths.length > 0) {
          addFiles(pdfPaths)
        }
      } else if (event.payload.type === "leave") {
        setIsDragging(false)
      }
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [addFiles])

  return (
    <div className="relative flex h-screen flex-col overflow-hidden bg-background border border-border">
      <TitleBar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <div className="flex flex-1 overflow-hidden">
            <MainPanel />
            {hasSelectedFile && <RightPanel />}
          </div>
          <Footer />
        </div>
      </div>
      <OcrSetupDialog />
      <SettingsDialog open={settingsDialogOpen} onOpenChange={closeSettingsDialog} />
      <Toaster position="top-center" richColors />

      {/* 拖放覆盖层 */}
      {isDragging && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
          <div className="rounded-lg border-2 border-dashed border-primary p-8 text-center">
            <p className="text-lg font-medium">释放以添加 PDF 文件</p>
          </div>
        </div>
      )}
    </div>
  )
}

export default function App() {
  return (
    <LinchDesktopProvider config={config}>
      <BrowserRouter>
        <AppContent />
      </BrowserRouter>
    </LinchDesktopProvider>
  )
}
