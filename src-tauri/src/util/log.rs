use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Write a debug message to a log file.
/// - Linux: $HOME/.local/share/pku-course-desktop/pku-course-desktop.log
/// - macOS: PKU Course Desktop.app/Contents/MacOS/pku-course-desktop.log
/// - Windows: same directory as the app executable
pub fn debug_log(msg: &str) {
    let ts = chrono::Local::now().format("%H:%M:%S%.3f");

    let log_path = if cfg!(target_os = "linux") {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pku-course-desktop");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("pku-course-desktop.log")
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pku-course-desktop.log")
    };

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "{} [DEBUG] {}", ts, msg);
    }
    eprintln!("{} [DEBUG] {}", ts, msg);
}
