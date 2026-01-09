import * as pdfjsLib from "pdfjs-dist"
import { convertFileSrc } from "@tauri-apps/api/core"

// 设置 worker
pdfjsLib.GlobalWorkerOptions.workerSrc = `https://cdnjs.cloudflare.com/ajax/libs/pdf.js/${pdfjsLib.version}/pdf.worker.min.mjs`

export interface PdfDocument {
  numPages: number
  getPage: (pageNum: number) => Promise<pdfjsLib.PDFPageProxy>
}

interface RenderTask {
  promise: Promise<void>
  cancel: () => void
}

const pdfCache = new Map<string, pdfjsLib.PDFDocumentProxy>()
let currentRenderTask: RenderTask | null = null

export async function loadPdf(filePath: string): Promise<PdfDocument> {
  if (pdfCache.has(filePath)) {
    const pdf = pdfCache.get(filePath)!
    return {
      numPages: pdf.numPages,
      getPage: (pageNum: number) => pdf.getPage(pageNum),
    }
  }

  // 转换本地文件路径为 Tauri asset URL
  const url = convertFileSrc(filePath)

  const loadingTask = pdfjsLib.getDocument({
    url,
    cMapUrl: `https://cdnjs.cloudflare.com/ajax/libs/pdf.js/${pdfjsLib.version}/cmaps/`,
    cMapPacked: true,
  })
  const pdf = await loadingTask.promise

  pdfCache.set(filePath, pdf)

  return {
    numPages: pdf.numPages,
    getPage: (pageNum: number) => pdf.getPage(pageNum),
  }
}

export async function renderPageToCanvas(
  page: pdfjsLib.PDFPageProxy,
  canvas: HTMLCanvasElement
): Promise<{ width: number; height: number }> {
  // 取消之前的渲染任务
  if (currentRenderTask) {
    currentRenderTask.cancel()
    currentRenderTask = null
  }

  const dpr = window.devicePixelRatio || 1
  // 使用更高的渲染比例确保清晰度
  // 基础比例 3.0 确保高清显示
  const baseScale = 3.0
  const renderScale = baseScale * dpr

  const viewport = page.getViewport({ scale: renderScale })
  const context = canvas.getContext("2d", {
    alpha: false, // 禁用透明度，提升性能
  })!

  // 清除画布
  context.clearRect(0, 0, canvas.width, canvas.height)

  // 设置 canvas 的实际像素尺寸
  canvas.width = viewport.width
  canvas.height = viewport.height

  // CSS 显示尺寸 - 除以 baseScale 和 dpr 得到实际显示大小
  const displayWidth = viewport.width / (baseScale * dpr)
  const displayHeight = viewport.height / (baseScale * dpr)
  canvas.style.width = `${displayWidth}px`
  canvas.style.height = `${displayHeight}px`

  // 设置白色背景
  context.fillStyle = "white"
  context.fillRect(0, 0, canvas.width, canvas.height)

  const renderContext = {
    canvasContext: context,
    viewport: viewport,
    background: "white",
  }

  const renderTask = page.render(renderContext)
  currentRenderTask = {
    promise: renderTask.promise,
    cancel: () => renderTask.cancel(),
  }

  try {
    await renderTask.promise
  } catch (e: any) {
    // 忽略取消错误
    if (e?.name === "RenderingCancelledException") {
      return { width: displayWidth, height: displayHeight }
    }
    throw e
  } finally {
    currentRenderTask = null
  }

  return { width: displayWidth, height: displayHeight }
}

export function clearPdfCache(filePath?: string) {
  if (filePath) {
    const pdf = pdfCache.get(filePath)
    pdf?.destroy()
    pdfCache.delete(filePath)
  } else {
    pdfCache.forEach((pdf) => pdf.destroy())
    pdfCache.clear()
  }
}
