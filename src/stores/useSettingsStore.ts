import { create } from "zustand"
import type { ProcessingSettings, RedactionMode } from "@/types"

type Template = "external" | "internal" | "custom"

interface SettingsStore {
  settings: ProcessingSettings
  template: Template

  // 操作
  updateSettings: (updates: Partial<ProcessingSettings>) => void
  setTemplate: (template: Template) => void
  setRedactionMode: (mode: RedactionMode) => void
  toggleCleaning: (key: keyof ProcessingSettings["cleaning"]) => void
  toggleVerification: (key: keyof ProcessingSettings["verification"]) => void
  setOutputDirectory: (directory: string) => void
}

const defaultSettings: ProcessingSettings = {
  mode: "redact",
  redactionMode: "text_replace",
  cleaning: {
    documentInfo: true,
    xmpMetadata: true,
    hiddenData: true,
    annotations: false, // 默认不删除注释（印章等可能是注释）
    forms: false,
    attachments: true,
    javascript: true,
  },
  verification: {
    textRecheck: true,
    imageSampling: false,
    outputReport: false,
  },
  output: {
    directory: "",
  },
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: defaultSettings,
  template: "external",

  updateSettings: (updates) => {
    set((state) => ({
      settings: { ...state.settings, ...updates },
    }))
  },

  setTemplate: (template) => {
    set({ template })
  },

  setRedactionMode: (mode) => {
    set((state) => ({
      settings: {
        ...state.settings,
        redactionMode: mode,
      },
    }))
  },

  toggleCleaning: (key) => {
    set((state) => ({
      settings: {
        ...state.settings,
        cleaning: {
          ...state.settings.cleaning,
          [key]: !state.settings.cleaning[key],
        },
      },
    }))
  },

  toggleVerification: (key) => {
    set((state) => ({
      settings: {
        ...state.settings,
        verification: {
          ...state.settings.verification,
          [key]: !state.settings.verification[key],
        },
      },
    }))
  },

  setOutputDirectory: (directory) => {
    set((state) => ({
      settings: {
        ...state.settings,
        output: { ...state.settings.output, directory },
      },
    }))
  },
}))
