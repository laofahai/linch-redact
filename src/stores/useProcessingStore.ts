import { create } from "zustand"
import type { ProcessingTask } from "@/types"

interface ProcessingStore {
  tasks: ProcessingTask[]
  isRunning: boolean

  // 操作
  startProcessing: (fileIds: string[]) => void
  pauseProcessing: () => void
  resumeProcessing: () => void
  cancelTask: (fileId: string) => void
  retryTask: (fileId: string) => void
  updateTaskProgress: (fileId: string, progress: number, step: string) => void
  setTaskStatus: (fileId: string, status: ProcessingTask["status"], error?: string) => void
  clearTasks: () => void
}

export const useProcessingStore = create<ProcessingStore>((set) => ({
  tasks: [],
  isRunning: false,

  startProcessing: (fileIds) => {
    const tasks: ProcessingTask[] = fileIds.map((fileId) => ({
      fileId,
      status: "queued",
      progress: 0,
      currentStep: "等待中",
    }))
    set({ tasks, isRunning: true })
  },

  pauseProcessing: () => {
    set((state) => ({
      isRunning: false,
      tasks: state.tasks.map((t) =>
        t.status === "running" ? { ...t, status: "paused" as const } : t
      ),
    }))
  },

  resumeProcessing: () => {
    set((state) => ({
      isRunning: true,
      tasks: state.tasks.map((t) =>
        t.status === "paused" ? { ...t, status: "queued" as const } : t
      ),
    }))
  },

  cancelTask: (fileId) => {
    set((state) => ({
      tasks: state.tasks.filter((t) => t.fileId !== fileId),
    }))
  },

  retryTask: (fileId) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.fileId === fileId
          ? { ...t, status: "queued" as const, progress: 0, error: undefined }
          : t
      ),
    }))
  },

  updateTaskProgress: (fileId, progress, step) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.fileId === fileId
          ? { ...t, progress, currentStep: step, status: "running" as const }
          : t
      ),
    }))
  },

  setTaskStatus: (fileId, status, error) => {
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.fileId === fileId ? { ...t, status, error } : t
      ),
    }))
  },

  clearTasks: () => {
    set({ tasks: [], isRunning: false })
  },
}))
