use ort::session::builder::SessionBuilder;
use ort::Error;

const DEFAULT_MAX_THREADS: usize = 4;

fn parse_env_usize(key: &str) -> Option<usize> {
    std::env::var(key).ok()?.parse::<usize>().ok()
}

fn default_thread_count() -> usize {
    let available = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(DEFAULT_MAX_THREADS);
    available.min(DEFAULT_MAX_THREADS).max(1)
}

pub fn thread_settings() -> (usize, usize) {
    let intra = parse_env_usize("LINCH_OCR_THREADS")
        .filter(|v| *v > 0)
        .unwrap_or_else(default_thread_count);
    let inter = parse_env_usize("LINCH_OCR_INTER_THREADS")
        .filter(|v| *v > 0)
        .unwrap_or(1);
    (intra, inter)
}

fn set_env_if_missing(key: &str, value: &str) {
    if std::env::var_os(key).is_none() {
        std::env::set_var(key, value);
    }
}

pub fn apply_session_threads(builder: SessionBuilder) -> Result<SessionBuilder, Error> {
    let (intra, inter) = thread_settings();

    set_env_if_missing("OMP_NUM_THREADS", &intra.to_string());
    set_env_if_missing("ORT_NUM_THREADS", &intra.to_string());

    log::info!("[OCR] 线程设置: intra={}, inter={}", intra, inter);

    let builder = builder.with_intra_threads(intra)?;
    let builder = builder.with_inter_threads(inter)?;
    builder.with_parallel_execution(false)
}
