//! OCR 错误类型

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OcrError {
    #[error("模型加载失败: {0}")]
    ModelLoad(String),

    #[error("图像处理失败: {0}")]
    ImageProcess(String),

    #[error("推理失败: {0}")]
    Inference(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}
