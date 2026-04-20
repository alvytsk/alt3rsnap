use std::path::PathBuf;

#[allow(dead_code)]
pub fn init() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let file_appender = tracing_appender::rolling::daily(&dir, "alt3rsnap.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("ALT3RSNAP_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    Some(guard)
}

#[allow(dead_code)]
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::fs::create_dir_all(log_dir());
        let crash_path = log_dir().join("crash.log");
        let payload = format!("{info}\n{}", std::backtrace::Backtrace::force_capture());
        let _ = std::fs::write(&crash_path, &payload);
        tracing::error!(target: "panic", %payload);
        default(info);
    }));
}

#[allow(dead_code)]
pub fn log_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("com", "Alt3rSnap", "Alt3rSnap") {
        return dirs.data_dir().join("logs");
    }
    PathBuf::from("logs")
}
