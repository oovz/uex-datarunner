use std::path::Path;

use serde_json::json;

use crate::{
    commodity_extraction_schema, extract_live_game_version, filter_commodities, filter_terminals,
    find_cuda_runtime_dir_in_roots, foundry_ocr_messages, is_foundry_transport_failure,
    merge_foundry_ocr_results, normalize_foundry_ocr_json, parse_uex_data_array,
    parse_uex_data_object, prepare_right_panel_ocr_images_in, select_foundry_vision_variant,
    strip_model_reasoning, terminal_matches_query, validate_foundry_ocr_json, AppConfig, AppError,
    FoundryVariantChoice, UexCommodity, UexTerminal, CUDA_VISION_MODEL_ALIAS,
    FOUNDRY_OCR_HTTP_TIMEOUT_SECS, FOUNDRY_OCR_MAX_TOKENS, QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS,
};

use super::{asset_path, temp_test_dir};

#[test]
fn app_config_defaults_to_the_primary_qwen35_foundry_model_alias() {
    let config = AppConfig::default();

    // MVP scope: Qwen 3.5 4B (CUDA) is the single primary model.
    assert_eq!(config.ai_model, CUDA_VISION_MODEL_ALIAS);
    assert_eq!(config.ai_model, crate::default_ai_model());
}

#[test]
fn foundry_keep_loaded_mode_reuses_loaded_model_and_web_service() {
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
    let ocr_model_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("model.rs"),
    )
    .expect("foundry_ocr/model.rs should be readable");
    let source = format!("{main_source}\n{ocr_source}\n{ocr_model_source}");

    assert!(
        source.contains("model.is_loaded().await?"),
        "OCR startup should skip model.load() when the selected Foundry model is already loaded"
    );
    assert!(
        source.contains("Reusing running Foundry Local web service"),
        "repeat OCR runs should reuse an already-running Foundry Local web service"
    );
    assert!(
        source.contains("Keeping Foundry Local model and web service loaded"),
        "keep_model_loaded should preserve both the model and web service for repeat runs"
    );
}

#[test]
fn foundry_ocr_messages_attach_the_screenshot_as_an_image_data_uri() {
    let body = foundry_ocr_messages("test-model", "abc123").expect("OCR request body should build");

    // The installed Foundry Local vision runtime accepts image input through
    // /v1/responses with media_type + image_data. Pure OpenAI image_url data
    // URIs either fail schema validation or route through a much heavier path.
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["max_output_tokens"], crate::FOUNDRY_OCR_MAX_TOKENS);
    assert!(
        body.get("instructions").is_some(),
        "system instructions must be present"
    );

    let input = body["input"].as_array().expect("input must be an array");
    assert_eq!(input.len(), 1, "one user message is sent");
    assert_eq!(input[0]["type"], "message");
    assert_eq!(input[0]["role"], "user");

    let content = input[0]["content"]
        .as_array()
        .expect("content must be an array");
    assert_eq!(content[0]["type"], "input_text");
    let prompt_text = content[0]["text"]
        .as_str()
        .expect("prompt text should be present");
    assert!(
        prompt_text.contains("Return a raw JSON object"),
        "prompt text should request raw JSON output"
    );
    assert!(
        !prompt_text.contains("/no_think"),
        "prompt text must not rely on unsupported Qwen thinking controls"
    );
    assert_eq!(content[1]["type"], "input_image");
    assert_eq!(content[1]["media_type"], "image/jpeg");
    assert_eq!(content[1]["image_data"], "abc123");
    assert!(body.get("enable_thinking").is_none());
    assert!(body.get("chat_template_kwargs").is_none());
}

