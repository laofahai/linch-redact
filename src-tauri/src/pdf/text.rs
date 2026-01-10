use super::types::MaskRect;
use super::utils::get_number;
use lopdf::{
    content::{Content, Operation},
    Object,
};

/// 估算单个字符的宽度
fn estimate_char_width(byte: u8, font_size: f32) -> f32 {
    if byte < 128 {
        font_size * 0.55
    } else {
        font_size * 1.0
    }
}

/// 估算文字宽度
fn estimate_text_width(text: &[u8], font_size: f32) -> f32 {
    text.iter()
        .map(|&b| estimate_char_width(b, font_size))
        .sum()
}

/// 检查单个字符是否在任何 mask 区域内
fn char_in_mask(
    char_x: f32,
    char_y: f32,
    char_width: f32,
    font_size: f32,
    masks: &[MaskRect],
) -> bool {
    let char_height = font_size.abs().max(12.0);
    masks
        .iter()
        .any(|m| m.intersects_text_bbox(char_x, char_y, char_width, char_height))
}

/// 对文字进行字符级脱敏：将落在 mask 区域内的字符删除（用空格替代），确保文字无法复制
fn redact_text_chars(
    text: &[u8],
    start_x: f32,
    start_y: f32,
    font_size: f32,
    masks: &[MaskRect],
) -> (Vec<u8>, bool) {
    let mut result = Vec::with_capacity(text.len());
    let mut current_x = start_x;
    let mut any_redacted = false;

    for &byte in text.iter() {
        let char_width = estimate_char_width(byte, font_size);
        let is_in_mask = char_in_mask(current_x, start_y, char_width, font_size, masks);

        if is_in_mask {
            // 用空格替代被脱敏的字符，这样可以：
            // 1. 保持文字流的位置不变（避免后续字符位置偏移）
            // 2. 确保文字无法被复制（空格没有实际内容）
            result.push(b' ');
            any_redacted = true;
        } else {
            result.push(byte);
        }

        current_x += char_width;
    }

    (result, any_redacted)
}

