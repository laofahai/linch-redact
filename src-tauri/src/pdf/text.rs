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

/// 对文字进行脱敏：将落在 mask 区域内的字符替换为空白 glyph (0x00)
/// 保持字节数不变，确保文字位置推进正确
/// 返回 (处理后的字节, 是否有脱敏)
fn redact_text_bytes(
    text: &[u8],
    start_x: f32,
    start_y: f32,
    font_size: f32,
    masks: &[MaskRect],
) -> (Vec<u8>, bool) {
    let mut current_x = start_x;
    let mut any_in_mask = false;

    // 先检查是否有任何字符在 mask 区域内
    for &byte in text.iter() {
        let char_width = estimate_char_width(byte, font_size);
        if char_in_mask(current_x, start_y, char_width, font_size, masks) {
            any_in_mask = true;
            break;
        }
        current_x += char_width;
    }

    if any_in_mask {
        // 将所有字节替换为 0x00（空白 glyph）
        // 保持字节数不变，确保位置推进正确
        let redacted = vec![0u8; text.len()];
        (redacted, true)
    } else {
        (text.to_vec(), false)
    }
}

/// 处理内容流，将 mask 区域内的文字替换为空格
pub fn process_content_stream(content_data: &[u8], masks: &[MaskRect]) -> Result<Vec<u8>, String> {
    let content = Content::decode(content_data).map_err(|e| e.to_string())?;
    let mut new_operations: Vec<Operation> = Vec::new();

    // 统计操作符
    let mut tj_count = 0;
    let mut big_tj_count = 0;
    for op in &content.operations {
        match op.operator.as_str() {
            "Tj" => tj_count += 1,
            "TJ" => big_tj_count += 1,
            _ => {}
        }
    }
    log::info!(
        "[TextReplace] 内容流统计: {} 个操作符, Tj={}, TJ={}, masks={:?}",
        content.operations.len(),
        tj_count,
        big_tj_count,
        masks
    );

    // 打印前 5 个 Tj 操作的内容（调试）
    let mut tj_samples = 0;
    for op in &content.operations {
        if op.operator == "Tj" && tj_samples < 5 {
            if let Some(Object::String(s, _)) = op.operands.first() {
                log::info!(
                    "[TextReplace] Tj 样本 {}: 字节={:?}, 文字={:?}",
                    tj_samples,
                    &s[..s.len().min(20)],
                    String::from_utf8_lossy(&s[..s.len().min(20)])
                );
            }
            tj_samples += 1;
        }
    }

    // 打印 mask 区域信息
    if let Some(m) = masks.first() {
        log::info!(
            "[TextReplace] Mask 区域: x=[{:.1}, {:.1}], y=[{:.1}, {:.1}]",
            m.x,
            m.x + m.width,
            m.y,
            m.y + m.height
        );
    }

    let mut graphics_state_stack: Vec<[f32; 6]> = Vec::new();

    #[allow(unused_assignments)]
    let mut text_matrix: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut line_matrix: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut ctm: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut in_text_object = false;
    let mut font_size: f32 = 12.0;

    // 统计所有 Tj 的 Y 坐标范围（调试）
    let mut min_y: f32 = f32::MAX;
    let mut max_y: f32 = f32::MIN;
    let mut tj_positions: Vec<(f32, f32)> = Vec::new();

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
                    // Td: Tm = Lm × T(tx, ty)
                    // 矩阵乘法: [a b c d e f] × [1 0 0 1 tx ty]
                    // 结果: [a b c d (a*tx + c*ty + e) (b*tx + d*ty + f)]
                    let new_e = line_matrix[0] * tx + line_matrix[2] * ty + line_matrix[4];
                    let new_f = line_matrix[1] * tx + line_matrix[3] * ty + line_matrix[5];
                    line_matrix[4] = new_e;
                    line_matrix[5] = new_f;
                    text_matrix = line_matrix;
                }
                new_operations.push(op);
            }
            "TD" if in_text_object && op.operands.len() >= 2 => {
                if let (Some(tx), Some(ty)) =
                    (get_number(&op.operands[0]), get_number(&op.operands[1]))
                {
                    // TD: 等同于 -ty TL, tx ty Td
                    // 矩阵乘法: [a b c d e f] × [1 0 0 1 tx ty]
                    let new_e = line_matrix[0] * tx + line_matrix[2] * ty + line_matrix[4];
                    let new_f = line_matrix[1] * tx + line_matrix[3] * ty + line_matrix[5];
                    line_matrix[4] = new_e;
                    line_matrix[5] = new_f;
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
                // 将 text_matrix 位置转换到用户空间（应用 CTM）
                // Mask 坐标是在用户空间中的，所以需要将文本位置也转换到用户空间
                let tm_x = text_matrix[4];
                let tm_y = text_matrix[5];
                let user_x = ctm[0] * tm_x + ctm[2] * tm_y + ctm[4];
                let user_y = ctm[1] * tm_x + ctm[3] * tm_y + ctm[5];

                let (text_bytes, str_format) =
                    if let Some(Object::String(s, fmt)) = op.operands.first() {
                        (s.clone(), *fmt)
                    } else {
                        (vec![], lopdf::StringFormat::Literal)
                    };

                // 记录位置统计
                min_y = min_y.min(user_y);
                max_y = max_y.max(user_y);
                tj_positions.push((user_x, user_y));

                // 打印第一个 Tj 的 CTM 和 text_matrix（调试）
                if tj_positions.len() == 1 {
                    log::info!(
                        "[TextReplace] 第一个Tj: CTM=[{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}], text_matrix=[{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}], user_pos=({:.1}, {:.1})",
                        ctm[0], ctm[1], ctm[2], ctm[3], ctm[4], ctm[5],
                        text_matrix[0], text_matrix[1], text_matrix[2], text_matrix[3], text_matrix[4], text_matrix[5],
                        user_x, user_y
                    );
                }

                // 检查 Tj 位置是否在 mask 区域附近（调试用）
                if let Some(m) = masks.first() {
                    // 扩大检查范围，看看有没有 Tj 在 mask 的 Y 坐标范围附近（±100）
                    let y_near_mask = user_y > m.y - 100.0 && user_y < m.y + m.height + 100.0;
                    if y_near_mask {
                        log::info!(
                            "[Tj] Y坐标接近mask! 位置: ({:.1}, {:.1}), font_size={:.1}, mask_y=[{:.1}, {:.1}]",
                            user_x, user_y, font_size, m.y, m.y + m.height
                        );
                    }
                }

                let (redacted_bytes, any_redacted) =
                    redact_text_bytes(&text_bytes, user_x, user_y, font_size, masks);

                if any_redacted {
                    log::info!(
                        "[Tj脱敏] 原始字节={:?}, 长度={}",
                        &text_bytes[..text_bytes.len().min(10)],
                        text_bytes.len()
                    );
                    new_operations.push(Operation::new(
                        "Tj",
                        vec![Object::String(redacted_bytes, str_format)],
                    ));
                } else {
                    new_operations.push(op);
                }
            }
            "TJ" if in_text_object => {
                // 将 text_matrix 位置转换到用户空间（应用 CTM）
                let tm_x = text_matrix[4];
                let tm_y = text_matrix[5];
                let mut current_x = ctm[0] * tm_x + ctm[2] * tm_y + ctm[4];
                let user_y = ctm[1] * tm_x + ctm[3] * tm_y + ctm[5];
                // CTM 的水平缩放因子，用于缩放文字宽度
                let x_scale = ctm[0].abs().max(0.001);

                let mut new_array: Vec<Object> = Vec::new();
                let mut any_redacted = false;

                if let Some(Object::Array(arr)) = op.operands.first() {
                    for item in arr {
                        match item {
                            Object::String(s, fmt) => {
                                let (redacted_bytes, redacted_this) =
                                    redact_text_bytes(s, current_x, user_y, font_size, masks);
                                if redacted_this {
                                    any_redacted = true;
                                    log::info!(
                                        "[TJ脱敏] 原始字节={:?}, 长度={}",
                                        &s[..s.len().min(10)],
                                        s.len()
                                    );
                                }
                                // 文字宽度需要按 CTM 缩放
                                current_x += estimate_text_width(s, font_size) * x_scale;
                                new_array.push(Object::String(redacted_bytes, *fmt));
                            }
                            Object::Integer(n) => {
                                // 调整值也需要按 CTM 缩放
                                let adjustment = (*n as f32) / 1000.0 * font_size * x_scale;
                                current_x -= adjustment;
                                new_array.push(item.clone());
                            }
                            Object::Real(n) => {
                                // 调整值也需要按 CTM 缩放
                                let adjustment = n / 1000.0 * font_size * x_scale;
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
                // 将 text_matrix 位置转换到用户空间（应用 CTM）
                let tm_x = text_matrix[4];
                let tm_y = text_matrix[5];
                let user_x = ctm[0] * tm_x + ctm[2] * tm_y + ctm[4];
                let user_y = ctm[1] * tm_x + ctm[3] * tm_y + ctm[5];

                let (text_bytes, str_format) =
                    if let Some(Object::String(s, fmt)) = op.operands.first() {
                        (s.clone(), *fmt)
                    } else {
                        (vec![], lopdf::StringFormat::Literal)
                    };

                let (redacted_bytes, any_redacted) =
                    redact_text_bytes(&text_bytes, user_x, user_y, font_size, masks);

                if any_redacted {
                    log::info!(
                        "['脱敏] 原始字节={:?}, 长度={}",
                        &text_bytes[..text_bytes.len().min(10)],
                        text_bytes.len()
                    );
                    new_operations.push(Operation::new(
                        "'",
                        vec![Object::String(redacted_bytes, str_format)],
                    ));
                } else {
                    new_operations.push(op);
                }
            }
            "\"" if in_text_object && op.operands.len() >= 3 => {
                // 将 text_matrix 位置转换到用户空间（应用 CTM）
                let tm_x = text_matrix[4];
                let tm_y = text_matrix[5];
                let user_x = ctm[0] * tm_x + ctm[2] * tm_y + ctm[4];
                let user_y = ctm[1] * tm_x + ctm[3] * tm_y + ctm[5];

                let (text_bytes, str_format) = if let Object::String(s, fmt) = &op.operands[2] {
                    (s.clone(), *fmt)
                } else {
                    (vec![], lopdf::StringFormat::Literal)
                };

                let (redacted_bytes, any_redacted) =
                    redact_text_bytes(&text_bytes, user_x, user_y, font_size, masks);

                if any_redacted {
                    log::info!(
                        "[\"脱敏] 原始字节={:?}, 长度={}",
                        &text_bytes[..text_bytes.len().min(10)],
                        text_bytes.len()
                    );
                    let mut new_operands = op.operands.clone();
                    new_operands[2] = Object::String(redacted_bytes, str_format);
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

    // 打印 Tj 位置统计
    if !tj_positions.is_empty() {
        log::info!(
            "[TextReplace] Tj 位置统计: {} 个, Y范围=[{:.1}, {:.1}]",
            tj_positions.len(),
            min_y,
            max_y
        );
        if let Some(m) = masks.first() {
            log::info!(
                "[TextReplace] Mask Y范围=[{:.1}, {:.1}], 是否有交集: {}",
                m.y,
                m.y + m.height,
                min_y <= m.y + m.height && max_y >= m.y
            );
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