#[test]
fn cuda_runtime_detection_accepts_cuda_toolkit_bin_layout() {
    let root = temp_test_dir("cuda-runtime-layout");
    let cuda_bin = root.join("CUDA").join("v12.9").join("bin");
    std::fs::create_dir_all(&cuda_bin).expect("test CUDA bin directory should be writable");
    std::fs::write(cuda_bin.join("cudart64_12.dll"), b"")
        .expect("test CUDA DLL should be writable");

    let detected = find_cuda_runtime_dir_in_roots([cuda_bin.as_path()])
        .expect("CUDA toolkit bin layout should be detected");

    assert_eq!(detected, cuda_bin);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn foundry_ocr_request_builder_does_not_create_retry_prompts() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("lib.rs should be readable");

    assert!(
        !source.contains("FOUNDRY_OCR_RETRY_PROMPT")
            && !source.contains("foundry_ocr_messages_for_attempt")
            && !source.contains("Retrying invalid JSON OCR response")
            && !source.contains("should_restart_foundry_after_ocr_error"),
        "OCR must not retry slow AI model calls or maintain alternate retry prompts"
    );
}

#[test]
fn commodity_extraction_schema_is_strict_and_lists_every_row_field() {
    let schema = commodity_extraction_schema();

    assert_eq!(
        schema.get("type").and_then(|value| value.as_str()),
        Some("object")
    );
    assert_eq!(
        schema
            .get("additionalProperties")
            .and_then(|value| value.as_bool()),
        Some(false),
        "the root object must forbid extra properties so decoding stays constrained"
    );

    let item = schema
        .get("properties")
        .and_then(|properties| properties.get("commodities"))
        .and_then(|commodities| commodities.get("items"))
        .expect("schema should describe commodity row objects");
    assert_eq!(
        item.get("additionalProperties")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let required: Vec<&str> = item
        .get("required")
        .and_then(|value| value.as_array())
        .expect("commodity rows should list required fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for field in ["name", "status", "scu", "pricePerScu", "cargoSizes"] {
        assert!(
            required.contains(&field),
            "schema must require the {field} field"
        );
    }
}

#[test]
fn right_panel_crop_step_produces_a_panel_image_for_ocr() {
    let source = asset_path("screenshot-eng-hickes.png");
    let output_dir = std::env::temp_dir().join("uex-datarunner-crop-step-test");
    let _ = std::fs::remove_dir_all(&output_dir);

    let images = prepare_right_panel_ocr_images_in(
        &source,
        &output_dir,
        "crop-step",
        CUDA_VISION_MODEL_ALIAS,
    )
    .expect("the right-panel crop step should produce an OCR image");

    assert_eq!(
        images.len(),
        1,
        "exactly one generous panel crop is sent per screenshot"
    );
    let panel = &images[0];
    assert!(
        panel.exists(),
        "the cropped panel image should be written to disk"
    );

    let decoded = image::open(panel).expect("the cropped panel should be a readable image");
    assert!(
        decoded.width() >= 200 && decoded.height() >= 200,
        "the crop should retain enough resolution to read terminal prices (got {}x{})",
        decoded.width(),
        decoded.height()
    );

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn right_panel_crop_does_not_upscale_small_screenshots() {
    let source = temp_test_dir("small-crop-source").join("small.png");
    let output_dir = temp_test_dir("small-crop-output");
    let _ = std::fs::remove_dir_all(&output_dir);
    let image = image::RgbImage::from_pixel(640, 360, image::Rgb([20, 20, 20]));
    image
        .save(&source)
        .expect("test source image should be writable");

    let images =
        prepare_right_panel_ocr_images_in(&source, &output_dir, "small", CUDA_VISION_MODEL_ALIAS)
            .expect("crop should succeed");
    let decoded = image::open(&images[0]).expect("crop should be readable");

    assert!(
        decoded.width() <= 640 && decoded.height() <= 360,
        "OCR preparation must not upscale screenshots and increase model input cost (got {}x{})",
        decoded.width(),
        decoded.height()
    );

    let _ = std::fs::remove_dir_all(&output_dir);
    let _ = std::fs::remove_file(source);
}

#[test]
fn foundry_ocr_http_timeout_is_bounded_to_one_request_window() {
    let timeout_secs = std::hint::black_box(FOUNDRY_OCR_HTTP_TIMEOUT_SECS);
    assert!(
        timeout_secs <= 180,
        "a hung Foundry request should be bounded to one request window instead of blocking indefinitely"
    );
}

#[test]
fn foundry_ocr_uses_bounded_image_and_output_budget_for_local_vision_runtime() {
    let image_max_side = std::hint::black_box(crate::foundry_ocr::OCR_IMAGE_MAX_SIDE);
    let max_tokens = std::hint::black_box(FOUNDRY_OCR_MAX_TOKENS);
    assert!(
        image_max_side <= 960.0,
        "Foundry Local vision OCR should keep the prepared panel small enough to avoid runtime memory spikes"
    );
    assert!(
        max_tokens <= 4096,
        "OCR output should be bounded to the compact JSON contract instead of allowing long reasoning output"
    );
}

#[test]
fn qwen3_vl_instruct_alias_matches_foundry_catalog_name() {
    assert_eq!(
        QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS,
        "qwen3-vl-4b-instruct",
        "the UI and backend must use the Foundry Local catalog alias for the Qwen 3 VL 4B instruct model"
    );
}

#[test]
fn foundry_ocr_rejects_reasoning_prose_before_frontend_parsing() {
    let error = validate_foundry_ocr_json(
        "The user wants me to identify commodities. First I will inspect the screenshot.",
    )
    .expect_err("reasoning prose should not be accepted as OCR output");

    assert!(error.to_string().contains("valid JSON"));
}

#[test]
fn foundry_ocr_normalizes_embedded_json_after_thinking_text() {
    let normalized = normalize_foundry_ocr_json(
        "I will inspect the screenshot first.\n{\"marketSide\":\"buy\",\"commodities\":[{\"name\":\"Agricium\",\"status\":\"high\",\"scu\":40,\"pricePerScu\":1515,\"cargoSizes\":[1,2,4]}]}\nDone.",
    )
    .expect("embedded structured JSON should be extracted and normalized");

    assert!(normalized.starts_with("{\"commodities\""));
    assert!(normalized.contains("\"marketSide\":\"buy\""));
    validate_foundry_ocr_json(&normalized).unwrap();
}

#[test]
fn foundry_ocr_strips_qwen_think_block_before_reading_structured_json() {
    let normalized = normalize_foundry_ocr_json(
        "<think>The user wants commodities. This is a sell terminal. Let me read the rows.</think>\n{\"marketSide\":\"sell\",\"commodities\":[{\"name\":\"Quartz\",\"status\":\"out of stock\",\"scu\":0,\"pricePerScu\":4000,\"cargoSizes\":[1,2,4]}]}",
    )
    .expect("a <think> block must be dropped and the JSON read directly");

    validate_foundry_ocr_json(&normalized).unwrap();
    assert!(!normalized.contains("<think>"));
    assert!(normalized.contains("\"marketSide\":\"sell\""));

    let value: serde_json::Value = serde_json::from_str(&normalized).unwrap();
    let commodities = value
        .get("commodities")
        .and_then(serde_json::Value::as_array)
        .unwrap();
    assert_eq!(commodities.len(), 1);
    assert_eq!(
        commodities[0]
            .get("name")
            .and_then(serde_json::Value::as_str),
        Some("Quartz")
    );
}

#[test]
fn strip_model_reasoning_handles_paired_unclosed_and_orphan_think_tags() {
    assert_eq!(
        strip_model_reasoning("<think>reason</think>answer"),
        "answer"
    );
    assert_eq!(
        strip_model_reasoning("prefix<think>reason</think>"),
        "prefix"
    );
    // Unclosed opener (token budget cut the reasoning short): drop the remainder.
    assert_eq!(strip_model_reasoning("keep<think>still thinking"), "keep");
    // Orphan closing tag (opener already truncated away): keep what follows.
    assert_eq!(
        strip_model_reasoning("dangling reasoning</think>final"),
        "final"
    );
    // No reasoning at all: unchanged (trimmed).
    assert_eq!(strip_model_reasoning("  plain  "), "plain");
}

#[test]
fn foundry_ocr_merges_multiple_crop_json_outputs() {
    let merged = merge_foundry_ocr_results(&[
        "{\"marketSide\":\"sell\",\"commodities\":[{\"name\":\"Agricium\",\"status\":\"high\",\"scu\":40,\"pricePerScu\":1515,\"cargoSizes\":[1,2]}]}".to_string(),
        "{\"marketSide\":\"sell\",\"commodities\":[{\"name\":\"Agricium\",\"status\":\"high\",\"scu\":40,\"pricePerScu\":1515,\"cargoSizes\":[1,2]},{\"name\":\"Processed Food\",\"status\":\"full\",\"scu\":0,\"pricePerScu\":1500,\"cargoSizes\":[1,2]}]}".to_string(),
    ])
    .expect("multiple OCR crop outputs should merge into one JSON document");
    let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
    let commodities = value
        .get("commodities")
        .and_then(serde_json::Value::as_array)
        .unwrap();

    assert_eq!(commodities.len(), 2);
    validate_foundry_ocr_json(&merged).unwrap();
}

#[test]
fn developer_logs_use_local_time_instead_of_unix_milliseconds() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("lib.rs should be readable");

    assert!(source.contains("Local::now().format"));
    assert!(!source.contains("unix_timestamp_ms"));
}

#[test]
fn foundry_cuda_model_selects_cuda_variant_instead_of_webgpu_or_cpu() {
    let variants = vec![
        FoundryVariantChoice::for_test(
            "qwen3.5-4b-webgpu-gpu",
            "Foundry",
            Some("WebGPUExecutionProvider"),
            Some("GPU"),
            "text,image",
            false,
        ),
        FoundryVariantChoice::for_test(
            "qwen3.5-4b-cuda-gpu",
            "Foundry CUDA",
            Some("CUDAExecutionProvider"),
            Some("GPU"),
            "text,image",
            false,
        ),
        FoundryVariantChoice::for_test(
            "qwen3.5-4b-cpu",
            "Foundry CPU",
            Some("CPUExecutionProvider"),
            Some("CPU"),
            "text,image",
            true,
        ),
    ];

    let selected = select_foundry_vision_variant(CUDA_VISION_MODEL_ALIAS, &variants)
        .expect("CUDA variant should be selected");

    assert_eq!(selected.id, "qwen3.5-4b-cuda-gpu");
}

#[test]
fn foundry_qwen3_vl_instruct_model_selects_cuda_variant_instead_of_webgpu_or_cpu() {
    let variants = vec![
        FoundryVariantChoice::for_test(
            "qwen3-vl-webgpu-gpu",
            "Foundry",
            Some("WebGPUExecutionProvider"),
            Some("GPU"),
            "text,image",
            true,
        ),
        FoundryVariantChoice::for_test(
            "qwen-3-vl-4b-instruct-cuda-gpu",
            "Foundry CUDA",
            Some("CUDAExecutionProvider"),
            Some("GPU"),
            "text,image",
            false,
        ),
        FoundryVariantChoice::for_test(
            "qwen3-vl-cpu",
            "Foundry CPU",
            Some("CPUExecutionProvider"),
            Some("CPU"),
            "text,image",
            true,
        ),
    ];

    let selected = select_foundry_vision_variant(QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS, &variants)
        .expect("CUDA variant should be selected");

    assert_eq!(selected.id, "qwen-3-vl-4b-instruct-cuda-gpu");
}

#[test]
fn foundry_variant_selection_rejects_catalogs_without_gpu_vision_variant() {
    let variants = vec![
        FoundryVariantChoice::for_test(
            "qwen3-vl-cpu",
            "Foundry CPU",
            Some("CPUExecutionProvider"),
            Some("CPU"),
            "text,image",
            true,
        ),
        FoundryVariantChoice::for_test(
            "qwen3-vl-webgpu",
            "Foundry",
            Some("WebGPUExecutionProvider"),
            Some("GPU"),
            "text,image",
            false,
        ),
    ];

    let error = select_foundry_vision_variant(QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS, &variants)
        .expect_err("CPU and WebGPU variants must not be accepted");

    assert!(error
        .to_string()
        .contains("No supported GPU vision variant"));
    assert!(error.to_string().contains("qwen3-vl-cpu"));
    assert!(error.to_string().contains("qwen3-vl-webgpu"));
}

#[test]
fn terminal_filter_matches_location_fields_without_api_calls() {
    let terminals = vec![
        serde_json::from_value::<UexTerminal>(json!({
            "id": 1,
            "name": "Area18 TDD",
            "displayname": "Area18 TDD",
            "planet_name": "ArcCorp",
            "star_system_name": "Stanton"
        }))
        .unwrap(),
        serde_json::from_value::<UexTerminal>(json!({
            "id": 2,
            "name": "Port Tressler",
            "displayname": "Port Tressler",
            "planet_name": "microTech",
            "star_system_name": "Stanton"
        }))
        .unwrap(),
    ];

    let filtered = filter_terminals(terminals, "microtech", 100);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].displayname.as_deref(), Some("Port Tressler"));
}

