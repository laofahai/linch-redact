import { useTranslation } from "react-i18next"
import { FilePlus, Files, Layers, Settings } from "lucide-react"
import { PageList } from "@/components/features/pages/PageList"
import { useFileStore, useSettingsDialogStore } from "@/stores"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { Button } from "@/components/ui/button"
import { open } from "@tauri-apps/plugin-dialog"

export function Sidebar() {
  const { t } = useTranslation()

  const documents = useFileStore((s) => s.documents)
  const selectedDocumentId = useFileStore((s) => s.selectedDocumentId)
  const addDocuments = useFileStore((s) => s.addDocuments)
  const selectDocument = useFileStore((s) => s.selectDocument)
  const removeDocument = useFileStore((s) => s.removeDocument)

  const openSettingsDialog = useSettingsDialogStore((s) => s.openDialog)

  const selectedDocument = documents.find((d) => d.id === selectedDocumentId)
  const pageCount = selectedDocument?.totalPages ?? 0
  const showPageList = selectedDocument && pageCount > 1

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
      await addDocuments(paths)
    }
  }

  const getFileTypeIcon = (fileType: string) => {
    switch (fileType) {
      case "pdf":
        return "📄"
      case "txt":
        return "📝"
      case "md":
        return "📑"
      default:
        return "📁"
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case "loading":
        return "text-yellow-500"
      case "ready":
        return "text-green-500"
      case "error":
        return "text-red-500"
      default:
        return "text-muted-foreground"
    }
  }

  return (
    <aside className="flex w-80 shrink-0 flex-col border-r bg-card overflow-hidden">
      <div className="flex-1 min-h-0 flex flex-col p-3 pb-0">
        <div className="flex items-center justify-between mb-2 shrink-0">
          <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
            <Files className="h-3 w-3" />
            {t("sidebar.fileList")}
          </h3>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleAddFile}>
            <FilePlus className="h-3.5 w-3.5" />
          </Button>
        </div>
        <ScrollArea className="flex-1 file-list-scroll">
          <div className="space-y-1">
            {documents.map((doc) => (
              <div
                key={doc.id}
                className={`group flex items-center gap-2 rounded-md px-2 py-1.5 cursor-pointer transition-colors ${
                  selectedDocumentId === doc.id ? "bg-primary/10 text-primary" : "hover:bg-muted"
                }`}
                onClick={() => selectDocument(doc.id)}
              >
                <span className="text-sm">{getFileTypeIcon(doc.fileType)}</span>
                <div className="flex-1 min-w-0">
                  <p className="text-sm truncate">{doc.name}</p>
                  <p className={`text-xs ${getStatusColor(doc.status)}`}>
                    {doc.status === "loading" && t("common.loading")}
                    {doc.status === "ready" && `${doc.totalPages} ${t("common.pages")}`}
                    {doc.status === "error" && t("errors.loadFailed")}
                  </p>
                </div>
                <button
                  className="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 rounded transition-opacity"
                  onClick={(e) => {
                    e.stopPropagation()
                    removeDocument(doc.id)
                  }}
                >
                  <svg
                    className="h-3 w-3 text-destructive"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </button>
              </div>
            ))}
          </div>
        </ScrollArea>
      </div>

      {showPageList && (
        <>
          <Separator />
          <div className="flex-1 min-h-0 flex flex-col p-3 py-2">
            <div className="flex items-center justify-between mb-2 shrink-0">
              <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground flex items-center gap-1.5">
                <Layers className="h-3 w-3" />
                {t("sidebar.pageList")}
              </h3>
              <span className="text-xs text-muted-foreground">
                {pageCount} {t("common.pages")}
              </span>
            </div>
            <ScrollArea className="flex-1">
              <PageList hideTitle />
            </ScrollArea>
          </div>
        </>
      )}

      <div className="h-14 shrink-0 flex items-center px-2 border-t">
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-start gap-2 text-muted-foreground hover:text-foreground"
          onClick={openSettingsDialog}
        >
          <Settings className="h-4 w-4" />
          {t("sidebar.settings")}
        </Button>
      </div>
    </aside>
  )
}
