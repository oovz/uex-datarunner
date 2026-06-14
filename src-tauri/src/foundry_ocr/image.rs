use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::error::AppResult;
use base64::{engine::general_purpose, Engine};
use image::{imageops::FilterType, DynamicImage, ImageFormat};

pub(crate) fn prepare_right_panel_ocr_images_in(
    input: &Path,
    output_dir: &Path,
    stem: &str,
    model_id: &str,
) -> AppResult<Vec<PathBuf>> {
    std::fs::create_dir_all(output_dir)?;
    let panel = right_panel_crop(input, 58, 13, 82, super::OCR_IMAGE_MAX_SIDE, model_id)?;
    let output = output_dir.join(format!("{stem}-panel.png"));
    panel.save(&output)?;

    Ok(vec![output])
}

pub(crate) fn right_panel_crop(
    input: &Path,
    crop_x_percent: u32,
    crop_y_percent: u32,
    crop_height_percent: u32,
    _max_dimension: f64,
    _model_id: &str,
) -> AppResult<DynamicImage> {
    let image = load_image_by_content(input)?;
    let width = image.width();
    let height = image.height();
    let crop_x = width.saturating_mul(crop_x_percent) / 100;
    let crop_width = width.saturating_sub(crop_x);
    let crop_y = height.saturating_mul(crop_y_percent) / 100;
    let crop_height = height.saturating_mul(crop_height_percent) / 100;
    let cropped = image.crop_imm(crop_x, crop_y, crop_width, crop_height);

    // Return the high-resolution crop directly to avoid double-resizing.
    // The scaling is performed once at the end in foundry_image_data.
    Ok(cropped)
}

pub(crate) fn load_image_by_content(input: &Path) -> AppResult<DynamicImage> {
    let bytes = std::fs::read(input)?;
    Ok(image::load_from_memory(&bytes)?)
}

pub(crate) fn foundry_image_data(image_path: &Path, model_id: &str) -> AppResult<String> {
    let image = load_image_by_content(image_path)?;
    let (w_final, h_final) = adjust_dimensions_for_model(
        image.width(),
        image.height(),
        model_id,
        super::OCR_IMAGE_MAX_SIDE,
    );
    let prepared = image.resize_exact(w_final, h_final, FilterType::Lanczos3);

    let mut bytes = Cursor::new(Vec::new());
    prepared.write_to(&mut bytes, ImageFormat::Jpeg)?;
    Ok(general_purpose::STANDARD.encode(bytes.into_inner()))
}

pub(crate) fn adjust_dimensions_for_model(
    width: u32,
    height: u32,
    model_id: &str,
    max_dimension: f64,
) -> (u32, u32) {
    let model_lower = model_id.to_lowercase();
    let factor = if model_lower.contains("qwen3") || model_lower.contains("qwen-3") {
        32
    } else {
        28
    };

    let longest = width.max(height) as f64;
    let scale = if longest > max_dimension {
        max_dimension / longest
    } else {
        1.0
    };

    let w_scaled = (width as f64 * scale).round() as u32;
    let h_scaled = (height as f64 * scale).round() as u32;

    let w_final = (((w_scaled as f32 / factor as f32).round() as u32) * factor).max(factor);
    let h_final = (((h_scaled as f32 / factor as f32).round() as u32) * factor).max(factor);

    (w_final, h_final)
}
