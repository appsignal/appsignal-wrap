use serde::Serialize;
use serde_json::Result;

pub fn to_string(values: Vec<impl Serialize>) -> Result<String> {
    let mut result = String::new();

    for value in values {
        result.push_str(&serde_json::to_string(&value)?);
        result.push('\n');
    }

    Ok(result)
}