#[test]
fn terminal_query_checks_all_location_names() {
    let terminal = serde_json::from_value::<UexTerminal>(json!({
        "id": 3,
        "name": "Mining Area 045",
        "displayname": "Mining Area 045",
        "moon_name": "Wala"
    }))
    .unwrap();

    assert!(terminal_matches_query(&terminal, "wala"));
}

#[test]
fn commodity_filter_matches_cached_names_codes_and_slugs_without_api_calls() {
    let commodities = vec![
        UexCommodity {
            id: 1,
            id_parent: None,
            name: "Scrap".to_string(),
            code: Some("SCRP".to_string()),
            slug: Some("scrap".to_string()),
            kind: None,
            weight_scu: None,
            price_buy: None,
            price_sell: None,
            is_available: None,
            is_available_live: None,
            is_visible: None,
            is_buyable: None,
            is_sellable: None,
            is_temporary: None,
            is_illegal: None,
            date_added: None,
            date_modified: None,
        },
        UexCommodity {
            id: 2,
            id_parent: None,
            name: "Titanium".to_string(),
            code: Some("TITA".to_string()),
            slug: Some("titanium".to_string()),
            kind: None,
            weight_scu: None,
            price_buy: None,
            price_sell: None,
            is_available: None,
            is_available_live: None,
            is_visible: None,
            is_buyable: None,
            is_sellable: None,
            is_temporary: None,
            is_illegal: None,
            date_added: None,
            date_modified: None,
        },
    ];

    let filtered = filter_commodities(commodities, "tita", 80);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, 2);
}

