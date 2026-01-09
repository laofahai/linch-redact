import { invoke } from "@tauri-apps/api/core"

// AppConfig 类型（与后端 AppConfig 对应）
export interface AppConfig {
  detModelPath?: string
  recModelPath?: string
  modelVersion?: string
  installSource?: string
  useMirror?: boolean
  ocrEngine?: "paddle" | "tesseract"
  tesseract?: {
    binaryPath?: string
    tessdataPath?: string
    lang?: string
    psm?: number
    oem?: number
  }
}

export async function loadConfig(): Promise<AppConfig> {
  return invoke("load_config")
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke("save_config", { config })
}
