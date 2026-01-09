import { create } from "zustand"

interface SettingsDialogStore {
  isOpen: boolean
  openDialog: () => void
  closeDialog: () => void
  toggleDialog: () => void
}

export const useSettingsDialogStore = create<SettingsDialogStore>((set) => ({
  isOpen: false,
  openDialog: () => set({ isOpen: true }),
  closeDialog: () => set({ isOpen: false }),
  toggleDialog: () => set((state) => ({ isOpen: !state.isOpen })),
}))
