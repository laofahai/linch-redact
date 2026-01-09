import { useEffect, useState } from "react"
import { BrowserRouter } from "react-router-dom"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { LinchDesktopProvider, TitleBar, useUpdater } from "@linch-tech/desktop-core"
import { Toaster } from "sonner"
import { config } from "./config"
import { Sidebar } from "@/components/layout/Sidebar"
import { MainPanel } from "@/components/layout/MainPanel"
import { Footer } from "@/components/layout/Footer"
import { OcrSetupDialog } from "@/components/features/ocr/OcrSetupDialog"
import { useOcrStore } from "@/stores/useOcrStore"
import { useFileStore } from "@/stores/useFileStore"

function AppContent() {
  const loadStatus = useOcrStore((s) => s.loadStatus)
  const addFiles = useFileStore((s) => s.addFiles)
  const [isDragging, setIsDragging] = useState(false)
  const { check } = useUpdater()

  useEffect(() => {
    loadStatus()
  }, [loadStatus])

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
          <MainPanel />
          <Footer />
        </div>
      </div>
      <OcrSetupDialog />
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
