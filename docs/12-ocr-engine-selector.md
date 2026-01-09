Title: OCR engine selector (Paddle ONNX + Tesseract CLI)

Goals
- Provide two OCR engines in parallel: current PP-OCRv5 (ONNX) and Tesseract CLI.
- Let users choose engine per scan while staying offline-first.
- Keep outputs unified for rules + auditability.
- Avoid tight FFI bindings; prefer CLI wrappers and replaceable integrations.

Non-goals
- No auto-download without explicit user action.
- No online OCR services.
- No model training or custom language packs bundled by default.

Engine options
1) Paddle (current)
   - Local ONNX models + built-in charset.
   - Fast on detection + recognition with bbox output.
2) Tesseract (new)
   - External CLI binary + tessdata (language packs).
   - Single-pass OCR per page (no separate detector).
   - TSV output parsed into bboxes and text items.

Unified OCR output (existing)
{
  "page": 0,
  "items": [
    {"text":"...", "confidence":0.92, "bbox":{"x":0.12,"y":0.34,"w":0.18,"h":0.02}}
  ]
}

Proposed config
{
  "ocr_engine": "paddle" | "tesseract",
  "paddle": { "det_model": "...", "rec_model": "...", "dict": "..." },
  "tesseract": {
    "binary_path": "...",
    "tessdata_path": "...",
    "lang": "chi_sim+eng",
    "psm": 6,
    "oem": 1
  }
}

UI/UX
- Add OCR engine dropdown (Paddle / Tesseract).
- Status pill per engine: installed / missing.
- For Tesseract: path picker + language picker (from tessdata).
- For Paddle: existing model path status.

Tesseract CLI invocation
- Render page to PNG (existing pipeline).
- Command example:
  tesseract <image> stdout -l chi_sim+eng --psm 6 --oem 1 tsv
- Parse TSV rows to word-level bboxes and confidence.
- Group words into line items (same line_num) to reduce fragmentation.
- Convert absolute pixels to relative bbox for unified output.

Performance considerations
- One invocation per page; no per-line detector pass.
- Configurable thread limits via environment if needed (OMP/TESSDATA).
- Use page-level timing logs similar to Paddle.
- Optional DPI override (shared with Paddle).

Audit requirements
- Record engine type, version, args (lang/psm/oem), tessdata hash.
- Record input/output, rule pack version, OCR mode, cleaning steps (existing).

Cross-platform install
- Provide "Install OCR engine" wizard for Tesseract like Paddle.
- Support offline import of a prebuilt bundle:
  tesseract-{os}-{arch}.zip + tessdata-{version}.zip
- Store in app data directory; record path + hash.
- Optional online download can be added later with explicit user action.

Integration steps (code)
1) Introduce OcrEngine enum and trait (recognize_image -> Vec<OcrResult>).
2) Move current Paddle ONNX into PaddleEngine implementation.
3) Add TesseractEngine that spawns CLI and parses TSV.
4) Extend detection pipeline to choose engine by config.
5) Add UI selector + config persistence.
6) Extend audit record with engine metadata.

Implementation status: DONE (2025-01-08)

Backend changes:
- src-tauri/src/ocr/mod.rs: Main OCR module with unified API
- src-tauri/src/ocr/types.rs: Shared types (OcrEngineType, TesseractConfig, etc.)
- src-tauri/src/ocr/engine.rs: OcrEngine trait definition
- src-tauri/src/ocr/paddle.rs: Paddle OCR implementation
- src-tauri/src/ocr/tesseract.rs: Tesseract CLI wrapper
- crates/ocr/src/lib.rs: Renamed OcrEngine -> PaddleOcrEngine
- src-tauri/src/config.rs: Added ocrEngine and tesseract fields

New Tauri commands:
- get_ocr_engine_status: Returns status of both engines
- set_ocr_engine: Switch between paddle/tesseract
- get_current_ocr_engine: Get current engine type
- init_paddle_ocr / install_paddle_ocr / is_paddle_ocr_installed
- init_tesseract_ocr / check_tesseract_status / save_tesseract_config
- get_tesseract_languages: List available language packs
- ocr_recognize: Unified recognition using current engine
- get_ocr_audit_info: Get audit info for current engine

Frontend changes:
- src/types/index.ts: Added OcrEngineType, TesseractConfig, OcrEngineStatus
- src/lib/tauri/ocr.ts: New API bindings
- src/stores/useOcrStore.ts: Engine selection state management
- src/components/features/ocr/OcrSetupDialog.tsx: Tab-based engine selector

Config format (implemented):
{
  "ocrEngine": "paddle" | "tesseract",
  "detModelPath": "...",
  "recModelPath": "...",
  "tesseract": {
    "binaryPath": "...",
    "tessdataPath": "...",
    "lang": "chi_sim+eng",
    "psm": 6,
    "oem": 1
  }
}

Open questions
- Default Tesseract PSM for scanned PDFs (using 6 by default - single uniform block).
- Language pack defaults per locale (using chi_sim+eng for Chinese users).
- Whether to include a lightweight offline bundle in releases (deferred).
