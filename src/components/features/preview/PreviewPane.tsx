import { useFileStore } from "@/stores"
import { PreviewCanvas } from "./PreviewCanvas"
import { TextPreview } from "./TextPreview"
import { useTranslation } from "react-i18next"

/**
 * 动态预览面板
 *
 * 根据当前选中文档的类型，自动加载对应的预览组件。
 */
export function PreviewPane() {
  const { t } = useTranslation()
  const document = useFileStore((s) => s.getSelectedDocument())
  const selectedFile = useFileStore((s) => s.getSelectedFile())

  // 新架构：优先使用 documents
  if (document) {
    switch (document.fileType) {
      case "pdf":
        // PDF 仍然使用原有的 PreviewCanvas（需要 PdfFile 类型）
        // 这里暂时显示文本预览，后续可以适配
        return <TextPreview document={document} />
      case "txt":
      case "md":
        return <TextPreview document={document} />
      default:
        return (
          <div className="flex items-center justify-center h-full">
            <p className="text-muted-foreground">不支持预览此文件类型: {document.fileType}</p>
          </div>
        )
    }
  }

  // 兼容层：使用原有的 files
  if (selectedFile) {
    return <PreviewCanvas file={selectedFile} />
  }

  // 无文件
  return (
    <div className="flex items-center justify-center h-full">
      <p className="text-muted-foreground">{t("preview.noFile")}</p>
    </div>
  )
}
