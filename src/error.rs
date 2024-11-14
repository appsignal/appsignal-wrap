use std::collections::BTreeMap;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use reqwest::Body;
use serde::Serialize;

use crate::client::client;
use crate::package::NAME;
use crate::signal::signal_name;
use crate::timestamp::Timestamp;

pub struct ErrorConfig {
    pub api_key: String,
    pub endpoint: String,
    pub action: String,
    pub hostname: String,
    pub digest: String,
}

impl ErrorConfig {
    pub fn request(
        &self,
        timestamp: &mut impl Timestamp,
        exit: &ExitStatus,
        lines: impl IntoIterator<Item = String>,
    ) -> Result<reqwest::Request, reqwest::Error> {
        let url = format!("{}/errors", self.endpoint);

        client()
            .post(url)
            .query(&[("api_key", &self.api_key)])
            .header("Content-Type", "application/json")
            .body(ErrorBody::from_config(&self, timestamp, exit, lines))
            .build()
    }
}

#[derive(Serialize)]
pub struct ErrorBody {
    pub timestamp: u64,
    pub action: String,
    pub namespace: String,
    pub error: ErrorBodyError,
    pub tags: BTreeMap<String, String>,
}

impl ErrorBody {
    pub fn from_config(
        config: &ErrorConfig,
        timestamp: &mut impl Timestamp,
        exit: &ExitStatus,
        lines: impl IntoIterator<Item = String>,
    ) -> Self {
        ErrorBody {
            timestamp: timestamp.as_secs(),
            action: config.action.clone(),
            namespace: "process".to_string(),
            error: ErrorBodyError::new(exit, lines),
            tags: exit_tags(exit)
                .into_iter()
                .chain([
                    ("hostname".to_string(), config.hostname.clone()),
                    (format!("{}-digest", NAME), config.digest.clone()),
                ])
                .collect(),
        }
    }
}

impl From<ErrorBody> for Body {
    fn from(body: ErrorBody) -> Self {
        Body::from(serde_json::to_string(&body).unwrap())
    }
}

#[derive(Serialize)]
pub struct ErrorBodyError {
    pub name: String,
    pub message: String,
}

impl ErrorBodyError {
    pub fn new(exit: &ExitStatus, lines: impl IntoIterator<Item = String>) -> Self {
        let (name, exit_context) = if let Some(code) = exit.code() {
            ("NonZeroExit".to_string(), format!("code {}", code))
        } else if let Some(signal) = exit.signal() {
            (
                "SignalExit".to_string(),
                format!("signal {}", signal_name(signal)),
            )
        } else {
            ("UnknownExit".to_string(), "unknown status".to_string())
        };

        let mut lines = lines.into_iter().collect::<Vec<String>>();
        lines.push(format!("[Process exited with {}]", exit_context));

        let message = lines.join("\n");

        ErrorBodyError { name, message }
    }
}

fn exit_tags(exit: &ExitStatus) -> BTreeMap<String, String> {
    if let Some(code) = exit.code() {
        [
            ("exit_code".to_string(), format!("{}", code)),
            ("exit_kind".to_string(), "code".to_string()),
        ]
        .into()
    } else if let Some(signal) = exit.signal() {
        [
            ("exit_signal".to_string(), signal_name(signal)),
            ("exit_kind".to_string(), "signal".to_string()),
        ]
        .into()
    } else {
        [("exit_kind".to_string(), "unknown".to_string())].into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timestamp::tests::{timestamp, EXPECTED_SECS};

    fn error_config() -> ErrorConfig {
        ErrorConfig {
            api_key: "some_api_key".to_string(),
            endpoint: "https://some-endpoint.com".to_string(),
            hostname: "some-hostname".to_string(),
            digest: "some-digest".to_string(),
            action: "some-action".to_string(),
        }
    }

    #[test]
    fn error_config_request() {
        let config = error_config();
        // `ExitStatus::from_raw` expects a wait status, not an exit status.
        // The wait status for exit code `n` is represented by `n << 8`.
        // See `__WEXITSTATUS` in `glibc/bits/waitstatus.h` for reference.
        let exit = ExitStatus::from_raw(42 << 8);
        let lines = vec!["line 1".to_string(), "line 2".to_string()];

        let request = config.request(&mut timestamp(), &exit, lines).unwrap();

        assert_eq!(request.method().as_str(), "POST");
        assert_eq!(
            request.url().as_str(),
            "https://some-endpoint.com/errors?api_key=some_api_key"
        );
        assert_eq!(
            request.headers().get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(
            String::from_utf8_lossy(request.body().unwrap().as_bytes().unwrap()),
            format!(
                concat!(
                    "{{",
                    r#""timestamp":{},"#,
                    r#""action":"some-action","#,
                    r#""namespace":"process","#,
                    r#""error":{{"#,
                    r#""name":"NonZeroExit","#,
                    r#""message":"line 1\nline 2\n[Process exited with code 42]""#,
                    r#"}},"#,
                    r#""tags":{{"#,
                    r#""{}-digest":"some-digest","#,
                    r#""exit_code":"42","#,
                    r#""exit_kind":"code","#,
                    r#""hostname":"some-hostname""#,
                    r#"}}"#,
                    "}}"
                ),
                EXPECTED_SECS, NAME
            )
        );
    }
}
