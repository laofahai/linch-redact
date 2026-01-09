import { create } from "zustand"
import type { Platform, OcrEngineType, OcrEngineStatus, TesseractConfig } from "@/types"
import * as tauri from "@/lib/tauri"
import { ocrManifest } from "@/data/ocrManifest"

interface OcrStore {
  // 状态
  platform: Platform
  engineStatus: OcrEngineStatus | null
  currentEngine: OcrEngineType
  isLoading: boolean
  isInstalling: boolean
  dialogOpen: boolean
  statusMessage: string
  useMirror: boolean

  // 操作
  loadStatus: () => Promise<void>
  setCurrentEngine: (engine: OcrEngineType) => Promise<void>
  installPaddleModels: () => Promise<void>
  initCurrentEngine: () => Promise<void>
  saveTesseractConfig: (config: TesseractConfig) => Promise<void>
  setUseMirror: (use: boolean) => void
  openDialog: () => void
  closeDialog: () => void
  setStatusMessage: (msg: string) => void
}

const defaultPlatform: Platform = {
  os: "",
  arch: "",
}

export const useOcrStore = create<OcrStore>((set, get) => ({
  platform: defaultPlatform,
  engineStatus: null,
  currentEngine: "paddle",
  isLoading: false,
  isInstalling: false,
  dialogOpen: false,
  statusMessage: "",
  useMirror: true,

  loadStatus: async () => {
    set({ isLoading: true })
    try {
      const [platform, engineStatus] = await Promise.all([
        tauri.getPlatform(),
        tauri.getOcrEngineStatus(),
      ])
      set({
        platform,
        engineStatus,
        currentEngine: engineStatus.currentEngine,
        isLoading: false,
      })
    } catch (error) {
      console.error("Failed to load OCR status:", error)
      set({ isLoading: false })
    }
  },

  setCurrentEngine: async (engine: OcrEngineType) => {
    try {
      await tauri.setOcrEngine(engine)
      set({ currentEngine: engine })

      // 刷新状态
      const engineStatus = await tauri.getOcrEngineStatus()
      set({ engineStatus })
    } catch (error) {
      console.error("Failed to set OCR engine:", error)
      throw error
    }
  },

  installPaddleModels: async () => {
    const { useMirror } = get()
    const model = ocrManifest.models.ppocr_v5

    if (!model) {
      set({ statusMessage: "模型配置不存在" })
      return
    }

    set({ isInstalling: true, statusMessage: "正在下载模型..." })

    try {
      const detUrl = useMirror ? model.files.det.mirrorUrl : model.files.det.url
      const recUrl = useMirror ? model.files.rec.mirrorUrl : model.files.rec.url

      await tauri.installPaddleOcr({
        detUrl,
        recUrl,
        modelVersion: model.version,
        installSource: "online",
      })

      // 刷新状态
      const engineStatus = await tauri.getOcrEngineStatus()
      set({
        engineStatus,
        statusMessage: "安装完成！",
      })

      setTimeout(() => {
        set({ isInstalling: false, dialogOpen: false, statusMessage: "" })
      }, 500)
    } catch (error) {
      console.error("Failed to install Paddle models:", error)
      set({ isInstalling: false, statusMessage: `安装失败: ${error}` })
      throw error
    }
  },

  initCurrentEngine: async () => {
    const { currentEngine } = get()
    try {
      if (currentEngine === "paddle") {
        await tauri.initPaddleOcr()
      } else {
        await tauri.initTesseractOcr()
      }
    } catch (error) {
      console.error("Failed to init OCR engine:", error)
      throw error
    }
  },

  saveTesseractConfig: async (config: TesseractConfig) => {
    try {
      await tauri.saveTesseractConfig(config)
      // 刷新状态
      const engineStatus = await tauri.getOcrEngineStatus()
      set({ engineStatus })
    } catch (error) {
      console.error("Failed to save Tesseract config:", error)
      throw error
    }
  },

  setUseMirror: (use) => set({ useMirror: use }),
  openDialog: () => set({ dialogOpen: true }),
  closeDialog: () => set({ dialogOpen: false, statusMessage: "" }),
  setStatusMessage: (msg) => set({ statusMessage: msg }),
}))
