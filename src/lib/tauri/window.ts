import { getCurrentWindow } from "@tauri-apps/api/window"

const appWindow = getCurrentWindow()

export async function minimizeWindow() {
  await appWindow.minimize()
}

export async function toggleMaximize() {
  await appWindow.toggleMaximize()
}

export async function closeWindow() {
  await appWindow.close()
}

export async function isMaximized() {
  return appWindow.isMaximized()
}

export async function startDragging() {
  await appWindow.startDragging()
}
