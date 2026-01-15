import { invoke } from "@tauri-apps/api/core"
import type { DocumentPage, FileType } from "@/types"

/// 后端返回的文档信息
export interface DocumentInfo {
  path: string
  name: string
  file_type: string
  pages: DocumentPage[]
  total_pages: number
  supported_features: string[]
}

/// 加载文档
export async function loadDocument(filePath: string): Promise<DocumentInfo> {
  return invoke<DocumentInfo>("load_document", { filePath })
}

/// 判断文件类型
export function getFileType(filePath: string): FileType | null {
  const ext = filePath.split(".").pop()?.toLowerCase()
  switch (ext) {
    case "pdf":
      return "pdf"
    case "txt":
      return "txt"
    case "md":
      return "md"
    case "docx":
      return "docx"
    default:
      return null
  }
}

/// 判断是否支持的文件类型
export function isSupportedFileType(filePath: string): boolean {
  return getFileType(filePath) !== null
}
