//! PP-OCRv5 ONNX Runtime 集成
//!
//! 基于 ONNX Runtime 的 OCR 识别库，使用 PP-OCRv5 模型

mod det;
mod error;
mod preprocess;
mod rec;
mod threading;

pub use det::{TextBox, TextDetector};
pub use error::OcrError;
pub use rec::{RecognitionResult, TextRecognizer};

use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

/// OCR 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// 检测模型路径
    pub det_model_path: String,
    /// 识别模型路径
    pub rec_model_path: String,
    /// 字典文件路径（可选，不提供则使用内置字典）
    pub dict_path: Option<String>,
}

/// OCR 识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// 识别的文字
    pub text: String,
    /// 置信度
    pub confidence: f32,
    /// 边界框 (相对坐标 0-1)
    pub bbox: BBox,
}

/// 边界框
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Paddle OCR 引擎 (PP-OCRv5 ONNX)
pub struct PaddleOcrEngine {
    detector: TextDetector,
    recognizer: TextRecognizer,
}

struct RecSegmentMeta {
    box_idx: usize,
    segment_idx: usize,
}

const DEFAULT_MIN_CONFIDENCE: f32 = 0.0;
const DEFAULT_SHORT_ASCII_MIN_CONFIDENCE: f32 = 0.6;
const DEFAULT_SPLIT_MIN_CONFIDENCE: f32 = 0.6;
const DEFAULT_MAX_SEGMENTS: usize = 6;
const DEFAULT_MAX_BATCH_SIZE: usize = 32;

impl PaddleOcrEngine {
    /// 创建 Paddle OCR 引擎
    pub fn new(config: &OcrConfig) -> Result<Self, OcrError> {
        let det_path = Path::new(&config.det_model_path);
        let rec_path = Path::new(&config.rec_model_path);

        log::info!("[OCR] 加载检测模型: {}", config.det_model_path);
        let detector = TextDetector::new(det_path)?;

        log::info!("[OCR] 加载识别模型: {}", config.rec_model_path);
        let recognizer = if let Some(dict) = &config.dict_path {
            TextRecognizer::new(rec_path, Path::new(dict))?
        } else {
            TextRecognizer::with_builtin_charset(rec_path)?
        };

        log::info!("[OCR] 引擎初始化完成");
        Ok(Self { detector, recognizer })
    }

    /// 识别图片文件中的文字
    pub fn recognize_file(&mut self, image_path: &str) -> Result<Vec<OcrResult>, OcrError> {
        let img = image::open(image_path)
            .map_err(|e| OcrError::ImageProcess(format!("打开图片失败: {}", e)))?;

        self.recognize_image(&img)
    }

