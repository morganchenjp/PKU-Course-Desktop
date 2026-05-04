/// Thin wrapper around the `log` crate's `debug!` macro.
///
/// Previously this wrote to a hand-rolled log file + stderr.  Now it delegates
/// to `log::debug!` which is captured by `tauri_plugin_log` (configured in
/// `main.rs` with `TargetKind::Stdout` + `TargetKind::LogDir`).  The crate
/// target log level is set to `Debug` for `pku_course_desktop`, so these
/// messages are emitted even when the global filter is `Info`.
pub fn debug_log(msg: &str) {
    log::debug!("{msg}");
}
