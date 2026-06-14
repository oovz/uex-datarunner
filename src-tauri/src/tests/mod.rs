mod fixtures;
mod live_ocr;
mod static_analysis;
mod unit_tests;

use std::path::{Path, PathBuf};

pub(crate) fn temp_test_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("uex-datarunner-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&path).expect("test temp directory should be writable");
    path
}

pub(crate) fn asset_path(filename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join("asset")
        .join(filename)
}
