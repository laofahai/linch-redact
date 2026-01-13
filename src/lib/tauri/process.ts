import { invoke } from "@tauri-apps/api/core"

interface Mask {
  x: number
  y: number
  width: number
  height: number
}

interface PageAction {
  index: number
  action: string
}

interface FileProcessRequest {
  path: string
  pages: PageAction[]
  masks_by_page: Record<number, Mask[]>
}

interface CleaningOptions {
  documentInfo: boolean
  xmpMetadata: boolean
  annotations: boolean
  forms: boolean
  attachments: boolean
  javascript: boolean
}

interface ProcessRequest {
  files: FileProcessRequest[]
  output_directory: string
  prefix: string
  mode: string
  cleaning: CleaningOptions
}

interface ProcessResult {
  success: boolean
  processed_files: string[]
  errors: string[]
}

export async function processPdfs(request: ProcessRequest): Promise<ProcessResult> {
  return invoke<ProcessResult>("process_pdfs", { request })
}