    /// 识别图片中的文字
    pub fn recognize_image(&mut self, img: &DynamicImage) -> Result<Vec<OcrResult>, OcrError> {
        let (orig_w, orig_h) = (img.width(), img.height());

        // 1. 文字检测
        let det_start = Instant::now();
        let (det_input, _ratio, _input_w, _input_h) = preprocess::prepare_det_input(img);
        let boxes = self.detector.detect(det_input, orig_w, orig_h)?;
        log::info!("[OCR] 检测耗时: {} ms", det_start.elapsed().as_millis());

        log::info!("[OCR] 检测到 {} 个文字区域", boxes.len());

        if boxes.is_empty() {
            return Ok(Vec::new());
        }

        let rec_start = Instant::now();

        let split_ratio = env_f32("LINCH_OCR_SPLIT_RATIO", preprocess::REC_SPLIT_MAX_RATIO);
        let split_min_conf = env_f32("LINCH_OCR_SPLIT_MIN_CONF", DEFAULT_SPLIT_MIN_CONFIDENCE);
        let max_segments = env_usize("LINCH_OCR_MAX_SEGMENTS", DEFAULT_MAX_SEGMENTS);

        // 2. 先对整行做一次识别，避免默认分段导致推理次数暴增
        let mut cropped_images: Vec<DynamicImage> = Vec::with_capacity(boxes.len());
        let mut box_ratios: Vec<f32> = Vec::with_capacity(boxes.len());
        for text_box in boxes.iter() {
            let cropped = preprocess::crop_text_region(img, &text_box.points);
            let ratio = if cropped.height() > 0 {
                cropped.width() as f32 / cropped.height() as f32
            } else {
                0.0
            };
            cropped_images.push(cropped);
            box_ratios.push(ratio);
        }

        let base_batch = DEFAULT_MAX_BATCH_SIZE.min(cropped_images.len().max(1));
        let base_results = run_rec_batches(&mut self.recognizer, &cropped_images, base_batch)?;

        // 3. 只对低置信度的长行做分段识别
        let mut segment_meta: Vec<RecSegmentMeta> = Vec::new();
        let mut segment_images: Vec<DynamicImage> = Vec::new();
        let mut split_boxes = 0usize;
        let splitting_enabled = split_ratio > 0.0 && max_segments > 1;

        if splitting_enabled {
            for (box_idx, cropped) in cropped_images.iter().enumerate() {
                let ratio = box_ratios[box_idx];
                let base_conf = base_results[box_idx].confidence;
                if ratio > split_ratio && base_conf < split_min_conf {
                    let segments = preprocess::split_long_text_region(cropped, split_ratio, max_segments);
                    if segments.len() > 1 {
                        split_boxes += 1;
                        for (segment_idx, segment) in segments.into_iter().enumerate() {
                            segment_meta.push(RecSegmentMeta { box_idx, segment_idx });
                            segment_images.push(segment);
                        }
                    }
                }
            }
        }

        if !segment_images.is_empty() {
            log::info!(
                "[OCR] 分段识别: {} 个区域需要拆分，{} 个子段",
                split_boxes,
                segment_images.len()
            );
        }

        let mut segments_by_box: Vec<Vec<(usize, RecognitionResult)>> = vec![Vec::new(); boxes.len()];
        if !segment_images.is_empty() {
            let split_batch = DEFAULT_MAX_BATCH_SIZE.min(segment_images.len().max(1));
            let split_results = run_rec_batches(&mut self.recognizer, &segment_images, split_batch)?;

            for (idx, rec_result) in split_results.into_iter().enumerate() {
                if idx >= segment_meta.len() {
                    break;
                }
                let meta = &segment_meta[idx];
                segments_by_box[meta.box_idx].push((meta.segment_idx, rec_result));
            }
        }

        let mut results = Vec::with_capacity(boxes.len());
        for (box_idx, mut segments) in segments_by_box.into_iter().enumerate() {
            let base = &base_results[box_idx];
            let mut final_text = base.text.clone();
            let mut final_conf = base.confidence;

            if !segments.is_empty() {
                segments.sort_by_key(|(idx, _)| *idx);

                let mut text_parts: Vec<String> = Vec::new();
                let mut conf_sum = 0.0f32;
                let mut conf_count = 0u32;

                for (_, rec_result) in segments.into_iter() {
                    if !should_keep_segment(&rec_result.text, rec_result.confidence) {
                        continue;
                    }
                    text_parts.push(rec_result.text);
                    conf_sum += rec_result.confidence;
                    conf_count += 1;
                }

                if !text_parts.is_empty() {
                    let split_text = merge_text_parts(&text_parts);
                    let split_conf = if conf_count > 0 {
                        conf_sum / conf_count as f32
                    } else {
                        0.0
                    };
                    let split_len = split_text.chars().count();
                    let base_len = final_text.chars().count();
                    if split_len > base_len || split_conf >= final_conf {
                        final_text = split_text;
                        final_conf = split_conf;
                    }
                }
            }

            if final_text.trim().is_empty() {
                continue;
            }

            let bbox = points_to_bbox(&boxes[box_idx].points, orig_w, orig_h);
            log::debug!("[OCR] 区域 {}: \"{}\" (置信度: {:.2})", box_idx, final_text, final_conf);
            results.push(OcrResult {
                text: final_text,
                confidence: final_conf,
                bbox,
            });
        }

        log::info!("[OCR] 识别耗时: {} ms", rec_start.elapsed().as_millis());
        log::info!("[OCR] 识别完成，共 {} 个结果", results.len());
        Ok(results)
    }

    /// 只提取文字（不返回位置信息）
    pub fn extract_text(&mut self, image_path: &str) -> Result<String, OcrError> {
        let results = self.recognize_file(image_path)?;
        let text: String = results.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join(" ");
        Ok(text)
    }
}

