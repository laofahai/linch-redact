//! 文字检测模块
//!
//! 使用 DBNet (Differentiable Binarization) 模型检测文字区域

use ndarray::{Array2, Array4};
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;

use crate::error::OcrError;
use crate::threading::apply_session_threads;

/// 检测阈值
const THRESH: f32 = 0.3;
const BOX_THRESH: f32 = 0.5;
const MIN_SIZE: f32 = 5.0;
const UNCLIP_RATIO: f32 = 1.6;

/// 文字检测器
pub struct TextDetector {
    session: Session,
}

/// 检测到的文字框
#[derive(Debug, Clone)]
pub struct TextBox {
    pub points: [[f32; 2]; 4],
    pub score: f32,
}

impl TextDetector {
    /// 从 ONNX 模型文件创建检测器
    pub fn new(model_path: &Path) -> Result<Self, OcrError> {
        let builder = Session::builder()
            .map_err(|e: ort::Error| OcrError::ModelLoad(e.to_string()))?;
        let builder = apply_session_threads(builder)
            .map_err(|e| OcrError::ModelLoad(e.to_string()))?;
        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| OcrError::ModelLoad(format!("加载检测模型失败: {}", e)))?;

        Ok(Self { session })
    }

    /// 检测图像中的文字区域
    pub fn detect(
        &mut self,
        input: Array4<f32>,
        orig_w: u32,
        orig_h: u32,
    ) -> Result<Vec<TextBox>, OcrError> {
        let input_h = input.shape()[2] as u32;
        let input_w = input.shape()[3] as u32;

        // 转换为 Tensor
        let input_tensor = Tensor::from_array(input)
            .map_err(|e| OcrError::Inference(e.to_string()))?;

        // 运行推理
        let outputs = self.session
            .run(ort::inputs![input_tensor])
            .map_err(|e| OcrError::Inference(format!("检测推理失败: {}", e)))?;

        // 获取输出并转换为 ndarray，复制数据以避免借用冲突
        let output_view = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|e| OcrError::Inference(e.to_string()))?;
        let output_owned = output_view.to_owned();
        drop(outputs);

        // 后处理
        let boxes = Self::post_process(&output_owned.view(), input_w, input_h, orig_w, orig_h)?;
        Ok(boxes)
    }

    /// 后处理
    fn post_process(
        output: &ndarray::ArrayViewD<f32>,
        input_w: u32,
        input_h: u32,
        orig_w: u32,
        orig_h: u32,
    ) -> Result<Vec<TextBox>, OcrError> {
        let shape = output.shape();
        let (h, w) = if shape.len() == 4 {
            (shape[2], shape[3])
        } else if shape.len() == 3 {
            (shape[1], shape[2])
        } else {
            return Err(OcrError::Inference(format!("意外的输出形状: {:?}", shape)));
        };

        let mut binary = Array2::<u8>::zeros((h, w));
        for i in 0..h {
            for j in 0..w {
                let val = if shape.len() == 4 { output[[0, 0, i, j]] } else { output[[0, i, j]] };
                if val > THRESH { binary[[i, j]] = 255; }
            }
        }

        Ok(find_boxes(&binary, output, input_w, input_h, orig_w, orig_h))
    }
}

fn find_boxes(
    binary: &Array2<u8>,
    prob_map: &ndarray::ArrayViewD<f32>,
    input_w: u32,
    input_h: u32,
    orig_w: u32,
    orig_h: u32,
) -> Vec<TextBox> {
    let h = binary.shape()[0];
    let w = binary.shape()[1];
    let shape = prob_map.shape();
    let mut visited = Array2::<bool>::from_elem((h, w), false);
    let mut boxes = Vec::new();

    for start_y in 0..h {
        for start_x in 0..w {
            if binary[[start_y, start_x]] == 255 && !visited[[start_y, start_x]] {
                let mut min_x = start_x;
                let mut max_x = start_x;
                let mut min_y = start_y;
                let mut max_y = start_y;
                let mut score_sum = 0.0f32;
                let mut count = 0;
                let mut queue = vec![(start_x, start_y)];
                visited[[start_y, start_x]] = true;

                while let Some((x, y)) = queue.pop() {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    let val = if shape.len() == 4 { prob_map[[0, 0, y, x]] } else { prob_map[[0, y, x]] };
                    score_sum += val;
                    count += 1;

                    for (dx, dy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                            let (nx, ny) = (nx as usize, ny as usize);
                            if binary[[ny, nx]] == 255 && !visited[[ny, nx]] {
                                visited[[ny, nx]] = true;
                                queue.push((nx, ny));
                            }
                        }
                    }
                }

                let box_w = (max_x - min_x) as f32;
                let box_h = (max_y - min_y) as f32;
                if box_w < MIN_SIZE || box_h < MIN_SIZE { continue; }
                let avg_score = score_sum / count as f32;
                if avg_score < BOX_THRESH { continue; }

                let expand_w = box_w * (UNCLIP_RATIO - 1.0) / 2.0;
                let expand_h = box_h * (UNCLIP_RATIO - 1.0) / 2.0;
                let x1 = (min_x as f32 - expand_w).max(0.0);
                let y1 = (min_y as f32 - expand_h).max(0.0);
                let x2 = (max_x as f32 + expand_w).min(w as f32 - 1.0);
                let y2 = (max_y as f32 + expand_h).min(h as f32 - 1.0);

                let scale_x = orig_w as f32 / input_w as f32;
                let scale_y = orig_h as f32 / input_h as f32;

                boxes.push(TextBox {
                    points: [
                        [x1 * scale_x, y1 * scale_y],
                        [x2 * scale_x, y1 * scale_y],
                        [x2 * scale_x, y2 * scale_y],
                        [x1 * scale_x, y2 * scale_y],
                    ],
                    score: avg_score,
                });
            }
        }
    }

    boxes.sort_by(|a, b| a.points[0][1].partial_cmp(&b.points[0][1]).unwrap_or(std::cmp::Ordering::Equal));
    boxes
}
