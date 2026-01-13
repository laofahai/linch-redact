import { create } from "zustand"
import { nanoid } from "nanoid"
import type { Mask, MasksByPage, MasksByFile } from "@/types"

interface DrawingState {
  startX: number
  startY: number
  currentX: number
  currentY: number
}

interface EditorStore {
  currentPage: number
  currentFileId: string | null
  masksByFile: MasksByFile
  drawing: DrawingState | null
  zoom: number
  selectedMaskId: string | null

  // 计算属性
  getCurrentMasks: () => Mask[]
  getMasksByPage: (fileId: string) => MasksByPage

  // 文件切换
  setCurrentFileId: (fileId: string | null) => void

  // 页面导航
  setCurrentPage: (page: number) => void
  nextPage: (maxPage: number) => void
  prevPage: () => void

  // 绘制操作
  startDrawing: (x: number, y: number) => void
  updateDrawing: (x: number, y: number) => void
  finishDrawing: () => void
  cancelDrawing: () => void

  // 遮盖框管理
  addMask: (page: number, mask: Mask, fileId?: string) => void
  removeMask: (page: number, maskId: string, fileId?: string) => void
  clearPageMasks: (page: number) => void
  clearFileMasks: (fileId: string) => void
  clearAllMasks: () => void

  // 遮盖框选择和调整
  selectMask: (maskId: string | null) => void
  resizeMask: (page: number, maskId: string, newBounds: Partial<Mask>) => void
  deleteSelectedMask: () => void

  // 缩放
  setZoom: (zoom: number) => void
  zoomIn: () => void
  zoomOut: () => void
  resetZoom: () => void
}

export const useEditorStore = create<EditorStore>((set, get) => ({
  currentPage: 0,
  currentFileId: null,
  masksByFile: {},
  drawing: null,
  zoom: 1,
  selectedMaskId: null,

  getCurrentMasks: () => {
    const { masksByFile, currentFileId, currentPage } = get()
    if (!currentFileId) return []
    return masksByFile[currentFileId]?.[currentPage] ?? []
  },

  getMasksByPage: (fileId: string) => {
    const { masksByFile } = get()
    return masksByFile[fileId] ?? {}
  },

  setCurrentFileId: (fileId) => {
    set({ currentFileId: fileId, currentPage: 0, drawing: null, selectedMaskId: null })
  },

  setCurrentPage: (page) => {
    set({ currentPage: page, drawing: null, selectedMaskId: null })
  },

  nextPage: (maxPage) => {
    set((state) => ({
      currentPage: Math.min(state.currentPage + 1, maxPage - 1),
      drawing: null,
      selectedMaskId: null,
    }))
  },

  prevPage: () => {
    set((state) => ({
      currentPage: Math.max(state.currentPage - 1, 0),
      drawing: null,
      selectedMaskId: null,
    }))
  },

  startDrawing: (x, y) => {
    set({
      drawing: { startX: x, startY: y, currentX: x, currentY: y },
    })
  },

  updateDrawing: (x, y) => {
    set((state) => {
      if (!state.drawing) return state
      return {
        drawing: { ...state.drawing, currentX: x, currentY: y },
      }
    })
  },

  finishDrawing: () => {
    const { drawing, currentPage, currentFileId } = get()
    if (!drawing || !currentFileId) return

    const x = Math.min(drawing.startX, drawing.currentX)
    const y = Math.min(drawing.startY, drawing.currentY)
    const width = Math.abs(drawing.currentX - drawing.startX)
    const height = Math.abs(drawing.currentY - drawing.startY)

    // 忽略太小的遮盖框
    if (width < 0.01 || height < 0.01) {
      set({ drawing: null })
      return
    }

    const mask: Mask = {
      id: nanoid(),
      x,
      y,
      width,
      height,
    }

    set((state) => {
      const fileMasks = state.masksByFile[currentFileId] ?? {}
      const pageMasks = fileMasks[currentPage] ?? []
      return {
        drawing: null,
        masksByFile: {
          ...state.masksByFile,
          [currentFileId]: {
            ...fileMasks,
            [currentPage]: [...pageMasks, mask],
          },
        },
      }
    })
  },

  cancelDrawing: () => {
    set({ drawing: null })
  },

  addMask: (page, mask, fileId) => {
    const targetFileId = fileId ?? get().currentFileId
    if (!targetFileId) return

    set((state) => {
      const fileMasks = state.masksByFile[targetFileId] ?? {}
      const pageMasks = fileMasks[page] ?? []
      return {
        masksByFile: {
          ...state.masksByFile,
          [targetFileId]: {
            ...fileMasks,
            [page]: [...pageMasks, mask],
          },
        },
      }
    })
  },

  removeMask: (page, maskId, fileId) => {
    const targetFileId = fileId ?? get().currentFileId
    if (!targetFileId) return

    set((state) => {
      const fileMasks = state.masksByFile[targetFileId] ?? {}
      const pageMasks = fileMasks[page] ?? []
      return {
        masksByFile: {
          ...state.masksByFile,
          [targetFileId]: {
            ...fileMasks,
            [page]: pageMasks.filter((m) => m.id !== maskId),
          },
        },
      }
    })
  },

  clearPageMasks: (page) => {
    const { currentFileId } = get()
    if (!currentFileId) return

    set((state) => {
      const fileMasks = { ...state.masksByFile[currentFileId] }
      delete fileMasks[page]
      return {
        masksByFile: {
          ...state.masksByFile,
          [currentFileId]: fileMasks,
        },
      }
    })
  },

  clearFileMasks: (fileId) => {
    set((state) => {
      const newMasks = { ...state.masksByFile }
      delete newMasks[fileId]
      return { masksByFile: newMasks, selectedMaskId: null }
    })
  },

  clearAllMasks: () => {
    set({ masksByFile: {}, selectedMaskId: null })
  },

  selectMask: (maskId) => {
    set({ selectedMaskId: maskId })
  },

  resizeMask: (page, maskId, newBounds) => {
    const { currentFileId } = get()
    if (!currentFileId) return

    set((state) => {
      const fileMasks = state.masksByFile[currentFileId] ?? {}
      const pageMasks = fileMasks[page] ?? []
      const updatedMasks = pageMasks.map((m) => (m.id === maskId ? { ...m, ...newBounds } : m))
      return {
        masksByFile: {
          ...state.masksByFile,
          [currentFileId]: {
            ...fileMasks,
            [page]: updatedMasks,
          },
        },
      }
    })
  },

  deleteSelectedMask: () => {
    const { selectedMaskId, currentPage, currentFileId, masksByFile } = get()
    if (!selectedMaskId || !currentFileId) return

    const fileMasks = masksByFile[currentFileId] ?? {}
    const pageMasks = fileMasks[currentPage] ?? []
    const updatedMasks = pageMasks.filter((m) => m.id !== selectedMaskId)

    set({
      selectedMaskId: null,
      masksByFile: {
        ...masksByFile,
        [currentFileId]: {
          ...fileMasks,
          [currentPage]: updatedMasks,
        },
      },
    })
  },

  setZoom: (zoom) => {
    set({ zoom: Math.max(0.25, Math.min(3, zoom)) })
  },

  zoomIn: () => {
    set((state) => ({ zoom: Math.min(state.zoom + 0.25, 3) }))
  },

  zoomOut: () => {
    set((state) => ({ zoom: Math.max(state.zoom - 0.25, 0.25) }))
  },

  resetZoom: () => {
    set({ zoom: 1 })
  },
}))
