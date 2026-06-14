use std::collections::HashSet;

use crate::error::{AppError, AppResult};
use serde_json::{json, Value};

pub(crate) fn foundry_ocr_response_text(response: &Value) -> AppResult<String> {
    if let Some(text) = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
    {
        return Ok(text.to_string());
    }

    if let Some(text) = response
        .get("output")
        .and_then(Value::as_array)
        .and_then(|output| {
            output.iter().find_map(|item| {
                item.get("content")
                    .and_then(Value::as_array)
                    .and_then(|content| {
                        content.iter().find_map(|part| {
                            if part.get("type")? == "output_text" {
                                part.get("text")?.as_str()
                            } else {
                                None
                            }
                        })
                    })
            })
        })
    {
        return Ok(text.to_string());
    }

    Err(AppError::Message(format!(
        "Foundry OCR returned an unrecognized response shape: {}",
        serde_json::to_string(response).unwrap_or_default()
    )))
}

pub(crate) fn foundry_ocr_messages(model_id: &str, image_base64: &str) -> AppResult<Value> {
    Ok(json!({
        "model": model_id,
        "instructions": super::FOUNDRY_OCR_INSTRUCTIONS,
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {"type": "input_text", "text": super::FOUNDRY_OCR_USER_PROMPT_TEMPLATE},
                    {
                        "type": "input_image",
                        "media_type": "image/jpeg",
                        "image_data": image_base64
                    }
                ]
            }
        ],
        "temperature": 0.0,
        "max_output_tokens": super::FOUNDRY_OCR_MAX_TOKENS
    }))
}

pub(crate) fn normalize_foundry_ocr_json(text: &str) -> AppResult<String> {
    let cleaned = strip_model_reasoning(text);

    let mut value = if let Ok(value) = parse_valid_foundry_ocr_json(&cleaned) {
        value
    } else {
        let mut candidates = json_object_candidates(&cleaned);
        candidates.sort_by_key(|candidate| {
            if candidate.starts_with("{\"marketSide\"") {
                0
            } else if candidate.starts_with("{\"commodities\"") {
                1
            } else {
                2
            }
        });
        let mut found = None;
        for candidate in &candidates {
            if let Ok(value) = parse_valid_foundry_ocr_json(candidate) {
                found = Some(value);
                break;
            }
        }
        match found {
            Some(val) => val,
            None => parse_valid_foundry_ocr_json(&cleaned)?,
        }
    };

    if let Some(obj) = value.as_object_mut() {
        let active_tab = obj.get("activeTab").and_then(Value::as_str);
        let list_header = obj.get("listHeader").and_then(Value::as_str);
        let raw_market_side = obj.get("marketSide").and_then(Value::as_str);

        let resolved_market_side =
            determine_market_side_from_heuristics(raw_market_side, active_tab, list_header);

        obj.insert("marketSide".to_string(), json!(resolved_market_side));
        obj.remove("activeTab");
        obj.remove("listHeader");
    }

    Ok(value.to_string())
}

fn determine_market_side_from_heuristics(
    raw_market_side: Option<&str>,
    active_tab: Option<&str>,
    list_header: Option<&str>,
) -> String {
    let active_tab_lower = active_tab.unwrap_or("").to_lowercase();
    let list_header_lower = list_header.unwrap_or("").to_lowercase();

    // 1. Check active tab highlights first, based on "Buy" vs "Local Market Value"
    // - "Buy" / "购买" -> UEX "buy"
    // - "Local Market Value" / "本地市场价格" / "Sell" / "出售" -> UEX "sell"
    if active_tab_lower.contains("local market")
        || active_tab_lower.contains("本地市场")
        || active_tab_lower.contains("本地价格")
        || active_tab_lower.contains("市场价格")
        || active_tab_lower.contains("value")
        || active_tab_lower.contains("sell")
        || active_tab_lower.contains("出售")
    {
        return "sell".to_string();
    }
    if active_tab_lower.contains("buy") || active_tab_lower.contains("购买") {
        return "buy".to_string();
    }

    // 2. Fallbacks based on specific list headers
    // - "demand" / "sellable" / "需求" / "出售" -> UEX "sell"
    // - "stock" / "have stock" / "有货" / "购买" -> UEX "buy"
    if (list_header_lower.contains("demand")
        && !list_header_lower.contains("had demand")
        && !list_header_lower.contains("no demand"))
        || list_header_lower.contains("sellable")
        || list_header_lower.contains("需求")
        || list_header_lower.contains("出售")
    {
        return "sell".to_string();
    }
    if list_header_lower.contains("stock")
        || list_header_lower.contains("have stock")
        || list_header_lower.contains("有货")
        || list_header_lower.contains("购买")
    {
        return "buy".to_string();
    }

    raw_market_side.unwrap_or("sell").to_string()
}

