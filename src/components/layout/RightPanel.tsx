import { useTranslation } from "react-i18next"
import { CleaningOptionsPanel } from "@/components/features/settings/CleaningOptionsPanel"
import { RulesPanel } from "@/components/features/rules/RulesPanel"
import { useFileStore } from "@/stores"

export function RightPanel() {
  const { t } = useTranslation()
  const selectedDocument = useFileStore((s) => s.getSelectedDocument())

  if (!selectedDocument) {
    return (
      <aside className="flex w-80 shrink-0 flex-col border-l bg-card">
        <div className="flex flex-1 items-center justify-center text-xs text-muted-foreground p-3">
          {t("sidebar.noPdf")}
        </div>
      </aside>
    )
  }

  const isPdf = selectedDocument.fileType === "pdf"

  return (
    <aside className="flex w-80 shrink-0 flex-col border-l bg-card">
      <div className="flex-1 min-h-0 flex flex-col p-3">
        <RulesPanel />
      </div>
      {/* PDF 特有选项 */}
      {isPdf && (
        <div className="shrink-0 border-t px-3 py-2">
          <CleaningOptionsPanel />
        </div>
      )}
    </aside>
  )
}
