use tauri::{AppHandle, Emitter};

use crate::logging::log_event;
use crate::types::*;

pub(crate) fn emit_ocr_progress(app: &AppHandle, message: &str) {
    emit_ocr_progress_event(
        app,
        OcrProgressEvent::Message(OcrMessageData {
            message: message.to_string(),
        }),
    );
    log_event("OCR", message);
}

pub(crate) fn emit_ocr_batch_started(app: &AppHandle, total: usize) {
    let message = format!("Processing {total} screenshot(s)");
    emit_ocr_progress_event(
        app,
        OcrProgressEvent::BatchStarted(OcrBatchStartedData { total, message }),
    );
}

pub(crate) fn emit_ocr_screenshot_started(
    app: &AppHandle,
    file: &ScreenshotFile,
    index: usize,
    total: usize,
) {
    emit_ocr_progress_event(
        app,
        OcrProgressEvent::ScreenshotStarted(OcrScreenshotStartedData {
            path: file.path.clone(),
            filename: file.filename.clone(),
            index,
            total,
            message: format!("Processing {} ({index}/{total})", file.filename),
        }),
    );
}

pub(crate) fn emit_ocr_screenshot_succeeded(app: &AppHandle, file: &ScreenshotFile) {
    emit_ocr_progress_event(
        app,
        OcrProgressEvent::ScreenshotSucceeded(OcrScreenshotSucceededData {
            path: file.path.clone(),
            filename: file.filename.clone(),
            message: format!("Done {}", file.filename),
        }),
    );
}

pub(crate) fn emit_ocr_screenshot_failed(app: &AppHandle, file: &ScreenshotFile, error: &str) {
    emit_ocr_progress_event(
        app,
        OcrProgressEvent::ScreenshotFailed(OcrScreenshotFailedData {
            path: file.path.clone(),
            filename: file.filename.clone(),
            error: error.to_string(),
            message: format!("Failed {}: {error}", file.filename),
        }),
    );
}

pub(crate) fn emit_ocr_finished(app: &AppHandle, processed: usize, warnings: usize) {
    let message = format!("Finished. {processed} processed, {warnings} warnings");
    emit_ocr_progress_event(
        app,
        OcrProgressEvent::Finished(OcrFinishedData {
            processed,
            warnings,
            message,
        }),
    );
}

fn emit_ocr_progress_event(app: &AppHandle, event: OcrProgressEvent) {
    let _ = app.emit("ocr-progress", event);
}
