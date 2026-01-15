import { create } from "zustand"
import { nanoid } from "nanoid"
import type { PdfFile, Page, DocumentFile, FileType } from "@/types"
import { useSettingsStore } from "./useSettingsStore"
import { useEditorStore } from "./useEditorStore"
import { loadDocument, getFileType, isSupportedFileType } from "@/lib/tauri/document"

interface FileStore {
  // ============================================================================
  // 新架构：通用文档支持
  // ============================================================================
  documents: DocumentFile[]
  selectedDocumentId: string | null
  isLoading: boolean
  error: string | null

  // 新架构操作
  getSelectedDocument: () => DocumentFile | null
  addDocuments: (paths: string[]) => Promise<void>
  removeDocument: (id: string) => void
  clearDocuments: () => void
  selectDocument: (id: string | null) => void

  // ============================================================================
  // 兼容层：保留原有 PDF 功能
  // ============================================================================
  files: PdfFile[]
  selectedFileId: string | null

  getSelectedFile: () => PdfFile | null
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
  // ============================================================================
  // 新架构状态
  // ============================================================================
  documents: [],
  selectedDocumentId: null,
  isLoading: false,
  error: null,

  getSelectedDocument: () => {
    const { documents, selectedDocumentId } = get()
    return documents.find((d) => d.id === selectedDocumentId) ?? null
  },

  addDocuments: async (paths) => {
    // 过滤不支持的文件类型
    const supportedPaths = paths.filter(isSupportedFileType)
    if (supportedPaths.length === 0) {
      set({ error: "没有支持的文件类型" })
      return
    }

    // 创建占位文档
    const placeholders: DocumentFile[] = supportedPaths.map((path) => ({
      id: nanoid(),
      path,
      name: path.split(/[/\\]/).pop() ?? path,
      fileType: getFileType(path) as FileType,
      pages: [],
      totalPages: 0,
      supportedFeatures: [],
      status: "loading",
    }))

    set((state) => ({
      documents: [...state.documents, ...placeholders],
      selectedDocumentId: state.selectedDocumentId ?? placeholders[0]?.id ?? null,
      isLoading: true,
      error: null,
    }))

    // 设置默认输出目录
    if (get().documents.length === placeholders.length) {
      const settingsStore = useSettingsStore.getState()
      if (!settingsStore.settings.output.directory) {
        settingsStore.setOutputDirectory(getDirectory(supportedPaths[0]))
      }
    }

    // 并行加载所有文档
    for (const placeholder of placeholders) {
      try {
        const info = await loadDocument(placeholder.path)
        set((state) => ({
          documents: state.documents.map((d) =>
            d.id === placeholder.id
              ? {
                  ...d,
                  fileType: info.file_type as FileType,
                  pages: info.pages,
                  totalPages: info.total_pages,
                  supportedFeatures: info.supported_features,
                  status: "ready",
                }
              : d
          ),
        }))
      } catch (e) {
        set((state) => ({
          documents: state.documents.map((d) =>
            d.id === placeholder.id ? { ...d, status: "error", error: String(e) } : d
          ),
        }))
      }
    }

    set({ isLoading: false })
  },

  removeDocument: (id) => {
    useEditorStore.getState().clearFileMasks(id)
    set((state) => {
      const documents = state.documents.filter((d) => d.id !== id)
      const newSelectedId =
        state.selectedDocumentId === id ? (documents[0]?.id ?? null) : state.selectedDocumentId
      return { documents, selectedDocumentId: newSelectedId }
    })
  },

  clearDocuments: () => {
    useEditorStore.getState().clearAllMasks()
    useEditorStore.getState().setCurrentFileId(null)
    set({ documents: [], selectedDocumentId: null, error: null })
  },

  selectDocument: (id) => {
    const { selectedDocumentId } = get()
    if (id !== selectedDocumentId) {
      useEditorStore.getState().setCurrentFileId(id)
    }
    set({ selectedDocumentId: id })
  },

  // ============================================================================
  // 兼容层：原有 PDF 功能
  // ============================================================================
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

      if (state.files.length === 0 && newFiles.length > 0) {
        const settingsStore = useSettingsStore.getState()
        if (!settingsStore.settings.output.directory) {
          const defaultDir = getDirectory(newFiles[0].path)
          settingsStore.setOutputDirectory(defaultDir)
        }
      }

      const newSelectedId = state.selectedFileId ?? newFiles[0]?.id ?? null

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
    useEditorStore.getState().clearFileMasks(id)

    set((state) => {
      const files = state.files.filter((f) => f.id !== id)
      const newSelectedId =
        state.selectedFileId === id ? (files[0]?.id ?? null) : state.selectedFileId

      if (newSelectedId !== state.selectedFileId) {
        useEditorStore.getState().setCurrentFileId(newSelectedId)
      }

      return { files, selectedFileId: newSelectedId }
    })
  },

  clearFiles: () => {
    useEditorStore.getState().clearAllMasks()
    useEditorStore.getState().setCurrentFileId(null)

    set({ files: [], selectedFileId: null })
  },

  selectFile: (id) => {
    const { selectedFileId } = get()
    if (id !== selectedFileId) {
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