#[test]
fn game_version_extraction_uses_live_even_when_ptu_is_present() {
    let response = json!({
        "status": "ok",
        "data": {
            "live": "4.7.2",
            "ptu": "4.8.0"
        }
    });

    assert_eq!(extract_live_game_version(&response).unwrap(), "4.7.2");
}

#[test]
fn game_version_extraction_rejects_ptu_only_response() {
    let response = json!({
        "status": "ok",
        "data": {
            "live": null,
            "ptu": "4.8.0"
        }
    });

    assert!(extract_live_game_version(&response).is_err());
}

#[test]
fn uex_api_array_response_parses_documented_data_array_shape() {
    let response = json!({
        "status": "ok",
        "data": [{
            "id": 89,
            "name": "Area18 TDD",
            "displayname": "Area18 TDD",
            "code": "AREA18-TDD"
        }]
    });

    let terminals = parse_uex_data_array::<UexTerminal>(response, "terminals").unwrap();

    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0].id, 89);
    assert_eq!(terminals[0].displayname.as_deref(), Some("Area18 TDD"));
}

#[test]
fn uex_api_array_response_rejects_malformed_data_instead_of_silently_emptying_cache() {
    let response = json!({
        "status": "ok",
        "data": {
            "id": 89,
            "name": "Area18 TDD"
        }
    });

    let error = parse_uex_data_array::<UexTerminal>(response, "terminals")
        .expect_err("object data must not be treated as an empty UEX terminal cache");

    assert!(error
        .to_string()
        .contains("UEX terminals response data was not an array"));
}

