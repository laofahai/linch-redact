//! 图像预处理模块
//!
//! PaddleOCR 模型需要特定的图像预处理

use image::{DynamicImage, ImageBuffer, Rgb, RgbImage};
use ndarray::{Array3, Array4};

/// 检测模型的输入尺寸限制
pub const DET_LIMIT_SIDE: u32 = 960;
pub const DET_LIMIT_MIN: u32 = 32;

/// 识别模型的输入高度
pub const REC_IMAGE_HEIGHT: u32 = 48;
pub const REC_IMAGE_WIDTH: u32 = 320;

/// 裁剪区域边距比例
const CROP_PAD_RATIO: f32 = 0.04;
pub const REC_SPLIT_MAX_RATIO: f32 = REC_IMAGE_WIDTH as f32 / REC_IMAGE_HEIGHT as f32;
const REC_SPLIT_OVERLAP_RATIO: f32 = 0.12;

/// 归一化参数 (PaddleOCR 标准: (x/255 - 0.5) / 0.5)
const MEAN: [f32; 3] = [0.5, 0.5, 0.5];
const STD: [f32; 3] = [0.5, 0.5, 0.5];

/// 为检测模型准备输入
///
/// 1. 缩放图像到合适大小（最长边不超过 DET_LIMIT_SIDE，最短边不小于 DET_LIMIT_MIN）
/// 2. 确保尺寸是 32 的倍数
/// 3. 归一化
pub fn prepare_det_input(img: &DynamicImage) -> (Array4<f32>, f32, u32, u32) {
    let rgb = img.to_rgb8();
    let (orig_w, orig_h) = (rgb.width(), rgb.height());

    // 计算缩放比例
    let ratio = calculate_det_ratio(orig_w, orig_h);

    // 缩放
    let new_w = ((orig_w as f32 * ratio) as u32 / 32 * 32).max(DET_LIMIT_MIN);
    let new_h = ((orig_h as f32 * ratio) as u32 / 32 * 32).max(DET_LIMIT_MIN);

    let resized = image::imageops::resize(&rgb, new_w, new_h, image::imageops::FilterType::Lanczos3);

    // 转换为 NCHW 格式并归一化
    let tensor = normalize_image(&resized);
    let batch = tensor.insert_axis(ndarray::Axis(0));

    (batch, ratio, new_w, new_h)
}

/// 为识别模型准备输入
///
/// 1. 缩放到固定高度，宽度按比例
/// 2. 填充到固定宽度
/// 3. 归一化
#[allow(dead_code)]
pub fn prepare_rec_input(img: &DynamicImage) -> Array4<f32> {
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());

    // 按高度缩放
    let ratio = REC_IMAGE_HEIGHT as f32 / h as f32;
    let new_w = (w as f32 * ratio).min(REC_IMAGE_WIDTH as f32) as u32;

    let resized = image::imageops::resize(&rgb, new_w, REC_IMAGE_HEIGHT, image::imageops::FilterType::Lanczos3);

    // 创建填充后的图像（灰色填充）
    let mut padded: RgbImage = ImageBuffer::from_pixel(REC_IMAGE_WIDTH, REC_IMAGE_HEIGHT, Rgb([127, 127, 127]));
    image::imageops::overlay(&mut padded, &resized, 0, 0);

    // 转换为 NCHW 格式并归一化
    let tensor = normalize_image(&padded);
    tensor.insert_axis(ndarray::Axis(0))
}

/// 批量准备识别输入
#[allow(dead_code)]
pub fn prepare_rec_batch(images: &[DynamicImage], batch_size: usize) -> Vec<Array4<f32>> {
    let mut batches = Vec::new();

    for chunk in images.chunks(batch_size) {
        let batch_tensors: Vec<Array3<f32>> = chunk
            .iter()
            .map(|img| {
                let rgb = img.to_rgb8();
                let (w, h) = (rgb.width(), rgb.height());

                let ratio = REC_IMAGE_HEIGHT as f32 / h as f32;
                let new_w = (w as f32 * ratio).min(REC_IMAGE_WIDTH as f32) as u32;

                let resized = image::imageops::resize(&rgb, new_w, REC_IMAGE_HEIGHT, image::imageops::FilterType::Lanczos3);

                let mut padded: RgbImage = ImageBuffer::from_pixel(REC_IMAGE_WIDTH, REC_IMAGE_HEIGHT, Rgb([127, 127, 127]));
                image::imageops::overlay(&mut padded, &resized, 0, 0);

                normalize_image(&padded)
            })
            .collect();

        // Stack into batch
        let batch_len = batch_tensors.len();
        let mut batch = Array4::<f32>::zeros((batch_len, 3, REC_IMAGE_HEIGHT as usize, REC_IMAGE_WIDTH as usize));
        for (i, tensor) in batch_tensors.into_iter().enumerate() {
            batch.slice_mut(ndarray::s![i, .., .., ..]).assign(&tensor);
        }

        batches.push(batch);
    }

    batches
}