/// 将四个角点转换为边界框（相对坐标）
fn points_to_bbox(points: &[[f32; 2]; 4], img_w: u32, img_h: u32) -> BBox {
    let min_x = points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
    let max_y = points.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);

    BBox {
        x: min_x / img_w as f32,
        y: min_y / img_h as f32,
        w: (max_x - min_x) / img_w as f32,
        h: (max_y - min_y) / img_h as f32,
    }
}

fn merge_text_parts(parts: &[String]) -> String {
    let mut merged = String::new();
    for part in parts {
        if merged.is_empty() {
            merged.push_str(part);
            continue;
        }
        let overlap = longest_overlap(&merged, part, 16);
        let suffix: String = part.chars().skip(overlap).collect();
        if needs_ascii_gap(&merged, &suffix) {
            merged.push(' ');
        }
        merged.push_str(&suffix);
    }
    merged
}

fn longest_overlap(left: &str, right: &str, max_len: usize) -> usize {
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    let max_len = max_len
        .min(left_chars.len())
        .min(right_chars.len());

    for len in (1..=max_len).rev() {
        if left_chars[left_chars.len() - len..] == right_chars[..len] {
            return len;
        }
    }
    0
}

fn should_keep_segment(text: &str, confidence: f32) -> bool {
    if text.is_empty() {
        return false;
    }

    let min_conf = std::env::var("LINCH_OCR_MIN_CONF")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(DEFAULT_MIN_CONFIDENCE);
    if confidence < min_conf {
        return false;
    }

    let char_count = text.chars().count();
    let is_ascii_letters = text.chars().all(|c| c.is_ascii_alphabetic());
    if char_count <= 2 && is_ascii_letters {
        let short_min_conf = std::env::var("LINCH_OCR_MIN_CONF_ASCII_SHORT")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(DEFAULT_SHORT_ASCII_MIN_CONFIDENCE);
        if confidence < short_min_conf {
            return false;
        }
    }

    true
}

fn needs_ascii_gap(left: &str, right: &str) -> bool {
    let left_char = left.chars().rev().find(|c| !c.is_whitespace());
    let right_char = right.chars().find(|c| !c.is_whitespace());
    match (left_char, right_char) {
        (Some(l), Some(r)) => {
            if l.is_ascii_digit() && r.is_ascii_digit() {
                return false;
            }
            l.is_ascii_alphanumeric() && r.is_ascii_alphanumeric()
        }
        _ => false,
    }
}

fn env_f32(key: &str, default: f32) -> f32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn run_rec_batches(
    recognizer: &mut TextRecognizer,
    images: &[DynamicImage],
    batch_size: usize,
) -> Result<Vec<RecognitionResult>, OcrError> {
    let mut results = Vec::with_capacity(images.len());
    let batch_input = preprocess::prepare_rec_batch(images, batch_size.max(1));

    for batch in batch_input.into_iter() {
        let batch_len = batch.shape()[0];
        match recognizer.recognize_batch(batch) {
            Ok(rec_results) => {
                results.extend(rec_results);
            }
            Err(e) => {
                log::warn!("[OCR] 批量识别失败: {}", e);
                for _ in 0..batch_len {
                    results.push(RecognitionResult {
                        text: String::new(),
                        confidence: 0.0,
                    });
                }
            }
        }
    }

    Ok(results)
}

/// 检查 OCR 模型是否已安装
pub fn is_models_installed(config: &OcrConfig) -> bool {
    let det_exists = Path::new(&config.det_model_path).exists();
    let rec_exists = Path::new(&config.rec_model_path).exists();
    det_exists && rec_exists
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_points_to_bbox() {
        let points = [
            [10.0, 20.0],
            [100.0, 20.0],
            [100.0, 50.0],
            [10.0, 50.0],
        ];
        let bbox = points_to_bbox(&points, 200, 100);
        assert!((bbox.x - 0.05).abs() < 0.001);
        assert!((bbox.y - 0.2).abs() < 0.001);
        assert!((bbox.w - 0.45).abs() < 0.001);
        assert!((bbox.h - 0.3).abs() < 0.001);
    }
}