#[test]
fn uex_api_object_response_parses_data_parameters_shape() {
    let response = json!({
        "status": "ok",
        "data": {
            "is_accepting_reports": 1,
            "is_datacenter_enabled": 1,
            "game_version": "4.2.1"
        }
    });

    let parameters =
        parse_uex_data_object::<crate::UexDataParameters>(response, "data_parameters").unwrap();

    assert_eq!(parameters.is_accepting_reports, Some(1));
    assert_eq!(parameters.game_version.as_deref(), Some("4.2.1"));
}

#[test]
fn transport_failure_detection_covers_timeout_connection_and_health_errors() {
    let timeout_err = AppError::Message("Foundry OCR request timed out after 180s".to_string());
    assert!(
        is_foundry_transport_failure(&timeout_err),
        "timeout errors must be classified as transport failures"
    );

    let connection_err =
        AppError::Message("Foundry OCR connection failed (service may not be running)".to_string());
    assert!(
        is_foundry_transport_failure(&connection_err),
        "connection errors must be classified as transport failures"
    );

    let health_err =
        AppError::Message("Foundry web service is not responding: timeout".to_string());
    assert!(
        is_foundry_transport_failure(&health_err),
        "health check failures must be classified as transport failures"
    );

    let json_err = AppError::Message("Foundry OCR did not return valid JSON".to_string());
    assert!(
        !is_foundry_transport_failure(&json_err),
        "JSON parsing errors must NOT be classified as transport failures"
    );

    let generic_err = AppError::Message("some other error".to_string());
    assert!(
        !is_foundry_transport_failure(&generic_err),
        "unrelated errors must NOT be classified as transport failures"
    );
}

