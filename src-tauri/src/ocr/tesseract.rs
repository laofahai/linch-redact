//! Tesseract OCR 引擎实现（CLI 包装）

use image::DynamicImage;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use crate::ocr::engine::OcrEngine;
use crate::ocr::types::{
    BBox, OcrAuditInfo, OcrEngineType, OcrTextResult, TesseractConfig, TesseractStatus,
};

/// Tesseract OCR 引擎
pub struct TesseractEngine {
    config: TesseractConfig,
    version: Option<String>,
}

impl TesseractEngine {
    /// 创建 Tesseract 引擎
    pub fn new(config: TesseractConfig) -> Result<Self, String> {
        // 验证 binary 是否可用
        let binary = config.binary_path.as_deref().unwrap_or("tesseract");
        let version = get_tesseract_version(binary)?;

        log::info!("[Tesseract] 初始化成功，版本: {}", version);

        Ok(Self {
            config,
            version: Some(version),
        })
    }

    fn binary_path(&self) -> &str {
        self.config.binary_path.as_deref().unwrap_or("tesseract")
    }
}

impl OcrEngine for TesseractEngine {
    fn recognize_image(&mut self, img: &DynamicImage) -> Result<Vec<OcrTextResult>, String> {
        let start = Instant::now();

        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let temp_id = std::process::id();
        let temp_input = temp_dir.join(format!("tesseract_input_{}.png", temp_id));

        // 保存图片到临时文件
        img.save(&temp_input)
            .map_err(|e| format!("保存临时图片失败: {}", e))?;

        // 调用 tesseract
        let results = self.recognize_file(temp_input.to_string_lossy().as_ref())?;

        // 清理临时文件
        let _ = std::fs::remove_file(&temp_input);

        log::info!(
            "[Tesseract] 识别完成，耗时: {} ms，结果数: {}",
            start.elapsed().as_millis(),
            results.len()
        );

        Ok(results)
    }

    fn recognize_file(&mut self, image_path: &str) -> Result<Vec<OcrTextResult>, String> {
        let start = Instant::now();

        // 构建命令
        let mut cmd = Command::new(self.binary_path());

        cmd.arg(image_path)
            .arg("stdout")
            .arg("-l")
            .arg(self.config.lang_or_default())
            .arg("--psm")
            .arg(self.config.psm_or_default().to_string())
            .arg("--oem")
            .arg(self.config.oem_or_default().to_string())
            .arg("tsv");

        // 设置 tessdata 路径
        if let Some(tessdata_path) = &self.config.tessdata_path {
            cmd.env("TESSDATA_PREFIX", tessdata_path);
        }

        log::info!(
            "[Tesseract] 执行: {} {} -l {} --psm {} --oem {} tsv",
            self.binary_path(),
            image_path,
            self.config.lang_or_default(),
            self.config.psm_or_default(),
            self.config.oem_or_default()
        );

        let output = cmd
            .output()
            .map_err(|e| format!("执行 tesseract 失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Tesseract 执行失败: {}", stderr));
        }

        let tsv_output = String::from_utf8_lossy(&output.stdout);

        // 获取图片尺寸用于归一化
        let img = image::open(image_path).map_err(|e| format!("读取图片失败: {}", e))?;
        let (img_width, img_height) = (img.width() as f32, img.height() as f32);

        // 解析 TSV 输出
        let results = parse_tesseract_tsv(&tsv_output, img_width, img_height)?;

        log::info!(
            "[Tesseract] 识别完成，耗时: {} ms，结果数: {}",
            start.elapsed().as_millis(),
            results.len()
        );

