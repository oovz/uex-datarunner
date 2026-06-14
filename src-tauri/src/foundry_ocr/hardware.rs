use std::path::{Path, PathBuf};

use crate::{log_event, AppError, AppResult};

pub(crate) fn is_cuda_runtime_available() -> bool {
    find_cuda_runtime_dir().is_some()
}

pub(crate) fn find_cuda_runtime_dir() -> Option<PathBuf> {
    let roots = cuda_runtime_search_roots();
    let found = find_cuda_runtime_dir_in_roots(roots.iter().map(PathBuf::as_path));
    if let Some(path) = &found {
        log_event("OCR", format!("CUDA runtime found: {}", path.display()));
    }
    found
}

fn cuda_runtime_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(paths) = std::env::var_os("PATH") {
        roots.extend(std::env::split_paths(&paths));
    }

    for (key, value) in std::env::vars_os() {
        let Some(key) = key.to_str() else {
            continue;
        };
        if key.eq_ignore_ascii_case("CUDA_PATH")
            || (key.to_ascii_uppercase().starts_with("CUDA_PATH_V") && !value.is_empty())
        {
            let root = PathBuf::from(value);
            roots.push(root.join("bin"));
            roots.push(root);
        }
    }

    if let Ok(program_files) = std::env::var("ProgramFiles") {
        let toolkit_root = Path::new(&program_files)
            .join("NVIDIA GPU Computing Toolkit")
            .join("CUDA");
        if let Ok(entries) = std::fs::read_dir(toolkit_root) {
            roots.extend(entries.flatten().map(|entry| entry.path().join("bin")));
        }

        roots.push(
            Path::new(&program_files)
                .join("NVIDIA Corporation")
                .join("NVIDIA Video Effects"),
        );
    }

    roots
}

pub(crate) fn find_cuda_runtime_dir_in_roots<'a>(
    roots: impl IntoIterator<Item = &'a Path>,
) -> Option<PathBuf> {
    roots
        .into_iter()
        .find(|root| root_contains_cuda_runtime(root))
        .map(Path::to_path_buf)
}

fn root_contains_cuda_runtime(root: &Path) -> bool {
    std::fs::read_dir(root)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .any(|name| {
            let lower = name.to_lowercase();
            lower.starts_with("cudart64_") && lower.ends_with(".dll")
        })
}

pub(crate) fn get_system_gpu_info() -> (Option<String>, String) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
        unsafe {
            if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
                let mut i = 0;
                let mut best_gpu: Option<(String, String, i32)> = None;

                while let Ok(adapter) = factory.EnumAdapters1(i) {
                    if let Ok(desc) = adapter.GetDesc1() {
                        let name = String::from_utf16_lossy(&desc.Description);
                        let trimmed = name.trim_matches(char::from(0)).trim().to_string();
                        let upper = trimmed.to_uppercase();
                        if !upper.contains("BASIC RENDER DRIVER") {
                            let (vendor, score) = if upper.contains("NVIDIA") {
                                ("NVIDIA CUDA".to_string(), 4)
                            } else if upper.contains("AMD") {
                                ("AMD (CUDA unsupported)".to_string(), 3)
                            } else if upper.contains("INTEL") {
                                ("Intel (CUDA unsupported)".to_string(), 2)
                            } else {
                                ("GPU (CUDA unsupported)".to_string(), 1)
                            };

                            let should_replace = match &best_gpu {
                                Some((_, _, best_score)) => score > *best_score,
                                None => true,
                            };
                            if should_replace {
                                best_gpu = Some((trimmed, vendor, score));
                            }
                        }
                    }
                    i += 1;
                }

                if let Some((name, vendor, _)) = best_gpu {
                    return (Some(name), vendor);
                }
            }
        }
    }
    (None, "Unknown/Generic".to_string())
}

pub(crate) fn default_ai_model() -> String {
    super::CUDA_VISION_MODEL_ALIAS.to_string()
}

pub(crate) fn configured_ai_model(config: &crate::AppConfig) -> String {
    config
        .ai_model
        .split_whitespace()
        .next()
        .filter(|alias| !alias.is_empty())
        .map(|alias| alias.to_string())
        .unwrap_or_else(default_ai_model)
}

pub(crate) fn validate_ocr_hardware_policy(model_alias: &str) -> AppResult<()> {
    let (gpu_name, gpu_vendor) = get_system_gpu_info();
    if gpu_name.is_none() {
        return Err(AppError::Message(
            "No compatible GPU (NVIDIA, AMD, or Intel) detected. CPU-only execution is not supported in this version.".to_string(),
        ));
    }

    let has_nvidia = gpu_vendor.to_uppercase().contains("NVIDIA");
    let has_cuda = is_cuda_runtime_available();
    if !has_nvidia || !has_cuda {
        return Err(AppError::Message(format!(
            "Foundry OCR model '{model_alias}' requires an NVIDIA GPU and an active CUDA runtime."
        )));
    }

    Ok(())
}

pub(crate) fn is_cuda_model_alias(model_alias: &str) -> bool {
    model_alias == super::CUDA_VISION_MODEL_ALIAS
}

pub(crate) fn is_cuda_runtime_name(value: &str) -> bool {
    let value = value.to_lowercase();
    value.contains("cuda") || value.contains("nvidia")
}
