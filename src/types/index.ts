// ============================================================================
// 新架构：通用文档类型
// ============================================================================

/// 支持的文件类型
export type FileType = "pdf" | "txt" | "md" | "docx"

/// 文档页面（来自后端）
export interface DocumentPage {
  page_number: number
  content: string
}

/// 通用文档文件
export interface DocumentFile {
  id: string
  path: string
  name: string
  fileType: FileType
  pages: DocumentPage[]
  totalPages: number
  supportedFeatures: string[]
  status: "pending" | "loading" | "ready" | "processing" | "completed" | "error"
  error?: string
}

// ============================================================================
// 兼容类型（逐步废弃）
// ============================================================================

// 文件相关类型
export interface PdfFile {
  id: string
  path: string
  name: string
  pageCount: number
  pages: Page[]
  status: "pending" | "processing" | "completed" | "error"
  error?: string
}

export interface Page {
  index: number
  action: "keep" | "redact" | "delete"
  thumbnail?: string
}

// 遮盖框类型
export interface Mask {
  id: string
  x: number // 0-1 百分比
  y: number
  width: number
  height: number
}

export interface MasksByPage {
  [pageIndex: number]: Mask[]
}

// 按文件存储的遮罩
export interface MasksByFile {
  [fileId: string]: MasksByPage
}

// 按文件存储的检测结果
export interface DetectionHitsByFile {
  [fileId: string]: DetectionHit[]
}

// OCR 引擎类型
export type OcrEngineType = "paddle" | "tesseract"

// Tesseract 配置
export interface TesseractConfig {
  binaryPath?: string
  tessdataPath?: string
  lang?: string
  psm?: number
  oem?: number
}

// Paddle 状态
export interface PaddleStatus {
  installed: boolean
  detModelPath?: string
  recModelPath?: string
  modelVersion?: string
}

// Tesseract 状态
export interface TesseractStatus {
  installed: boolean
  version?: string
  binaryPath?: string
  tessdataPath?: string
  availableLangs: string[]
  error?: string
}

// OCR 引擎整体状态
export interface OcrEngineStatus {
  paddle: PaddleStatus
  tesseract: TesseractStatus
  currentEngine: OcrEngineType
}

// OCR 审计信息
export interface OcrAuditInfo {
  engineType: OcrEngineType
  engineVersion?: string
  engineParams?: string
  tessdataHash?: string
}

export interface Platform {
  os: "windows" | "macos" | "linux" | ""
  arch: "x86_64" | "aarch64" | ""
}

export type PlatformKey = "win-x64" | "mac-arm64" | "mac-x64" | "linux-x64"

// 脱敏模式
export type RedactionMode = "auto" | "text_replace" | "safe_render" | "image_mode" | "black_overlay"

// 清理选项（与后端 CleaningOptions 对应）
export interface CleaningOptions {
  documentInfo: boolean
  xmpMetadata: boolean
  hiddenData: boolean
  annotations: boolean
  forms: boolean
  attachments: boolean
  javascript: boolean
}

// 页面内容类型
export type PageContentType = "text" | "path_drawn" | "image_based" | "mixed" | "empty"

// PDF 分析结果
export interface PdfAnalysis {
  pageTypes: PageContentType[]
  hasForms: boolean
  hasAnnotations: boolean
  hasMetadata: boolean
  hasAttachments: boolean
  hasJavascript: boolean
  recommendedMode: RedactionMode
}

// 规则类型
export interface Rule {
  id: string
  name: string
  ruleType: "keyword" | "regex"
  pattern: string
  enabled: boolean
}

// 检测命中结果
export interface DetectionHit {
  page: number
  bbox: { x: number; y: number; width: number; height: number }
  ruleId: string
  ruleName: string
  snippet: string
}

// 设置类型
export interface ProcessingSettings {
  mode: "check" | "redact" | "searchable"
  redactionMode: RedactionMode
  cleaning: CleaningOptions
  verification: {
    textRecheck: boolean
    imageSampling: boolean
    outputReport: boolean
  }
  output: {
    directory: string
  }
}

// 处理任务类型
export interface ProcessingTask {
  fileId: string
  status: "queued" | "running" | "paused" | "completed" | "error"
  progress: number
  currentStep: string
  error?: string
}

// OCR 清单类型
export interface OcrModelFile {
  url: string
  mirrorUrl: string
  filename: string
  size: number
}

export interface OcrModelConfig {
  name: string
  description: string
  files: {
    det: OcrModelFile
    rec: OcrModelFile
  }
  version: string
}

export interface OcrManifest {
  version: string
  // 使用 ONNX Runtime，无需单独引擎
  engine: null
  models: Record<string, OcrModelConfig>
}