/// 处理内容流，将 mask 区域内的文字替换为空格
pub fn process_content_stream(content_data: &[u8], masks: &[MaskRect]) -> Result<Vec<u8>, String> {
    let content = Content::decode(content_data).map_err(|e| e.to_string())?;
    let mut new_operations: Vec<Operation> = Vec::new();

    let mut graphics_state_stack: Vec<[f32; 6]> = Vec::new();

    #[allow(unused_assignments)]
    let mut text_matrix: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut line_matrix: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut ctm: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut in_text_object = false;
    let mut font_size: f32 = 12.0;

    for op in content.operations {
        let operator = op.operator.as_str();

        match operator {
            "q" => {
                graphics_state_stack.push(ctm);
                new_operations.push(op);
            }
            "Q" => {
                if let Some(saved_ctm) = graphics_state_stack.pop() {
                    ctm = saved_ctm;
                }
                new_operations.push(op);
            }
            "cm" if op.operands.len() >= 6 => {
                if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                    get_number(&op.operands[0]),
                    get_number(&op.operands[1]),
                    get_number(&op.operands[2]),
                    get_number(&op.operands[3]),
                    get_number(&op.operands[4]),
                    get_number(&op.operands[5]),
                ) {
                    let new_ctm = [
                        ctm[0] * a + ctm[2] * b,
                        ctm[1] * a + ctm[3] * b,
                        ctm[0] * c + ctm[2] * d,
                        ctm[1] * c + ctm[3] * d,
                        ctm[0] * e + ctm[2] * f + ctm[4],
                        ctm[1] * e + ctm[3] * f + ctm[5],
                    ];
                    ctm = new_ctm;
                }
                new_operations.push(op);
            }
            #[allow(unused_assignments)]
            "BT" => {
                in_text_object = true;
                text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                new_operations.push(op);
            }
            "ET" => {
                in_text_object = false;
                new_operations.push(op);
            }
            "Tm" if in_text_object && op.operands.len() >= 6 => {
                if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                    get_number(&op.operands[0]),
                    get_number(&op.operands[1]),
                    get_number(&op.operands[2]),
                    get_number(&op.operands[3]),
                    get_number(&op.operands[4]),
                    get_number(&op.operands[5]),
                ) {
                    text_matrix = [a, b, c, d, e, f];
                    line_matrix = text_matrix;
                }
                new_operations.push(op);
            }
            "Td" if in_text_object && op.operands.len() >= 2 => {
                if let (Some(tx), Some(ty)) =
                    (get_number(&op.operands[0]), get_number(&op.operands[1]))
                {
                    line_matrix[4] += tx;
                    line_matrix[5] += ty;
                    text_matrix = line_matrix;
                }
                new_operations.push(op);
            }
            "TD" if in_text_object && op.operands.len() >= 2 => {
                if let (Some(tx), Some(ty)) =
                    (get_number(&op.operands[0]), get_number(&op.operands[1]))
                {
                    line_matrix[4] += tx;
                    line_matrix[5] += ty;
                    text_matrix = line_matrix;
                }
                new_operations.push(op);
            }
            "T*" if in_text_object => {
                new_operations.push(op);
            }
            "Tf" if op.operands.len() >= 2 => {
                if let Some(size) = get_number(&op.operands[1]) {
                    font_size = size.abs();
                }
                new_operations.push(op);
            }
            "Tj" if in_text_object => {
                let user_x = ctm[0] * text_matrix[4] + ctm[2] * text_matrix[5] + ctm[4];
                let user_y = ctm[1] * text_matrix[4] + ctm[3] * text_matrix[5] + ctm[5];

                let (text_bytes, str_format) =
                    if let Some(Object::String(s, fmt)) = op.operands.first() {
                        (s.clone(), *fmt)
                    } else {
                        (vec![], lopdf::StringFormat::Literal)
                    };

                let (redacted_text, any_redacted) =
                    redact_text_chars(&text_bytes, user_x, user_y, font_size, masks);

                if any_redacted {
                    log::info!(
                        "[Tj脱敏] {:?} -> {:?}",
                        String::from_utf8_lossy(&text_bytes),
                        String::from_utf8_lossy(&redacted_text)
                    );
                    new_operations.push(Operation::new(
                        "Tj",
                        vec![Object::String(redacted_text, str_format)],
                    ));
                } else {
                    new_operations.push(op);
                }
            }
            "TJ" if in_text_object => {
                let mut current_x = ctm[0] * text_matrix[4] + ctm[2] * text_matrix[5] + ctm[4];
                let user_y = ctm[1] * text_matrix[4] + ctm[3] * text_matrix[5] + ctm[5];

                let mut new_array: Vec<Object> = Vec::new();
                let mut any_redacted = false;

                if let Some(Object::Array(arr)) = op.operands.first() {
                    for item in arr {
                        match item {
                            Object::String(s, fmt) => {
                                let (redacted, redacted_this) =
                                    redact_text_chars(s, current_x, user_y, font_size, masks);
                                if redacted_this {
                                    any_redacted = true;
                                    log::info!(
                                        "[TJ脱敏] {:?} -> {:?}",
                                        String::from_utf8_lossy(s),
                                        String::from_utf8_lossy(&redacted)
                                    );
                                }
                                current_x += estimate_text_width(s, font_size);
                                new_array.push(Object::String(redacted, *fmt));
                            }
                            Object::Integer(n) => {
                                let adjustment = (*n as f32) / 1000.0 * font_size;
                                current_x -= adjustment;
                                new_array.push(item.clone());
                            }
                            Object::Real(n) => {
                                let adjustment = n / 1000.0 * font_size;
                                current_x -= adjustment;
                                new_array.push(item.clone());
                            }
                            _ => {
                                new_array.push(item.clone());
                            }
                        }
                    }
                }

                if any_redacted {
                    new_operations.push(Operation::new("TJ", vec![Object::Array(new_array)]));
                } else {
                    new_operations.push(op);
                }
            }
            "'" if in_text_object => {
                let user_x = ctm[0] * text_matrix[4] + ctm[2] * text_matrix[5] + ctm[4];
                let user_y = ctm[1] * text_matrix[4] + ctm[3] * text_matrix[5] + ctm[5];

                let (text_bytes, str_format) =
                    if let Some(Object::String(s, fmt)) = op.operands.first() {
                        (s.clone(), *fmt)
                    } else {
                        (vec![], lopdf::StringFormat::Literal)
                    };

                let (redacted_text, any_redacted) =
                    redact_text_chars(&text_bytes, user_x, user_y, font_size, masks);

                if any_redacted {
                    log::info![
                        "['脱敏] {:?} -> {:?}",
                        String::from_utf8_lossy(&text_bytes),
                        String::from_utf8_lossy(&redacted_text)
                    ];
                    new_operations.push(Operation::new(
                        "'",
                        vec![Object::String(redacted_text, str_format)],
                    ));
                } else {
                    new_operations.push(op);
                }
            }
            "\"" if in_text_object && op.operands.len() >= 3 => {
                let user_x = ctm[0] * text_matrix[4] + ctm[2] * text_matrix[5] + ctm[4];
                let user_y = ctm[1] * text_matrix[4] + ctm[3] * text_matrix[5] + ctm[5];

                let (text_bytes, str_format) = if let Object::String(s, fmt) = &op.operands[2] {
                    (s.clone(), *fmt)
                } else {
                    (vec![], lopdf::StringFormat::Literal)
                };

                let (redacted_text, any_redacted) =
                    redact_text_chars(&text_bytes, user_x, user_y, font_size, masks);

                if any_redacted {
                    log::info!(
                        "[\"脱敏] {:?} -> {:?}",
                        String::from_utf8_lossy(&text_bytes),
                        String::from_utf8_lossy(&redacted_text)
                    );
                    let mut new_operands = op.operands.clone();
                    new_operands[2] = Object::String(redacted_text, str_format);
                    new_operations.push(Operation::new("\"", new_operands));
                } else {
                    new_operations.push(op);
                }
            }
            _ => {
                new_operations.push(op);
            }
        }
    }

    let new_content = Content {
        operations: new_operations,
    };
    new_content.encode().map_err(|e| e.to_string())
}

