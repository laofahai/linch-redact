# Task Breakdown

## UI (Tauri)
- File queue (drag/drop + folder import).
- Rule template manager + switches.
- OCR mode selector.
- Preview pane + hit list.
- Output config + progress log.

## Render
- PDF -> page images (for OCR + preview).

## OCR
- PaddleOCR CLI wrapper; unified JSON output.
- Engine lifecycle and concurrency control.

## Rules
- Schema, parsing, matchers (keyword/regex/dict).
- Page scoping (list/range).

## Redaction
- BBox merge + padding.
- Overlay generator + flatten.
- Optional object delete (later).

## Cleaning
- Metadata/XMP removal.
- Annots, forms, attachments, JS.

## Verify
- Post text search on output.
- OCR sampling check.

## Audit/Report
- audit.json + report.html + evidence.zip.

## Packaging
- Model pack management.
- Engine checks.
- Installers.
