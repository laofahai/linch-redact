import { create } from "zustand"
import { nanoid } from "nanoid"
import type { PdfFile, Page } from "@/types"
import { useSettingsStore } from "./useSettingsStore"
import { useEditorStore } from "./useEditorStore"

interface FileStore {
  files: PdfFile[]
  selectedFileId: string | null

  // 计算属性
  getSelectedFile: () => PdfFile | null

  // 操作
  addFiles: (paths: string[]) => void
  removeFile: (id: string) => void
  clearFiles: () => void
  selectFile: (id: string | null) => void
  updateFile: (id: string, updates: Partial<PdfFile>) => void
  setPageCount: (id: string, count: number) => void
  setPageAction: (id: string, pageIndex: number, action: Page["action"]) => void
}

// 获取文件所在目录
function getDirectory(filePath: string): string {
  const parts = filePath.split(/[/\\]/)
  parts.pop()
  return parts.join("/") || "/"
}

export const useFileStore = create<FileStore>((set, get) => ({
  files: [],
  selectedFileId: null,

  getSelectedFile: () => {
    const { files, selectedFileId } = get()
    return files.find((f) => f.id === selectedFileId) ?? null
  },

  addFiles: (paths) => {
    const newFiles: PdfFile[] = paths.map((path) => ({
      id: nanoid(),
      path,
      name: path.split(/[/\\]/).pop() ?? path,
      pageCount: 0,
      pages: [],
      status: "pending",
    }))

    set((state) => {
      const updated = [...state.files, ...newFiles]

      // 如果是第一次添加文件且输出目录为空，设置默认输出目录
      if (state.files.length === 0 && newFiles.length > 0) {
        const settingsStore = useSettingsStore.getState()
        if (!settingsStore.settings.output.directory) {
          const defaultDir = getDirectory(newFiles[0].path)
          settingsStore.setOutputDirectory(defaultDir)
        }
      }

      const newSelectedId = state.selectedFileId ?? newFiles[0]?.id ?? null

      // 如果是第一个文件，设置 editor store 的 currentFileId
      if (!state.selectedFileId && newSelectedId) {
        useEditorStore.getState().setCurrentFileId(newSelectedId)
      }

      return {
        files: updated,
        selectedFileId: newSelectedId,
      }
    })
  },

  removeFile: (id) => {
    // 清除该文件关联的 masks
    useEditorStore.getState().clearFileMasks(id)

    set((state) => {
      const files = state.files.filter((f) => f.id !== id)
      const newSelectedId =
        state.selectedFileId === id ? (files[0]?.id ?? null) : state.selectedFileId

      // 更新 editor store 的 currentFileId
      if (newSelectedId !== state.selectedFileId) {
        useEditorStore.getState().setCurrentFileId(newSelectedId)
      }

      return { files, selectedFileId: newSelectedId }
    })
  },

  clearFiles: () => {
    // 清除所有 masks 和重置编辑器状态
    useEditorStore.getState().clearAllMasks()
    useEditorStore.getState().setCurrentFileId(null)

    set({ files: [], selectedFileId: null })
  },

  selectFile: (id) => {
    const { selectedFileId } = get()
    if (id !== selectedFileId) {
      // 切换文件时更新 editor store 的 currentFileId
      useEditorStore.getState().setCurrentFileId(id)
    }
    set({ selectedFileId: id })
  },

  updateFile: (id, updates) => {
    set((state) => ({
      files: state.files.map((f) => (f.id === id ? { ...f, ...updates } : f)),
    }))
  },

  setPageCount: (id, count) => {
    set((state) => ({
      files: state.files.map((f) => {
        if (f.id !== id) return f
        const pages: Page[] = Array.from({ length: count }, (_, i) => ({
          index: i,
          action: "keep" as const,
        }))
        return { ...f, pageCount: count, pages }
      }),
    }))
  },

  setPageAction: (id, pageIndex, action) => {
    set((state) => ({
      files: state.files.map((f) => {
        if (f.id !== id) return f
        const pages = f.pages.map((p) => (p.index === pageIndex ? { ...p, action } : p))
        return { ...f, pages }
      }),
    }))
  },
}))
