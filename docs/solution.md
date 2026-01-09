# PDF Redaction Tool - Full Solution Notes

## Scope
A local, offline-first desktop tool to create an irreversible redacted PDF copy, with audit trail.

### Non-goals
- Not a full PDF editor (no layout/annotation/font editing).
- Not an online service.
- Not 100% object-level removal for all PDFs; cover+flatten is the safety net.

## Core Guarantees
- Non-reversible output (text layer, OCR layer, metadata, attachments, annotations).
- Never overwrite source files (UI and core enforcement).
- Auditable output (audit.json + report).

## Features
### Files & Tasks
- Single/batch import via drag/drop or folder import.
- Task queue: start/pause/resume/retry.
- Output naming: `__redacted` or `__external__YYYYMMDD`.
- Task templates: rules + OCR mode + cleaning switches.

### Rules
Types: keyword, regex, dictionary, region template, page rule.
Scope: global, page list, page range.
Actions: redact_text, redact_region, remove_page, keep_only_pages.
Versioning: rule pack version/hash/signature (V2).

### Redaction Engine
PDF types: searchable, scanned, mixed.
- Text rules: map to page + bbox.
- OCR rules: OCR bbox for scanned/mixed.
- Merge hits: per page, bbox merge + padding.
- Apply: overlay coverage + flatten (default); optional object delete.

### OCR Modes
- Detect: OCR report only.
- Clear: cover OCR hits + remove text layer + flatten (default).
- Rebuild: cover OCR hits + rebuild text layer (V2).

### Cleaning
- Document Info + XMP metadata.
- Annotations, AcroForm fields, embedded files.
- JavaScript/actions, optional content groups (best-effort).
- Rewrite/repack to reduce residue.

### Verify & Preview
- Before/after preview rendering.
- Hit list: rule, page, snippet, confidence, bbox.
- Post-process text search and optional OCR sampling.

### Audit & Evidence
Outputs: `audit.json`, `report.html`, optional `evidence.zip`.
Audit includes operator (optional), time, input/output sha256, rule pack hash/version, OCR mode, cleaning flags, hits, and result status.

## Architecture
Workspace layout:
- `src/` (React UI)
- `src-tauri/` (Tauri backend)
- `crates/core` (task orchestration, audit)
- `crates/rules` (schema + matchers)
- `crates/ocr` (PaddleOCR CLI/service wrapper)
- `crates/pdf` (cleaning + overlay + flatten)
- `crates/render` (PDF -> images for OCR/preview)
- `crates/verify` (post-checks)

## Naming
- Desktop bundle identifier: `redact.linch.tech`
- Product display name: `Linch · 文档脱敏器`

## PaddleOCR Integration
Prefer CLI in V1; optional HTTP service in V2.
Unified OCR output schema:
```
{
  "page": 1,
  "items": [
    {"text":"...","confidence":0.96,"bbox":{"x":120,"y":340,"w":280,"h":36}}
  ]
}
```

### Setup Wizard
See `docs/paddle-setup.md` for the engine/model distribution plan and first-run wizard.

## Pipeline (V1)
1) Open PDF read-only.
2) Pre-scan structure for attachments/forms/annots.
3) Render pages to images.
4) OCR (Detect/Clear/Rebuild).
5) Rules match (text + OCR).
6) Merge hits; expand bbox.
7) Apply overlay + flatten.
8) Clean layers (metadata/xmp/annots/forms/attachments/js).
9) Rewrite output PDF.
10) Verify (text search + optional OCR sampling).
11) Emit outputs (redacted PDF + audit/report).

## Engine Strategy
V1: use stable external tools (qpdf) for rewrite/cleanup, Rust for orchestration.
V2: gradually replace with pure Rust (lopdf).

## Risks
- PDF text extraction failure -> OCR first + cover+flatten default.
- Recoverable overlays -> always flatten + rewrite.
- OCR misses -> bbox padding + sampling OCR re-check + manual box tool.
- Performance -> OCR concurrency limits, caching.

## Milestones
- V1: batch queue, Detect/Clear, overlay+flatten, full cleaning, audit/report, verification.
- V2: OCR server, Rebuild, signed rule packs.
- V3: enterprise flow (operators, approvals, integrations).
