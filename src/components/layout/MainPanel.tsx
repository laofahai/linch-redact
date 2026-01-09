import { PreviewCanvas } from "@/components/features/preview/PreviewCanvas"
import { MaskList } from "@/components/features/preview/MaskList"
import { ZoomControls } from "@/components/features/preview/ZoomControls"
import { useFileStore, useEditorStore } from "@/stores"
import { open } from "@tauri-apps/plugin-dialog"

export function MainPanel() {
  const selectedFile = useFileStore((s) => s.getSelectedFile())
  const addFiles = useFileStore((s) => s.addFiles)
  const currentPage = useEditorStore((s) => s.currentPage)
  const nextPage = useEditorStore((s) => s.nextPage)
  const prevPage = useEditorStore((s) => s.prevPage)

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
    <main className="flex flex-1 flex-col overflow-hidden bg-muted/30">
      {/* 顶部页面导航 - 固定 */}
      {selectedFile && (
        <div className="flex shrink-0 items-center justify-center gap-4 border-b bg-card px-4 py-2 text-sm">
          <button
            className="rounded px-3 py-1 hover:bg-muted disabled:opacity-50"
            onClick={() => prevPage()}
            disabled={currentPage === 0}
          >
            上一页
          </button>
          <span className="text-muted-foreground">
            {currentPage + 1} / {selectedFile.pageCount || "?"}
          </span>
          <button
            className="rounded px-3 py-1 hover:bg-muted disabled:opacity-50"
            onClick={() => nextPage(selectedFile.pageCount)}
            disabled={currentPage >= selectedFile.pageCount - 1}
          >
            下一页
          </button>
        </div>
      )}

      {/* 预览区域 */}
      <div className="flex-1 overflow-auto p-4">
        {selectedFile ? (
          <div className="flex min-h-full items-start justify-center">
            <PreviewCanvas file={selectedFile} />
          </div>
        ) : (
          <EmptyState onClick={handleAddFile} />
        )}
      </div>

      {/* 底部工具栏 */}
      {selectedFile && (
        <div className="flex shrink-0 items-center justify-between border-t bg-card px-4 py-2">
          <MaskList />
          <ZoomControls />
        </div>
      )}
    </main>
  )
}

function EmptyState({ onClick }: { onClick: () => void }) {
  return (
    <div
      className="flex h-full w-full cursor-pointer flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed border-muted-foreground/25 text-muted-foreground transition-colors hover:border-primary hover:bg-muted/50"
      onClick={onClick}
    >
      <div className="flex h-16 w-16 items-center justify-center rounded-full bg-muted">
        <svg
          className="h-8 w-8"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
      </div>
      <p className="text-sm">点击选择或拖入 PDF 文件</p>
    </div>
  )
}
