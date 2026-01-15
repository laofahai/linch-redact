import { useTranslation } from "react-i18next"
import { PreviewCanvas } from "@/components/features/preview/PreviewCanvas"
import { PreviewPane } from "@/components/features/preview/PreviewPane"
import { MaskList } from "@/components/features/preview/MaskList"
import { ZoomControls } from "@/components/features/preview/ZoomControls"
import { useFileStore, useEditorStore } from "@/stores"
import { open } from "@tauri-apps/plugin-dialog"

export function MainPanel() {
  const { t } = useTranslation()

  // 新架构
  const selectedDocument = useFileStore((s) => s.getSelectedDocument())
  const addDocuments = useFileStore((s) => s.addDocuments)

  // 兼容层
  const selectedFile = useFileStore((s) => s.getSelectedFile())

  const currentPage = useEditorStore((s) => s.currentPage)
  const nextPage = useEditorStore((s) => s.nextPage)
  const prevPage = useEditorStore((s) => s.prevPage)

  // 判断当前是使用新架构还是兼容层
  const hasDocument = selectedDocument !== null
  const hasFile = selectedFile !== null
  const hasContent = hasDocument || hasFile

  // 获取页数（新架构或兼容层）
  const pageCount = hasDocument ? selectedDocument.totalPages : hasFile ? selectedFile.pageCount : 0

  // 是否显示分页控件（PDF 或多页文档）
  const showPagination = hasContent && pageCount > 1

  // 是否显示遮盖框控件（仅 PDF）
  const showMaskControls = hasFile || (hasDocument && selectedDocument.fileType === "pdf")

  const handleAddFile = async () => {
    const selected = await open({
      multiple: true,
      filters: [
        { name: "All Supported", extensions: ["pdf", "txt", "md"] },
        { name: "PDF", extensions: ["pdf"] },
        { name: "Text", extensions: ["txt", "md"] },
      ],
      title: t("sidebar.addFile"),
    })
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected]
      // 使用新架构
      await addDocuments(paths)
    }
  }

  return (
    <main className="flex flex-1 min-w-0 flex-col overflow-hidden bg-muted/30">
      {/* 分页控件 */}
      {showPagination && (
        <div className="flex shrink-0 items-center justify-center gap-4 border-b bg-card px-4 py-2 text-sm">
          <button
            className="rounded px-3 py-1 hover:bg-muted disabled:opacity-50"
            onClick={() => prevPage()}
            disabled={currentPage === 0}
          >
            {t("common.prev")}
          </button>
          <span className="text-muted-foreground">
            {currentPage + 1} / {pageCount || "?"}
          </span>
          <button
            className="rounded px-3 py-1 hover:bg-muted disabled:opacity-50"
            onClick={() => nextPage(pageCount)}
            disabled={currentPage >= pageCount - 1}
          >
            {t("common.next")}
          </button>
        </div>
      )}

      {/* 预览区域 */}
      <div className="flex-1 overflow-auto p-4">
        {hasContent ? (
          <div className="flex min-h-full items-start justify-center">
            {hasDocument ? <PreviewPane /> : <PreviewCanvas file={selectedFile!} />}
          </div>
        ) : (
          <EmptyState onClick={handleAddFile} label={t("sidebar.addFile")} />
        )}
      </div>

      {/* 底部工具栏 */}
      {showMaskControls && (
        <div className="flex shrink-0 items-center justify-between border-t bg-card px-4 py-2">
          <MaskList />
          <ZoomControls />
        </div>
      )}
    </main>
  )
}

function EmptyState({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <div
      className="flex h-full w-full cursor-pointer flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed border-muted-foreground/25 text-muted-foreground transition-colors hover:border-primary hover:bg-muted/50"
      onClick={onClick}
    >
      <div className="flex h-16 w-16 items-center justify-center rounded-full bg-muted">
        <svg className="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
      </div>
      <p className="text-sm">{label}</p>
    </div>
  )
}
