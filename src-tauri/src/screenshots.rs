use std::fs;
use std::path::Path;

use crate::error::AppResult;
use crate::types::{system_time_to_ms, ScreenshotFile};

pub(crate) fn list_screenshot_files(dir: &Path) -> AppResult<Vec<ScreenshotFile>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_supported_image(&path) {
            continue;
        }
        let metadata = entry.metadata()?;
        files.push(ScreenshotFile {
            filename: entry.file_name().to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            modified_at_ms: system_time_to_ms(metadata.modified()?),
        });
    }

    files.sort_by(|a, b| b.modified_at_ms.cmp(&a.modified_at_ms));
    Ok(files)
}

pub(crate) fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "webp"
            )
        })
        .unwrap_or(false)
}
