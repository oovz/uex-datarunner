use std::path::Path;

#[test]
fn ocr_backend_uses_foundry_local_cuda_without_windows_ocr_or_windows_ai_dependency() {
    let main_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("lib.rs should be readable");
    let ocr_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("mod.rs"),
    )
    .expect("foundry_ocr/mod.rs should be readable");
    let source = format!("{main_source}\n{ocr_source}");
    let cargo_toml =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("Cargo.toml should be readable");
    let build_rs = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
        .expect("build.rs should be readable");

    assert!(
        source.contains("FoundryLocalManager")
            && source.contains("CUDA_VISION_MODEL_ALIAS")
            && cargo_toml.contains("foundry-local-sdk")
            && !cargo_toml.contains("winml"),
        "OCR backend should use Foundry Local without the stale Windows ML feature path"
    );
    for disallowed_reference in [
        "Microsoft.Windows.AI",
        "TextRecognizer",
        "ImageBuffer::CreateForSoftwareBitmap",
        "windows-bindgen",
        "systemAIModels",
        "Windows.Media.Ocr",
        "Media::Ocr",
        "OcrEngine",
        "TryCreateFromUserProfileLanguages",
        "Media_Ocr",
        "DirectML",
        "Windows ML DirectML",
        "DmlExecutionProvider",
    ] {
        assert!(
            !source.contains(disallowed_reference),
            "OCR backend must not reference removed Windows OCR/Windows AI API: {disallowed_reference}"
        );
        assert!(
            !cargo_toml.contains(disallowed_reference),
            "Cargo dependencies must not require removed Windows OCR/Windows AI API: {disallowed_reference}"
        );
        assert!(
            !build_rs.contains(disallowed_reference),
            "build script must not generate removed Windows OCR/Windows AI bindings: {disallowed_reference}"
        );
    }
}

#[test]
fn package_scripts_do_not_require_windows_ai_package_identity_for_ocr() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri should have a repository parent");
    let package_json = std::fs::read_to_string(repo_root.join("package.json"))
        .expect("package.json should be readable");
    let package_scripts = serde_json::from_str::<serde_json::Value>(&package_json)
        .expect("package.json should be valid JSON");
    let package_scripts = package_scripts
        .get("scripts")
        .and_then(serde_json::Value::as_object)
        .expect("package.json should contain scripts");

    assert!(
        !package_scripts.contains_key("test:ocr:identity")
            && !package_scripts.contains_key("tauri:dev:withidentity")
            && !package_scripts.contains_key("identity:run"),
        "Foundry Local OCR must not require winapp package identity scripts"
    );
}

#[test]
fn foundry_prompt_requests_structured_commodity_output_without_guessing() {
    let main_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("lib.rs should be readable");
    let ocr_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("mod.rs"),
    )
    .expect("foundry_ocr/mod.rs should be readable");
    let source = format!("{main_source}\n{ocr_source}");

    assert!(source.contains("\"marketSide\""));
    assert!(source.contains("\"cargoSizes\""));
    // Foundry Local's local endpoint does not enforce json_schema for this
    // vision model, so we enforce structure with a compact output contract and
    // explicitly demand raw JSON.
    assert!(
        source.contains("raw JSON object"),
        "OCR prompt must explicitly demand raw JSON output"
    );
    assert!(source.contains("not translated"));
    for unsupported_control in ["/no_think", "enable_thinking", "chat_template_kwargs"] {
        assert!(
            !source.contains(unsupported_control),
            "Foundry Local OCR prompt/request must not rely on unsupported thinking control: {unsupported_control}"
        );
    }
}

#[test]
fn ocr_model_loading_does_not_fall_back_to_cpu_variants() {
    let main_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("lib.rs should be readable");
    let ocr_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("mod.rs"),
    )
    .expect("foundry_ocr/mod.rs should be readable");
    let source = format!("{main_source}\n{ocr_source}");

    for disallowed_reference in [
        "CPU fallback",
        "Attempting CPU",
        "Downloading CPU",
        "Use this CPU variant",
        "No CPU fallback",
    ] {
        assert!(
            !source.contains(disallowed_reference),
            "OCR backend must reject CPU model variants instead of falling back to them: {disallowed_reference}"
        );
    }
}

#[test]
fn foundry_ocr_does_not_remove_cached_models_owned_by_other_scenarios() {
    let main_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("lib.rs should be readable");
    let ocr_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("mod.rs"),
    )
    .expect("foundry_ocr/mod.rs should be readable");
    let source = format!("{main_source}\n{ocr_source}");

    assert!(
        !source.contains("remove_from_cache")
            && !source.contains("Removing unsupported cached Foundry model"),
        "OCR startup must validate the selected model variant without deleting other cached Foundry models"
    );
}

#[test]
fn foundry_pipeline_has_no_recovery_attempts() {
    let mod_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("mod.rs"),
    )
    .expect("mod.rs should be readable");

    assert!(
        !mod_source.contains("has_attempted_recovery"),
        "batch processing must NOT track recovery attempts since recovery is disabled"
    );
    assert!(
        !mod_source.contains("recovery already attempted"),
        "batch processing must NOT have recovery warning logic"
    );
}

#[test]
fn foundry_health_check_targets_v1_models_endpoint() {
    let client_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("client.rs"),
    )
    .expect("client.rs should be readable");

    assert!(
        client_source.contains("/v1/models"),
        "health check must probe the /v1/models endpoint as a lightweight readiness test"
    );
}