#[test]
fn foundry_timeout_error_message_includes_cause_detail() {
    let client_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("client.rs"),
    )
    .expect("client.rs should be readable");

    // The timeout error should include the underlying cause, not just the
    // timeout duration, so we can distinguish between actual timeouts and
    // other reqwest errors.
    assert!(
        client_source.contains("Cause:") || client_source.contains("cause:"),
        "timeout error messages should include the underlying cause for diagnostics"
    );
}

#[test]
fn foundry_ocr_client_has_health_check_before_inference() {
    let mod_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("mod.rs"),
    )
    .expect("mod.rs should be readable");

    assert!(
        mod_source.contains("verify_foundry_service_health"),
        "OCR pipeline must verify web service health before sending inference requests"
    );
    assert!(
        !mod_source.contains("restart_foundry_session"),
        "OCR pipeline must NOT have a recovery mechanism when Foundry hangs per user configuration"
    );
}

#[test]
fn foundry_model_loading_verifies_post_load_state() {
    let model_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("model.rs"),
    )
    .expect("model.rs should be readable");

    assert!(
        model_source.contains("Post-load verification"),
        "model loading must verify the model is truly loaded after load() returns"
    );
    assert!(
        model_source.contains("Pre-load state"),
        "model loading must log pre-load state for diagnostics"
    );
    assert!(
        model_source.contains("is_loaded() returned false"),
        "model loading must fail explicitly if post-load check shows the model is not loaded"
    );
}

#[test]
fn foundry_ocr_client_does_not_retry_on_transport_failure() {
    let client_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("client.rs"),
    )
    .expect("client.rs should be readable");

    assert!(
        !client_source.contains("FOUNDRY_OCR_MAX_RETRIES"),
        "OCR client must NOT define a max retry count for transient failures per user configuration"
    );
    assert!(
        !client_source.contains("Retrying OCR request"),
        "OCR client must NOT log retry attempts"
    );
    assert!(
        client_source.contains("send_foundry_ocr_request"),
        "OCR request logic must be factored into a reusable function for retry"
    );
}

#[test]
fn foundry_ocr_logs_request_diagnostics() {
    let client_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("foundry_ocr")
            .join("client.rs"),
    )
    .expect("client.rs should be readable");

    // Verify that the OCR client logs the endpoint, body size, and image info
    assert!(
        client_source.contains("endpoint="),
        "OCR client must log the Foundry endpoint URL"
    );
    assert!(
        client_source.contains("body_size="),
        "OCR client must log the request body size"
    );
    assert!(
        client_source.contains("image_base64_len="),
        "OCR client must log the image payload size"
    );
    assert!(
        client_source.contains("HTTP response: status="),
        "OCR client must log the HTTP response status immediately"
    );
}
