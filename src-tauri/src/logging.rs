/// Routed through the `log` facade so every event reaches the configured
/// tauri-plugin-log sinks: stdout, the rolling log file in the app log dir,
/// and the webview console. The plugin stamps each line with local time.
pub(crate) fn log_event(scope: &str, message: impl AsRef<str>) {
    #[cfg(test)]
    {
        eprintln!("[{scope}] {}", message.as_ref());
    }
    log::info!(target: scope, "{}", message.as_ref());
}
