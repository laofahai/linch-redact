Title: OCR line segmentation and crop padding

Problem
- Long single-line regions are squeezed to the fixed recognition width (320px), which drops or smears trailing digits.
- Tight detection boxes can cut off edge glyphs (phone numbers, suffixes).

Approach
- Add a small crop padding ratio around each detected text box before recognition.
- Run a single-pass line recognition first to avoid excessive segmentation work.
- Only split long, low-confidence lines into horizontal segments and merge them with overlap-aware stitching.
- Keep render DPI configurable and avoid keyword heuristics to preserve offline-first behavior.

Notes
- This is recognition-time only; detection remains unchanged.
- BBoxes remain the original detection box for auditability and consistency.
- Optional confidence filters can suppress short low-confidence ASCII noise.

Tuning knobs (env)
- LINCH_OCR_DPI: override render DPI (default 150).
- LINCH_OCR_SPLIT_RATIO: line width/height ratio threshold for splitting (default REC_IMAGE_WIDTH / REC_IMAGE_HEIGHT).
- LINCH_OCR_SPLIT_MIN_CONF: only split when base line confidence is below this value (default 0.6).
- LINCH_OCR_MAX_SEGMENTS: cap segment count for a line (default 6; set 0 to disable splitting).

Open questions
- Confirm overlap ratio and max overlap length if we see repeated characters in merged text.
- Decide if we should expose these as config in the UI later.
