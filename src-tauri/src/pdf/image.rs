use std::io::Cursor;
use lopdf::{Document, Object, Stream};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use super::types::Mask;

/// 在图片上绘制黑色矩形
fn draw_black_rectangles_on_image(img: &mut RgbaImage, masks: &[Mask]) {
  let (img_width, img_height) = img.dimensions();
  let black = Rgba([0u8, 0u8, 0u8, 255u8]);

  for mask in masks {
    let x_start = (mask.x * img_width as f64) as u32;
    let y_start = (mask.y * img_height as f64) as u32;
    let rect_width = (mask.width * img_width as f64) as u32;
    let rect_height = (mask.height * img_height as f64) as u32;

    let x_end = (x_start + rect_width).min(img_width);
    let y_end = (y_start + rect_height).min(img_height);

    for y in y_start..y_end {
      for x in x_start..x_end {
        img.put_pixel(x, y, black);
      }
    }
  }
}

/// 处理页面中的图片（用于扫描件 PDF）
pub fn redact_page_images(
  doc: &mut Document,
  page_id: lopdf::ObjectId,
  masks: &[Mask],
) -> Result<bool, String> {
  let page = doc.get_object(page_id).map_err(|e| e.to_string())?;
  let resources_ref = if let Object::Dictionary(dict) = page {
    dict.get(b"Resources").ok().cloned()
  } else {
    return Ok(false);
  };

  let resources = match resources_ref {
    Some(Object::Reference(ref_id)) => {
      doc.get_object(ref_id).ok().cloned()
    }
    Some(obj) => Some(obj),
    None => None,
  };

  let xobject_dict = match resources {
    Some(Object::Dictionary(dict)) => {
      match dict.get(b"XObject") {
        Ok(Object::Reference(ref_id)) => {
          doc.get_object(*ref_id).ok().cloned()
        }
        Ok(obj) => Some(obj.clone()),
        Err(_) => None,
      }
    }
    _ => None,
  };

  let xobject_dict = match xobject_dict {
    Some(Object::Dictionary(dict)) => dict,
    _ => return Ok(false),
  };

  let mut processed_any = false;

  for (name, obj) in xobject_dict.iter() {
    let image_ref = match obj {
      Object::Reference(ref_id) => *ref_id,
      _ => continue,
    };

    let image_obj = match doc.get_object(image_ref) {
      Ok(Object::Stream(stream)) => stream.clone(),
      _ => continue,
    };

    let subtype = image_obj.dict.get(b"Subtype");
    if !matches!(subtype, Ok(Object::Name(n)) if n == b"Image") {
      continue;
    }

    log::info!("找到图片 XObject: {:?}", String::from_utf8_lossy(name));

    let width = match image_obj.dict.get(b"Width") {
      Ok(Object::Integer(w)) => *w as u32,
      _ => continue,
    };
    let height = match image_obj.dict.get(b"Height") {
      Ok(Object::Integer(h)) => *h as u32,
      _ => continue,
    };

    let image_data = match image_obj.decompressed_content() {
      Ok(data) => data,
      Err(_) => image_obj.content.clone(),
    };

    let bits_per_component = match image_obj.dict.get(b"BitsPerComponent") {
      Ok(Object::Integer(b)) => *b as u8,
      _ => 8,
    };

    let color_space = image_obj.dict.get(b"ColorSpace");
    let is_rgb = matches!(color_space, Ok(Object::Name(n)) if n == b"DeviceRGB");
    let is_gray = matches!(color_space, Ok(Object::Name(n)) if n == b"DeviceGray");

    let mut rgba_img = if is_rgb && bits_per_component == 8 && image_data.len() == (width * height * 3) as usize {
      let mut img = RgbaImage::new(width, height);
      for (i, pixel) in image_data.chunks(3).enumerate() {
        if pixel.len() == 3 {
          let x = (i as u32) % width;
          let y = (i as u32) / width;
          img.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], 255]));
        }
      }
      img
    } else if is_gray && bits_per_component == 8 && image_data.len() == (width * height) as usize {
      let mut img = RgbaImage::new(width, height);
      for (i, &gray) in image_data.iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        img.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
      }
      img
    } else {
      match image::load_from_memory(&image_data) {
        Ok(img) => img.to_rgba8(),
        Err(_) => {
          log::warn!("无法解码图片，跳过");
          continue;
        }
      }
    };

    draw_black_rectangles_on_image(&mut rgba_img, masks);

    let mut output_data = Vec::new();
    let mut cursor = Cursor::new(&mut output_data);

    let filter = image_obj.dict.get(b"Filter");
    let use_jpeg = matches!(filter, Ok(Object::Name(n)) if n == b"DCTDecode");

    if use_jpeg {
      let rgb_img = DynamicImage::ImageRgba8(rgba_img).to_rgb8();
      rgb_img.write_to(&mut cursor, ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;
    } else {
      let rgb_img = DynamicImage::ImageRgba8(rgba_img).to_rgb8();
      output_data = rgb_img.into_raw();
    }

    let mut new_dict = image_obj.dict.clone();
    if !use_jpeg {
      new_dict.remove(b"Filter");
      new_dict.remove(b"DecodeParms");
    }
    new_dict.set(b"Length", Object::Integer(output_data.len() as i64));

    let new_stream = if use_jpeg {
      Stream::new(new_dict, output_data)
    } else {
      let mut s = Stream::new(new_dict, output_data);
      s.compress().ok();
      s
    };

    doc.objects.insert(image_ref, Object::Stream(new_stream));
    processed_any = true;
    log::info!("图片 {:?} 脱敏完成", String::from_utf8_lossy(name));
  }

  Ok(processed_any)
}
