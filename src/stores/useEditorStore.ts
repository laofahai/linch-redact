import { create } from "zustand"
import { nanoid } from "nanoid"
import type { Mask, MasksByPage } from "@/types"

interface DrawingState {
  startX: number
  startY: number
  currentX: number
  currentY: number
}

interface EditorStore {
  currentPage: number
  masksByPage: MasksByPage
  drawing: DrawingState | null
  zoom: number
  selectedMaskId: string | null

  // 计算属性
  getCurrentMasks: () => Mask[]

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
  addMask: (page: number, mask: Mask) => void
  removeMask: (page: number, maskId: string) => void
  clearPageMasks: (page: number) => void
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
  masksByPage: {},
  drawing: null,
  zoom: 1,
  selectedMaskId: null,

  getCurrentMasks: () => {
    const { masksByPage, currentPage } = get()
    return masksByPage[currentPage] ?? []
  },

  setCurrentPage: (page) => {
    set({ currentPage: page, drawing: null })
  },

  nextPage: (maxPage) => {
    set((state) => ({
      currentPage: Math.min(state.currentPage + 1, maxPage - 1),
      drawing: null,
    }))
  },

  prevPage: () => {
    set((state) => ({
      currentPage: Math.max(state.currentPage - 1, 0),
      drawing: null,
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
    const { drawing, currentPage } = get()
    if (!drawing) return

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

    set((state) => ({
      drawing: null,
      masksByPage: {
        ...state.masksByPage,
        [currentPage]: [...(state.masksByPage[currentPage] ?? []), mask],
      },
    }))
  },

  cancelDrawing: () => {
    set({ drawing: null })
  },

  addMask: (page, mask) => {
    set((state) => ({
      masksByPage: {
        ...state.masksByPage,
        [page]: [...(state.masksByPage[page] ?? []), mask],
      },
    }))
  },

  removeMask: (page, maskId) => {
    set((state) => ({
      masksByPage: {
        ...state.masksByPage,
        [page]: (state.masksByPage[page] ?? []).filter((m) => m.id !== maskId),
      },
    }))
  },

  clearPageMasks: (page) => {
    set((state) => {
      const { [page]: _, ...rest } = state.masksByPage
      return { masksByPage: rest }
    })
  },

  clearAllMasks: () => {
    set({ masksByPage: {}, selectedMaskId: null })
  },

  selectMask: (maskId) => {
    set({ selectedMaskId: maskId })
  },

  resizeMask: (page, maskId, newBounds) => {
    set((state) => {
      const masks = state.masksByPage[page] ?? []
      const updatedMasks = masks.map((m) =>
        m.id === maskId ? { ...m, ...newBounds } : m
      )
      return {
        masksByPage: {
          ...state.masksByPage,
          [page]: updatedMasks,
        },
      }
    })
  },

  deleteSelectedMask: () => {
    const { selectedMaskId, currentPage, masksByPage } = get()
    if (!selectedMaskId) return

    const masks = masksByPage[currentPage] ?? []
    const updatedMasks = masks.filter((m) => m.id !== selectedMaskId)

    set({
      selectedMaskId: null,
      masksByPage: {
        ...masksByPage,
        [currentPage]: updatedMasks,
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