        Ok(results)
    }

    fn audit_info(&self) -> OcrAuditInfo {
        let params = serde_json::json!({
            "lang": self.config.lang_or_default(),
            "psm": self.config.psm_or_default(),
            "oem": self.config.oem_or_default(),
        });

        OcrAuditInfo {
            engine_type: OcrEngineType::Tesseract,
            engine_version: self.version.clone(),
            engine_params: Some(params.to_string()),
            tessdata_hash: self
                .config
                .tessdata_path
                .as_ref()
                .and_then(|p| compute_tessdata_hash(p).ok()),
        }
    }
}

/// 解析 Tesseract TSV 输出
///
/// TSV 格式：
/// level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
///
/// 返回单词级别的结果，每个词有独立的 bbox，便于精确遮盖
fn parse_tesseract_tsv(
    tsv: &str,
    img_width: f32,
    img_height: f32,
) -> Result<Vec<OcrTextResult>, String> {
    let mut results = Vec::new();

    for line in tsv.lines().skip(1) {
        // 跳过表头
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }

        let level: i32 = cols[0].parse().unwrap_or(-1);
        let left: f32 = cols[6].parse().unwrap_or(0.0);
        let top: f32 = cols[7].parse().unwrap_or(0.0);
        let width: f32 = cols[8].parse().unwrap_or(0.0);
        let height: f32 = cols[9].parse().unwrap_or(0.0);
        let conf: f32 = cols[10].parse().unwrap_or(-1.0);
        let text = cols[11].trim();

        // 只处理 word 级别 (level=5)，跳过空文本和低置信度
        if level != 5 || text.is_empty() || conf < 0.0 {
            continue;
        }

        // 归一化 bbox 到 0-1 范围
        let bbox = BBox {
            x: left / img_width,
            y: top / img_height,
            w: width / img_width,
            h: height / img_height,
        };

        results.push(OcrTextResult {
            text: text.to_string(),
            confidence: conf / 100.0, // Tesseract 置信度是 0-100
            bbox,
        });
    }

    Ok(results)
}

