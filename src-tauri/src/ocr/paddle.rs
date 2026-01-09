//! Paddle OCR 引擎实现

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use tauri::Emitter;

use crate::config::{models_dir, save_config, AppConfig, ConfigResult};
use crate::ocr::types::{
    BBox, DownloadProgress, OcrTextResult, PaddleInstallRequest, PaddleInstallResult, PaddleStatus,
};

/// 全局 Paddle OCR 引擎实例
static PADDLE_ENGINE: Mutex<Option<linch_ocr::PaddleOcrEngine>> = Mutex::new(None);

/// 存储模型路径用于自动初始化
static PADDLE_MODEL_PATHS: Mutex<Option<(String, String)>> = Mutex::new(None);

/// 下载文件（带进度）
fn download_file_with_progress(
    app: &tauri::AppHandle,
    url: &str,
    dest: &Path,
    file_name: &str,
    file_index: u32,
    total_files: u32,
) -> Result<(), String> {
    log::info!("[Paddle] 下载: {} -> {:?}", url, dest);

    let response = reqwest::blocking::get(url).map_err(|err| format!("下载失败: {}", err))?;
    if !response.status().is_success() {
        return Err(format!("下载失败，状态码: {}", response.status()));
    }

    let total_size = response.content_length();
    let mut reader = response;
    let mut file = fs::File::create(dest).map_err(|err| format!("创建文件失败: {}", err))?;
    let mut buffer = [0u8; 65536];
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;

    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|err| format!("读取数据失败: {}", err))?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|err| format!("写入文件失败: {}", err))?;
        downloaded += count as u64;

        if downloaded - last_emit > 102400 || count == 0 {
            last_emit = downloaded;
            let percent = total_size
                .map(|t| (downloaded as f32 / t as f32) * 100.0)
                .unwrap_or(0.0);
            let _ = app.emit(
                "ocr-download-progress",
                DownloadProgress {
                    file_name: file_name.to_string(),
                    file_index,
                    total_files,
                    downloaded,
                    total: total_size,
                    percent,
                },
            );
        }
    }

    let _ = app.emit(
        "ocr-download-progress",
        DownloadProgress {
            file_name: file_name.to_string(),
            file_index,
            total_files,
            downloaded,
            total: total_size,
            percent: 100.0,
        },
    );

    log::info!("[Paddle] 下载完成: {} 字节", downloaded);
    Ok(())
}

/// 检查 Paddle OCR 模型是否已安装
pub fn is_paddle_installed(app: &tauri::AppHandle) -> ConfigResult<bool> {
    let config = crate::config::load_config(app.clone())?;

    let det_exists = config
        .det_model_path
        .as_ref()
        .map(|p| Path::new(p).exists())
        .unwrap_or(false);

    let rec_exists = config
        .rec_model_path
        .as_ref()
        .map(|p| Path::new(p).exists())
        .unwrap_or(false);

    Ok(det_exists && rec_exists)
}

