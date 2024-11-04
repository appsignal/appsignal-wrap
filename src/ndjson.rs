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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Test {
        a: i32,
        b: String,
    }

    #[test]
    fn test_to_string() {
        let values = vec![
            Test {
                a: 1,
                b: "foo".to_string(),
            },
            Test {
                a: 2,
                b: "bar".to_string(),
            },
        ];

        let result = to_string(values).unwrap();
        assert_eq!(result, "{\"a\":1,\"b\":\"foo\"}\n{\"a\":2,\"b\":\"bar\"}\n");
    }
}