/// 获取 Tesseract 版本
pub fn get_tesseract_version(binary_path: &str) -> Result<String, String> {
    let output = Command::new(binary_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("无法执行 tesseract: {}", e))?;

    if !output.status.success() {
        return Err("tesseract --version 执行失败".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // 解析版本号（通常在第一行）
    for line in combined.lines() {
        if line.contains("tesseract") {
            // 格式通常是 "tesseract 5.3.0" 或 "tesseract v5.3.0"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return Ok(parts[1].trim_start_matches('v').to_string());
            }
        }
    }

    Ok("unknown".to_string())
}

/// 获取 Tesseract 可用语言列表
pub fn get_tesseract_langs(
    binary_path: &str,
    tessdata_path: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut cmd = Command::new(binary_path);
    cmd.arg("--list-langs");

    if let Some(path) = tessdata_path {
        cmd.env("TESSDATA_PREFIX", path);
    }

    let output = cmd.output().map_err(|e| format!("执行失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    let mut langs = Vec::new();
    let mut found_list = false;

    for line in combined.lines() {
        let line = line.trim();
        if line.contains("List of available languages") || line.contains("traineddata") {
            found_list = true;
            continue;
        }
        if found_list && !line.is_empty() && !line.contains(':') {
            langs.push(line.to_string());
        }
    }

    Ok(langs)
}

/// 检测 Tesseract 安装状态
pub fn detect_tesseract_status(config: &TesseractConfig) -> TesseractStatus {
    // 首先尝试用户配置的路径，然后尝试 PATH 中的 tesseract，最后尝试常见安装路径
    let binary_path = config.binary_path.as_deref().unwrap_or("tesseract");

    // 尝试获取版本（使用配置的路径或 PATH）
    if let Ok(version) = get_tesseract_version(binary_path) {
        let langs =
            get_tesseract_langs(binary_path, config.tessdata_path.as_deref()).unwrap_or_default();
        let actual_path = which_tesseract(binary_path);
        let tessdata = config
            .tessdata_path
            .clone()
            .or_else(|| find_tessdata_path(binary_path));

        return TesseractStatus {
            installed: true,
            version: Some(version),
            binary_path: actual_path,
            tessdata_path: tessdata,
            available_langs: langs,
            error: None,
        };
    }

    // 如果直接检测失败，尝试查找常见安装路径
    if let Some(found_path) = which_tesseract("tesseract") {
        if let Ok(version) = get_tesseract_version(&found_path) {
            let langs = get_tesseract_langs(&found_path, config.tessdata_path.as_deref())
                .unwrap_or_default();
            let tessdata = config
                .tessdata_path
                .clone()
                .or_else(|| find_tessdata_path(&found_path));

            return TesseractStatus {
                installed: true,
                version: Some(version),
                binary_path: Some(found_path),
                tessdata_path: tessdata,
                available_langs: langs,
                error: None,
            };
        }
    }

    TesseractStatus {
        installed: false,
        version: None,
        binary_path: None,
        tessdata_path: None,
        available_langs: Vec::new(),
        error: Some("无法检测到 Tesseract，请确认已安装并正确配置".to_string()),
    }
}

/// 查找 tesseract 可执行文件的完整路径
fn which_tesseract(binary: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // 首先尝试 where 命令
        let cmd = Command::new("where").arg(binary).output();
        if let Some(path) = cmd
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
        {
            return Some(path);
        }

        // 如果 where 找不到，检查常见安装路径
        let common_paths = [
            "C:\\Program Files\\Tesseract-OCR\\tesseract.exe",
            "C:\\Program Files (x86)\\Tesseract-OCR\\tesseract.exe",
        ];

        for path in common_paths {
            if Path::new(path).exists() {
                return Some(path.to_string());
            }
        }

        // 尝试通过环境变量
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            let path = format!("{}\\Tesseract-OCR\\tesseract.exe", program_files);
            if Path::new(&path).exists() {
                return Some(path);
            }
        }

        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        let cmd = Command::new("which").arg(binary).output();
        cmd.ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    }
}

/// 查找 tessdata 路径
fn find_tessdata_path(binary_path: &str) -> Option<String> {
    // 尝试从环境变量获取
    if let Ok(path) = std::env::var("TESSDATA_PREFIX") {
        if Path::new(&path).exists() {
            return Some(path);
        }
    }

    // 尝试从 tesseract 输出中获取
    let output = Command::new(binary_path)
        .arg("--print-parameters")
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("tessdata") {
            // 尝试解析路径
            if let Some(path) = line.split_whitespace().last() {
                let path = path.trim_matches('"');
                if Path::new(path).exists() {
                    return Some(path.to_string());
                }
            }
        }
    }

    // 常见默认路径
    #[cfg(target_os = "windows")]
    let common_paths: Vec<std::path::PathBuf> = {
        let mut paths = Vec::new();
        // Windows 常见安装路径
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            paths.push(
                Path::new(&program_files)
                    .join("Tesseract-OCR")
                    .join("tessdata"),
            );
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            paths.push(
                Path::new(&program_files_x86)
                    .join("Tesseract-OCR")
                    .join("tessdata"),
            );
        }
        // 硬编码备用路径
        paths.push(Path::new("C:\\Program Files\\Tesseract-OCR\\tessdata").to_path_buf());
        paths.push(Path::new("C:\\Program Files (x86)\\Tesseract-OCR\\tessdata").to_path_buf());
        paths
    };

    #[cfg(not(target_os = "windows"))]
    let common_paths: Vec<std::path::PathBuf> = vec![
        Path::new("/usr/share/tesseract-ocr/5/tessdata").to_path_buf(),
        Path::new("/usr/share/tesseract-ocr/4.00/tessdata").to_path_buf(),
        Path::new("/usr/share/tessdata").to_path_buf(),
        Path::new("/usr/local/share/tessdata").to_path_buf(),
        Path::new("/opt/homebrew/share/tessdata").to_path_buf(),
    ];

    for path in common_paths {
        if path.exists() {
            return path.to_string_lossy().to_string().into();
        }
    }

    None
}

