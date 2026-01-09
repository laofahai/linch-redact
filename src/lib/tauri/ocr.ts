import { invoke } from "@tauri-apps/api/core"
import type {
  Platform,
  OcrEngineType,
  OcrEngineStatus,
  TesseractConfig,
  TesseractStatus,
  OcrAuditInfo,
} from "@/types"

// ============ 通用 API ============

export async function getPlatform(): Promise<Platform> {
  return invoke("get_platform")
}

export async function getOcrEngineStatus(): Promise<OcrEngineStatus> {
  return invoke("get_ocr_engine_status")
}

export async function setOcrEngine(engineType: OcrEngineType): Promise<void> {
  return invoke("set_ocr_engine", { engineType })
}

export async function getCurrentOcrEngine(): Promise<OcrEngineType> {
  return invoke("get_current_ocr_engine")
}

export async function getOcrAuditInfo(): Promise<OcrAuditInfo> {
  return invoke("get_ocr_audit_info")
}

export interface OcrTextResult {
  text: string
  confidence: number
  bbox: {
    x: number
    y: number
    w: number
    h: number
  }
}

export async function ocrRecognize(imagePath: string): Promise<OcrTextResult[]> {
  return invoke("ocr_recognize", { imagePath })
}

// ============ Paddle OCR API ============

export async function isPaddleOcrInstalled(): Promise<boolean> {
  return invoke("is_paddle_ocr_installed")
}

export interface PaddleInstallRequest {
  detUrl: string
  recUrl: string
  modelVersion?: string
  installSource?: string
}

export async function installPaddleOcr(request: PaddleInstallRequest): Promise<{
  detModelPath: string
  recModelPath: string
}> {
  return invoke("install_paddle_ocr", { request })
}

export async function initPaddleOcr(): Promise<void> {
  return invoke("init_paddle_ocr")
}

// ============ Tesseract OCR API ============

export async function checkTesseractStatus(): Promise<TesseractStatus> {
  return invoke("check_tesseract_status")
}

export async function initTesseractOcr(): Promise<void> {
  return invoke("init_tesseract_ocr")
}

export async function saveTesseractConfig(tesseractConfig: TesseractConfig): Promise<void> {
  return invoke("save_tesseract_config", { tesseractConfig })
}

export async function getTesseractLanguages(): Promise<string[]> {
  return invoke("get_tesseract_languages")
}

export async function getCurrentPlatform(): Promise<string> {
  return invoke("get_current_platform")
}

export async function installTesseractOcr(): Promise<void> {
  return invoke("install_tesseract_ocr")
}

// Tesseract 安装进度事件类型
export interface TesseractInstallProgress {
  stage: string
  message: string
  done: boolean
  success: boolean
  error?: string
}