/// 获取 Paddle 状态
pub fn get_paddle_status(app: &tauri::AppHandle) -> PaddleStatus {
    let config = crate::config::load_config(app.clone()).ok();

    let installed = config
        .as_ref()
        .map(|c| {
            c.det_model_path
                .as_ref()
                .map(|p| Path::new(p).exists())
                .unwrap_or(false)
                && c.rec_model_path
                    .as_ref()
                    .map(|p| Path::new(p).exists())
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    PaddleStatus {
        installed,
        det_model_path: config.as_ref().and_then(|c| c.det_model_path.clone()),
        rec_model_path: config.as_ref().and_then(|c| c.rec_model_path.clone()),
        model_version: config.as_ref().and_then(|c| c.model_version.clone()),
    }
}

/// 安装 Paddle OCR 模型
pub async fn install_paddle_models(
    app: tauri::AppHandle,
    request: PaddleInstallRequest,
) -> ConfigResult<PaddleInstallResult> {
    let app_clone = app.clone();

    tauri::async_runtime::spawn_blocking(move || -> ConfigResult<PaddleInstallResult> {
        let model_dir = models_dir(&app_clone).map_err(|err| err.to_string())?;
        fs::create_dir_all(&model_dir).map_err(|err| format!("创建目录失败: {}", err))?;

        let det_path = model_dir.join("det.onnx");
        let rec_path = model_dir.join("rec.onnx");

        download_file_with_progress(
            &app_clone,
            &request.det_url,
            &det_path,
            "检测模型 (det.onnx)",
            1,
            2,
        )?;

        download_file_with_progress(
            &app_clone,
            &request.rec_url,
            &rec_path,
            "识别模型 (rec.onnx)",
            2,
            2,
        )?;

        let config = AppConfig {
            det_model_path: Some(det_path.to_string_lossy().to_string()),
            rec_model_path: Some(rec_path.to_string_lossy().to_string()),
            model_version: request.model_version,
            install_source: request.install_source,
            ..Default::default()
        };
        save_config(app_clone, config.clone())?;

        log::info!("[Paddle] 模型安装完成");

        Ok(PaddleInstallResult {
            det_model_path: config.det_model_path.unwrap_or_default(),
            rec_model_path: config.rec_model_path.unwrap_or_default(),
        })
    })
    .await
    .map_err(|err| err.to_string())?
}

/// 初始化 Paddle OCR 引擎
pub fn init_paddle_engine(app: &tauri::AppHandle) -> ConfigResult<()> {
    let config = crate::config::load_config(app.clone())?;

    let det_path = config.det_model_path.ok_or("检测模型未安装")?;
    let rec_path = config.rec_model_path.ok_or("识别模型未安装")?;

    // 存储模型路径用于自动初始化
    if let Ok(mut paths) = PADDLE_MODEL_PATHS.lock() {
        *paths = Some((det_path.clone(), rec_path.clone()));
    }

    init_paddle_engine_with_paths(&det_path, &rec_path)
}

/// 使用指定路径初始化 Paddle OCR 引擎
fn init_paddle_engine_with_paths(det_path: &str, rec_path: &str) -> ConfigResult<()> {
    let ocr_config = linch_ocr::OcrConfig {
        det_model_path: det_path.to_string(),
        rec_model_path: rec_path.to_string(),
        dict_path: None,
    };

    let engine = linch_ocr::PaddleOcrEngine::new(&ocr_config)
        .map_err(|e| format!("初始化 Paddle OCR 引擎失败: {}", e))?;

    let mut guard = PADDLE_ENGINE
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    *guard = Some(engine);

    log::info!("[Paddle] 引擎初始化成功");
    Ok(())
}

/// 使用 Paddle 引擎识别图片
pub fn paddle_recognize(image_path: &str) -> ConfigResult<Vec<OcrTextResult>> {
    // 先检查是否需要初始化
    {
        let guard = PADDLE_ENGINE
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;

        if guard.is_none() {
            log::info!("[Paddle] 引擎未初始化，尝试自动初始化...");
            // 获取模型路径
            let paths = PADDLE_MODEL_PATHS.lock().ok().and_then(|g| g.clone());

            drop(guard); // 先释放引擎锁

            if let Some((det_path, rec_path)) = paths {
                init_paddle_engine_with_paths(&det_path, &rec_path)?;
            } else {
                return Err("Paddle OCR 模型未安装，请先安装模型".to_string());
            }
        }
    }

    // 现在引擎应该已初始化，重新获取锁进行识别
    let mut guard = PADDLE_ENGINE
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let engine = guard.as_mut().ok_or("Paddle OCR 引擎未初始化")?;

    let results = engine
        .recognize_file(image_path)
        .map_err(|e| format!("识别失败: {}", e))?;

    Ok(results
        .into_iter()
        .map(|r| OcrTextResult {
            text: r.text,
            confidence: r.confidence,
            bbox: BBox {
                x: r.bbox.x,
                y: r.bbox.y,
                w: r.bbox.w,
                h: r.bbox.h,
            },
        })
        .collect())
}