/// 获取当前操作系统类型
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // 各平台只使用对应的 variant
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return Platform::Unknown;
    }
}

/// Tesseract 安装进度事件
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesseractInstallProgress {
    pub stage: String,
    pub message: String,
    pub done: bool,
    pub success: bool,
    pub error: Option<String>,
}

/// 安装 Tesseract（根据平台自动选择方式）
pub async fn install_tesseract<F>(progress_callback: F) -> Result<(), String>
where
    F: Fn(TesseractInstallProgress) + Send + 'static,
{
    let platform = Platform::current();

    match platform {
        Platform::Linux => install_tesseract_linux(progress_callback).await,
        Platform::MacOS => install_tesseract_macos(progress_callback).await,
        Platform::Windows => install_tesseract_windows(progress_callback).await,
        Platform::Unknown => Err("不支持的操作系统".to_string()),
    }
}

/// 检测是否在 WSL 环境中运行
fn is_wsl() -> bool {
    // 检查 /proc/version 是否包含 WSL 或 Microsoft
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        return version.to_lowercase().contains("wsl")
            || version.to_lowercase().contains("microsoft");
    }
    false
}

/// Linux 安装 Tesseract
async fn install_tesseract_linux<F>(progress_callback: F) -> Result<(), String>
where
    F: Fn(TesseractInstallProgress) + Send + 'static,
{
    // 检测包管理器和对应的安装命令
    let install_script = if Command::new("apt").arg("--version").output().is_ok() {
        "sudo apt install -y tesseract-ocr tesseract-ocr-chi-sim tesseract-ocr-eng"
    } else if Command::new("dnf").arg("--version").output().is_ok() {
        "sudo dnf install -y tesseract tesseract-langpack-chi_sim tesseract-langpack-eng"
    } else if Command::new("pacman").arg("--version").output().is_ok() {
        "sudo pacman -S --noconfirm tesseract tesseract-data-chi_sim tesseract-data-eng"
    } else {
        return Err("未检测到支持的包管理器 (apt/dnf/pacman)".to_string());
    };

    // WSL 环境：返回手动安装命令
    if is_wsl() {
        progress_callback(TesseractInstallProgress {
            stage: "wsl".to_string(),
            message: install_script.to_string(),
            done: true,
            success: false,
            error: Some("WSL 环境需要手动安装".to_string()),
        });
        return Err(format!("WSL_MANUAL:{}", install_script));
    }

    progress_callback(TesseractInstallProgress {
        stage: "install".to_string(),
        message: "正在打开终端安装...".to_string(),
        done: false,
        success: false,
        error: None,
    });

    // 尝试不同的终端模拟器
    let terminals = [
        ("gnome-terminal", vec!["--", "bash", "-c"]),
        ("konsole", vec!["-e", "bash", "-c"]),
        ("xfce4-terminal", vec!["-e", "bash -c"]),
        ("xterm", vec!["-e", "bash", "-c"]),
        ("x-terminal-emulator", vec!["-e", "bash", "-c"]),
    ];

    let full_script = format!(
        "{}; echo ''; echo '安装完成，按回车键关闭此窗口...'; read",
        install_script
    );

    for (terminal, args) in &terminals {
        if Command::new("which")
            .arg(terminal)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let mut cmd = Command::new(terminal);
            for arg in args {
                cmd.arg(arg);
            }
            cmd.arg(&full_script);

            match cmd.spawn() {
                Ok(_) => {
                    progress_callback(TesseractInstallProgress {
                        stage: "waiting".to_string(),
                        message: "已打开终端，请在终端中输入密码完成安装".to_string(),
                        done: true,
                        success: true,
                        error: None,
                    });
                    return Ok(());
                }
                Err(_) => continue,
            }
        }
    }

    // 如果没有找到终端模拟器，尝试 pkexec
    progress_callback(TesseractInstallProgress {
        stage: "install".to_string(),
        message: "尝试使用 pkexec 安装...".to_string(),
        done: false,
        success: false,
        error: None,
    });

    let install_cmd = if Command::new("apt").arg("--version").output().is_ok() {
        vec![
            "apt",
            "install",
            "-y",
            "tesseract-ocr",
            "tesseract-ocr-chi-sim",
            "tesseract-ocr-eng",
        ]
    } else if Command::new("dnf").arg("--version").output().is_ok() {
        vec![
            "dnf",
            "install",
            "-y",
            "tesseract",
            "tesseract-langpack-chi_sim",
            "tesseract-langpack-eng",
        ]
    } else {
        vec![
            "pacman",
            "-S",
            "--noconfirm",
            "tesseract",
            "tesseract-data-chi_sim",
            "tesseract-data-eng",
        ]
    };

    let mut cmd = Command::new("pkexec");
    for arg in &install_cmd {
        cmd.arg(arg);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("执行安装命令失败: {}", e))?;

    if output.status.success() {
        progress_callback(TesseractInstallProgress {
            stage: "complete".to_string(),
            message: "Tesseract 安装成功！".to_string(),
            done: true,
            success: true,
            error: None,
        });
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = if stderr.contains("dismiss")
            || stderr.contains("cancel")
            || stderr.contains("Not authorized")
        {
            "用户取消了安装".to_string()
        } else if stderr.contains("textual authentication") || stderr.contains("/dev/tty") {
            "无法获取授权，请手动在终端中运行安装命令".to_string()
        } else {
            format!("安装失败: {}", stderr)
        };
        progress_callback(TesseractInstallProgress {
            stage: "error".to_string(),
            message: error_msg.clone(),
            done: true,
            success: false,
            error: Some(error_msg.clone()),
        });
        Err(error_msg)
    }
}

