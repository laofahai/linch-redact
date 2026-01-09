import type { OcrManifest } from "@/types"

// PP-OCRv5 ONNX 模型配置
// 模型来源: https://huggingface.co/monkt/paddleocr-onnx
export const ocrManifest: OcrManifest = {
  version: "1.0",
  // 使用 ONNX Runtime，无需单独下载引擎
  engine: null,
  models: {
    ppocr_v5: {
      name: "PP-OCRv5",
      description: "PaddleOCR v5 模型，支持中英文识别",
      files: {
        det: {
          // HuggingFace 源
          url: "https://huggingface.co/monkt/paddleocr-onnx/resolve/main/detection/v5/det.onnx",
          // 中国镜像源
          mirrorUrl: "https://hf-mirror.com/monkt/paddleocr-onnx/resolve/main/detection/v5/det.onnx",
          filename: "det.onnx",
          size: 88000000, // 88 MB
        },
        rec: {
          url: "https://huggingface.co/monkt/paddleocr-onnx/resolve/main/languages/chinese/rec.onnx",
          mirrorUrl: "https://hf-mirror.com/monkt/paddleocr-onnx/resolve/main/languages/chinese/rec.onnx",
          filename: "rec.onnx",
          size: 84500000, // 84.5 MB
        },
      },
      version: "5.0",
    },
  },
}
