import { useTranslation } from "react-i18next"
import type { DocumentFile } from "@/types"

interface TextPreviewProps {
  document: DocumentFile
}

export function TextPreview({ document }: TextPreviewProps) {
  const { t } = useTranslation()

  if (document.status === "loading") {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-muted-foreground">{t("common.loading")}</div>
      </div>
    )
  }

  if (document.status === "error") {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-sm text-destructive">{document.error}</p>
          <p className="text-xs text-muted-foreground mt-1">{document.name}</p>
        </div>
      </div>
    )
  }

  const content = document.pages[0]?.content ?? ""

  return (
    <div className="flex flex-col h-full">
      {/* 文件信息栏 */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium uppercase px-2 py-0.5 rounded bg-primary/10 text-primary">
            {document.fileType}
          </span>
          <span className="text-sm text-muted-foreground">{document.name}</span>
        </div>
        <span className="text-xs text-muted-foreground">{content.length} 字符</span>
      </div>

      {/* 文本内容 */}
      <div className="flex-1 overflow-auto p-4">
        <pre className="whitespace-pre-wrap font-mono text-sm leading-relaxed text-foreground">
          {content || <span className="text-muted-foreground italic">（空文件）</span>}
        </pre>
      </div>
    </div>
  )
}