/// macOS 安装 Tesseract（使用 Homebrew）
async fn install_tesseract_macos<F>(progress_callback: F) -> Result<(), String>
where
    F: Fn(TesseractInstallProgress) + Send + 'static,
{
    progress_callback(TesseractInstallProgress {
        stage: "prepare".to_string(),
        message: "检查 Homebrew...".to_string(),
        done: false,
        success: false,
        error: None,
    });

    // 检查 Homebrew 是否安装
    let brew_check = Command::new("brew").arg("--version").output();
    if brew_check.is_err() || !brew_check.unwrap().status.success() {
        let error_msg = "未检测到 Homebrew，请先安装 Homebrew: https://brew.sh".to_string();
        progress_callback(TesseractInstallProgress {
            stage: "error".to_string(),
            message: error_msg.clone(),
            done: true,
            success: false,
            error: Some(error_msg.clone()),
        });
        return Err(error_msg);
    }

    progress_callback(TesseractInstallProgress {
        stage: "install".to_string(),
        message: "使用 Homebrew 安装 Tesseract...".to_string(),
        done: false,
        success: false,
        error: None,
    });

    // 安装 tesseract 和语言包
    let output = Command::new("brew")
        .args(["install", "tesseract", "tesseract-lang"])
        .output()
        .map_err(|e| format!("执行 brew install 失败: {}", e))?;

    if output.status.success() {
        progress_callback(TesseractInstallProgress {
            stage: "complete".to_string(),
            message: "Tesseract 安装成功！".to_string(),
            done: true,
            success: true,
            error: None,
        });
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("Homebrew 安装失败: {}", stderr);
        progress_callback(TesseractInstallProgress {
            stage: "error".to_string(),
            message: error_msg.clone(),
            done: true,
            success: false,
            error: Some(error_msg.clone()),
        });
        Err(error_msg)
    }
}