/// 计算检测模型的缩放比例
fn calculate_det_ratio(w: u32, h: u32) -> f32 {
    let max_side = w.max(h) as f32;
    let min_side = w.min(h) as f32;

    let mut ratio = 1.0f32;

    // 如果最长边超过限制，缩小
    if max_side > DET_LIMIT_SIDE as f32 {
        ratio = DET_LIMIT_SIDE as f32 / max_side;
    }

    // 确保最短边不小于最小值
    if min_side * ratio < DET_LIMIT_MIN as f32 {
        ratio = DET_LIMIT_MIN as f32 / min_side;
    }

    ratio
}

/// 将 RGB 图像归一化为 CHW 格式的 tensor (BGR 顺序，PP-OCRv5 要求)
fn normalize_image(img: &RgbImage) -> Array3<f32> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut tensor = Array3::<f32>::zeros((3, h, w));

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x as u32, y as u32);
            // RGB -> BGR 并归一化: (pixel / 255 - mean) / std
            // PP-OCRv5 使用 BGR 顺序
            tensor[[0, y, x]] = (pixel[2] as f32 / 255.0 - MEAN[0]) / STD[0]; // B
            tensor[[1, y, x]] = (pixel[1] as f32 / 255.0 - MEAN[1]) / STD[1]; // G
            tensor[[2, y, x]] = (pixel[0] as f32 / 255.0 - MEAN[2]) / STD[2]; // R
        }
    }

    tensor
}

/// 裁剪检测到的文本区域
pub fn crop_text_region(img: &DynamicImage, box_points: &[[f32; 2]; 4]) -> DynamicImage {
    // 计算边界框
    let min_x = box_points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min).max(0.0) as i32;
    let min_y = box_points.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min).max(0.0) as i32;
    let max_x = box_points.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max) as i32;
    let max_y = box_points.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max) as i32;

    let width = (max_x - min_x).max(1);
    let height = (max_y - min_y).max(1);

    let pad_x = (width as f32 * CROP_PAD_RATIO).round() as i32;
    let pad_y = (height as f32 * CROP_PAD_RATIO).round() as i32;

    let img_w = img.width() as i32;
    let img_h = img.height() as i32;

    let x0 = (min_x - pad_x).max(0);
    let y0 = (min_y - pad_y).max(0);
    let x1 = (max_x + pad_x)
        .min(img_w.saturating_sub(1))
        .max(0);
    let y1 = (max_y + pad_y)
        .min(img_h.saturating_sub(1))
        .max(0);

    let crop_w = (x1 - x0).max(1) as u32;
    let crop_h = (y1 - y0).max(1) as u32;

    img.crop_imm(x0 as u32, y0 as u32, crop_w, crop_h)
}

/// 将过长文本区域拆分为多段，避免识别时被过度压缩
pub fn split_long_text_region(
    img: &DynamicImage,
    max_ratio: f32,
    max_segments: usize,
) -> Vec<DynamicImage> {
    if max_ratio <= 0.0 {
        return vec![img.clone()];
    }
    if max_segments == 0 {
        return vec![img.clone()];
    }

    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return vec![img.clone()];
    }

    let ratio = w as f32 / h as f32;
    if ratio <= max_ratio {
        return vec![img.clone()];
    }

    let mut segments = (ratio / max_ratio).ceil().max(1.0) as u32;
    if max_segments > 0 {
        segments = segments.min(max_segments as u32);
    }
    if segments <= 1 {
        return vec![img.clone()];
    }
    let base_width = (w as f32 / segments as f32).ceil() as u32;
    let overlap = (base_width as f32 * REC_SPLIT_OVERLAP_RATIO).round() as u32;

    let mut results = Vec::new();
    for i in 0..segments {
        let mut start = i * base_width;
        let mut end = (start + base_width).min(w);
        if i > 0 {
            start = start.saturating_sub(overlap);
        }
        if i + 1 < segments {
            end = (end + overlap).min(w);
        }
        if end <= start {
            continue;
        }
        let width = end - start;
        results.push(img.crop_imm(start, 0, width, h));
    }

    if results.is_empty() {
        vec![img.clone()]
    } else {
        results
    }
}
