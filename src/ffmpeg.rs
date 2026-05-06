pub fn ffmpeg_last_line(log_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(log_path).ok()?;
    content.split('\r').filter(|s| !s.trim().is_empty()).next_back()
        .map(|s| s.trim().to_string())
}
