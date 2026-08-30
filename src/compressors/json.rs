use serde_json::Value;

pub fn compress_json(input: &str) -> String {
    let Ok(val) = serde_json::from_str::<Value>(input) else {
        return input.to_string();
    };

    let compacted = compact_value(&val);
    serde_json::to_string_pretty(&compacted).unwrap_or_else(|_| input.to_string())
}

fn compact_value(val: &Value) -> Value {
    match val {
        Value::Array(arr) => {
            if arr.len() <= 5 {
                Value::Array(arr.iter().map(compact_value).collect())
            } else {
                let mut new_arr = Vec::new();
                new_arr.push(compact_value(&arr[0]));
                new_arr.push(compact_value(&arr[1]));
                new_arr.push(Value::String(format!(
                    "[... {} similar items omitted ...]",
                    arr.len() - 3
                )));
                new_arr.push(compact_value(arr.last().unwrap()));
                Value::Array(new_arr)
            }
        }
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), compact_value(v));
            }
            Value::Object(new_map)
        }
        _ => val.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_json_array() {
        let raw = r#"[{"id": 1}, {"id": 2}, {"id": 3}, {"id": 4}, {"id": 5}, {"id": 6}, {"id": 7}, {"id": 8}]"#;
        let compressed = compress_json(raw);
        assert!(compressed.contains("5 similar items omitted"));
    }
}