#[allow(dead_code)]
pub(crate) fn validate_foundry_ocr_json(text: &str) -> AppResult<()> {
    parse_valid_foundry_ocr_json(text).map(|_| ())
}

pub(crate) fn parse_valid_foundry_ocr_json(text: &str) -> AppResult<Value> {
    let value: Value = serde_json::from_str(text.trim()).map_err(|error| {
        AppError::Message(format!(
            "Foundry OCR did not return valid JSON: {error}. Raw output: {}",
            text.chars().take(1200).collect::<String>()
        ))
    })?;
    let market_side = value.get("marketSide").and_then(Value::as_str);
    if !matches!(market_side, Some("buy" | "sell")) {
        return Err(AppError::Message(format!(
            "Foundry OCR JSON omitted marketSide buy/sell. Raw output: {}",
            text.chars().take(1200).collect::<String>()
        )));
    }
    if !value
        .get("commodities")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        return Err(AppError::Message(format!(
            "Foundry OCR JSON omitted commodities array. Raw output: {}",
            text.chars().take(1200).collect::<String>()
        )));
    }

    Ok(value)
}

pub(crate) fn strip_model_reasoning(text: &str) -> String {
    const OPEN: &str = "<think>";
    const CLOSE: &str = "</think>";

    let mut output = String::with_capacity(text.len());
    let mut rest = text;

    loop {
        let lower = rest.to_lowercase();
        match lower.find(OPEN) {
            Some(open) => {
                output.push_str(&rest[..open]);
                let after_open = &rest[open + OPEN.len()..];
                match after_open.to_lowercase().find(CLOSE) {
                    Some(close) => rest = &after_open[close + CLOSE.len()..],
                    None => {
                        break;
                    }
                }
            }
            None => {
                output.push_str(rest);
                break;
            }
        }
    }

    let lower = output.to_lowercase();
    if let Some(pos) = lower.rfind(CLOSE) {
        output = output[pos + CLOSE.len()..].to_string();
    }

    output.trim().to_string()
}

pub(crate) fn merge_foundry_ocr_results(results: &[String]) -> AppResult<String> {
    if results.is_empty() {
        return Ok("{\"marketSide\":\"sell\",\"commodities\":[]}".to_string());
    }

    let mut market_side: Option<String> = None;
    let mut commodities = Vec::<Value>::new();
    let mut seen_names = HashSet::<String>::new();

    for result in results {
        let value = parse_valid_foundry_ocr_json(result)?;
        if market_side.is_none() {
            market_side = value
                .get("marketSide")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        if let Some(rows) = value.get("commodities").and_then(Value::as_array) {
            for row in rows {
                let Some(name) = row.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let key = name.trim().to_lowercase();
                if seen_names.insert(key) {
                    commodities.push(row.clone());
                }
            }
        }
    }

    Ok(json!({
        "marketSide": market_side.unwrap_or_else(|| "sell".to_string()),
        "commodities": commodities,
    })
    .to_string())
}

pub(crate) fn json_object_candidates(text: &str) -> Vec<&str> {
    text.char_indices()
        .filter(|(_, character)| *character == '{')
        .filter_map(|(start, _)| {
            json_object_end(&text[start..]).map(|relative_end| &text[start..start + relative_end])
        })
        .collect()
}

pub(crate) fn json_object_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index + character.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}

#[allow(dead_code)]
pub(crate) fn commodity_extraction_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["marketSide", "commodities"],
        "properties": {
            "marketSide": { "type": "string", "enum": ["buy", "sell"] },
            "commodities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "status", "scu", "pricePerScu", "cargoSizes"],
                    "properties": {
                        "name": { "type": "string" },
                        "status": { "type": ["integer", "null"], "minimum": 1, "maximum": 7 },
                        "scu": { "type": ["number", "null"] },
                        "pricePerScu": { "type": ["number", "null"] },
                        "cargoSizes": {
                            "type": "array",
                            "items": { "type": "number" }
                        }
                    }
                }
            }
        }
    })
}
