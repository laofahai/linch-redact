//! 文字识别模块
//!
//! 使用 CRNN 模型识别检测到的文字区域

use ndarray::Array4;
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;

use crate::error::OcrError;
use crate::threading::apply_session_threads;

/// 文字识别器
pub struct TextRecognizer {
    session: Session,
    charset: Vec<String>,
}

/// 识别结果
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    pub text: String,
    pub confidence: f32,
}

impl TextRecognizer {
    /// 从 ONNX 模型文件创建识别器
    pub fn new(model_path: &Path, dict_path: &Path) -> Result<Self, OcrError> {
        let builder = Session::builder()
            .map_err(|e: ort::Error| OcrError::ModelLoad(e.to_string()))?;
        let builder = apply_session_threads(builder)
            .map_err(|e| OcrError::ModelLoad(e.to_string()))?;
        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| OcrError::ModelLoad(format!("加载识别模型失败: {}", e)))?;

        let charset = load_charset(dict_path)?;
        log::info!("[OCR] 加载字符集: {} 个字符", charset.len());

        Ok(Self { session, charset })
    }

    /// 使用内置字符集创建识别器
    pub fn with_builtin_charset(model_path: &Path) -> Result<Self, OcrError> {
        let builder = Session::builder()
            .map_err(|e: ort::Error| OcrError::ModelLoad(e.to_string()))?;
        let builder = apply_session_threads(builder)
            .map_err(|e| OcrError::ModelLoad(e.to_string()))?;
        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| OcrError::ModelLoad(format!("加载识别模型失败: {}", e)))?;

        let charset = builtin_charset();
        log::info!("[OCR] 使用内置字符集: {} 个字符", charset.len());

        Ok(Self { session, charset })
    }

    /// 识别单张图像中的文字
    pub fn recognize(&mut self, input: Array4<f32>) -> Result<RecognitionResult, OcrError> {
        let results = self.recognize_batch(input)?;
        Ok(results.into_iter().next().unwrap_or(RecognitionResult {
            text: String::new(),
            confidence: 0.0,
        }))
    }

    /// 批量识别
    pub fn recognize_batch(&mut self, input: Array4<f32>) -> Result<Vec<RecognitionResult>, OcrError> {
        let batch_size = input.shape()[0];

        // 转换为 Tensor
        let input_tensor = Tensor::from_array(input)
            .map_err(|e| OcrError::Inference(e.to_string()))?;

        let outputs = self.session
            .run(ort::inputs![input_tensor])
            .map_err(|e| OcrError::Inference(format!("识别推理失败: {}", e)))?;

        let output_view = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|e| OcrError::Inference(e.to_string()))?;
        let output_owned = output_view.to_owned();
        let shape = output_owned.shape();
        let seq_len = shape[1];
        let num_classes = shape[2];
        drop(outputs);

        let mut results = Vec::with_capacity(batch_size);
        for b in 0..batch_size {
            let (text, confidence) = self.decode_ctc(&output_owned.view(), b, seq_len, num_classes);
            results.push(RecognitionResult { text, confidence });
        }

        Ok(results)
    }

    /// CTC 解码
    fn decode_ctc(
        &self,
        output: &ndarray::ArrayViewD<f32>,
        batch_idx: usize,
        seq_len: usize,
        num_classes: usize,
    ) -> (String, f32) {
        let mut text = String::new();
        let mut confidence_sum = 0.0f32;
        let mut char_count = 0;
        let mut last_idx: Option<usize> = None;

        for t in 0..seq_len {
            let mut max_prob = f32::NEG_INFINITY;
            let mut max_idx = 0;

            for c in 0..num_classes {
                let prob = output[[batch_idx, t, c]];
                if prob > max_prob {
                    max_prob = prob;
                    max_idx = c;
                }
            }

            let blank_idx = 0;
            if max_idx != blank_idx && Some(max_idx) != last_idx {
                let char_idx = max_idx.saturating_sub(1);
                if char_idx < self.charset.len() {
                    text.push_str(&self.charset[char_idx]);
                    let prob = 1.0 / (1.0 + (-max_prob).exp());
                    confidence_sum += prob;
                    char_count += 1;
                }
            }
            last_idx = Some(max_idx);
        }

        let avg_confidence = if char_count > 0 { confidence_sum / char_count as f32 } else { 0.0 };
        (text, avg_confidence)
    }
}

fn load_charset(path: &Path) -> Result<Vec<String>, OcrError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| OcrError::ModelLoad(format!("加载字符集失败: {}", e)))?;
    let charset: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    if charset.is_empty() {
        return Err(OcrError::ModelLoad("字符集为空".to_string()));
    }
    Ok(charset)
}

fn builtin_charset() -> Vec<String> {
    include_str!("charset/ppocr_keys_ocrv5.txt")
        .lines()
        .map(|s| s.to_string())
        .collect()
}
