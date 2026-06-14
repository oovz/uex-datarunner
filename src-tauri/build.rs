fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "cancel_ocr",
            "check_uex_account",
            "clear_working_set",
            "delete_submitted_screenshots",
            "get_ocr_status",
            "hide_to_tray",
            "list_screenshots",
            "load_config",
            "load_working_set",
            "open_screenshot",
            "prefetch_commodities",
            "prefetch_data_parameters",
            "prefetch_terminals",
            "process_screenshots",
            "process_selected_screenshots",
            "save_config",
            "save_working_set",
            "search_commodities",
            "submit_to_uex",
        ]),
    ))
    .expect("failed to build Tauri app");
}