/// Windows 安装 Tesseract（尝试 winget，否则打开下载页面）
async fn install_tesseract_windows<F>(progress_callback: F) -> Result<(), String>
where
    F: Fn(TesseractInstallProgress) + Send + 'static,
{
    progress_callback(TesseractInstallProgress {
        stage: "prepare".to_string(),
        message: "检查 winget...".to_string(),
        done: false,
        success: false,
        error: None,
    });

    // 尝试使用 winget
    let winget_check = Command::new("winget").arg("--version").output();

    if winget_check.is_ok() && winget_check.unwrap().status.success() {
        progress_callback(TesseractInstallProgress {
            stage: "install".to_string(),
            message: "正在下载并安装 Tesseract，可能需要几分钟...".to_string(),
            done: false,
            success: false,
            error: None,
        });

        // 使用 spawn 启动安装进程，不阻塞等待
        // 让用户可以看到安装界面并交互
        let result = Command::new("winget")
            .args([
                "install",
                "--id",
                "UB-Mannheim.TesseractOCR",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ])
            .spawn();

        match result {
            Ok(_child) => {
                progress_callback(TesseractInstallProgress {
                    stage: "installing".to_string(),
                    message:
                        "Tesseract 安装程序已启动，请按照安装向导完成安装。安装完成后点击刷新按钮。"
                            .to_string(),
                    done: true,
                    success: true,
                    error: None,
                });
                return Ok(());
            }
            Err(e) => {
                let error_msg = format!("启动安装程序失败: {}", e);
                progress_callback(TesseractInstallProgress {
                    stage: "error".to_string(),
                    message: error_msg.clone(),
                    done: true,
                    success: false,
                    error: Some(error_msg.clone()),
                });
                return Err(error_msg);
            }
        }
    }

    // winget 不可用，提示手动下载
    let download_url = "https://github.com/UB-Mannheim/tesseract/wiki";

    // 尝试打开下载页面
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/c", "start", download_url])
            .spawn();
    }

    progress_callback(TesseractInstallProgress {
        stage: "manual".to_string(),
        message: format!(
            "已打开下载页面，请下载安装后点击刷新按钮。\n下载地址: {}",
            download_url
        ),
        done: true,
        success: false,
        error: Some("MANUAL_DOWNLOAD".to_string()),
    });

    Err("MANUAL_DOWNLOAD".to_string())
}

/// 计算 tessdata 目录的简单 hash（用于审计）
fn compute_tessdata_hash(tessdata_path: &str) -> Result<String, String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let path = Path::new(tessdata_path);
    if !path.exists() {
        return Err("tessdata 路径不存在".to_string());
    }

    let mut hasher = DefaultHasher::new();

    // 列出所有 .traineddata 文件
    let entries: Vec<_> = std::fs::read_dir(path)
        .map_err(|e| format!("读取目录失败: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "traineddata")
                .unwrap_or(false)
        })
        .collect();

    // 对文件名和大小进行 hash
    for entry in entries {
        let name = entry.file_name();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        name.hash(&mut hasher);
        size.hash(&mut hasher);
    }

    Ok(format!("{:x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tsv_word_level() {
        // 测试单词级别解析：每个 level=5 的单词独立返回
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	1	1	1	1	100	200	50	20	95.5	Hello
5	1	1	1	1	2	160	200	60	20	92.3	World
5	1	1	1	2	1	100	250	100	20	88.0	Test
"#;
        let results = parse_tesseract_tsv(tsv, 1000.0, 1000.0).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].text, "Hello");
        assert_eq!(results[1].text, "World");
        assert_eq!(results[2].text, "Test");

        // 验证 bbox 归一化
        assert!((results[0].bbox.x - 0.1).abs() < 0.001); // 100/1000 = 0.1
        assert!((results[0].bbox.y - 0.2).abs() < 0.001); // 200/1000 = 0.2
        assert!((results[0].bbox.w - 0.05).abs() < 0.001); // 50/1000 = 0.05
    }
}