/// 添加黑框覆盖到内容流
pub fn add_black_overlay(content_data: &[u8], masks: &[MaskRect]) -> Result<Vec<u8>, String> {
    let content = Content::decode(content_data).map_err(|e| e.to_string())?;
    let mut new_operations = content.operations;

    // 保存当前图形状态
    new_operations.push(Operation::new("q", vec![]));

    // 重置 CTM 为单位矩阵，确保黑框在正确的页面坐标位置
    // 注意：如果内容流中有 cm 操作修改了 CTM，这里需要重置
    // 使用 cm 设置单位矩阵: [1 0 0 1 0 0]
    // 但更安全的做法是不重置 CTM，因为有些 PDF 的坐标系可能依赖于 CTM

    // 设置填充颜色为黑色 (RGB: 0, 0, 0)
    new_operations.push(Operation::new(
        "rg",
        vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)],
    ));

    // 设置描边颜色也为黑色（以防某些 PDF 阅读器行为不一致）
    new_operations.push(Operation::new(
        "RG",
        vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)],
    ));

    for rect in masks {
        log::info!(
            "[BlackOverlay] 绘制黑框: x={}, y={}, w={}, h={}",
            rect.x,
            rect.y,
            rect.width,
            rect.height
        );
        // 绘制矩形路径
        new_operations.push(Operation::new(
            "re",
            vec![
                Object::Real(rect.x),
                Object::Real(rect.y),
                Object::Real(rect.width),
                Object::Real(rect.height),
            ],
        ));
        // 填充路径（使用非零绕组规则）
        new_operations.push(Operation::new("f", vec![]));
    }

    // 恢复图形状态
    new_operations.push(Operation::new("Q", vec![]));

    let new_content = Content {
        operations: new_operations,
    };
    new_content.encode().map_err(|e| e.to_string())
}
