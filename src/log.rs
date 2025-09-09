use std::collections::BTreeMap;

use serde::Serialize;

use crate::client::client;
use crate::ndjson;
use crate::package::NAME;
use crate::timestamp::Timestamp;

pub struct LogConfig {
    pub api_key: String,
    pub endpoint: String,
    pub hostname: String,
    pub group: String,
    pub origin: LogOrigin,
    pub digest: String,
    pub revision: Option<String>,
    pub command: String,
}

impl LogConfig {
    pub fn request(&self, messages: Vec<LogMessage>) -> Result<reqwest::Request, reqwest::Error> {
        let url = format!("{}/logs/json", self.endpoint);

        client()
            .post(url)
            .query(&[("api_key", &self.api_key)])
            .header("Content-Type", "application/x-ndjson")
            .body(ndjson::to_string(messages).expect("failed to serialize log messages"))
            .build()
    }

    fn tags(&self) -> BTreeMap<String, String> {
        [
            (format!("{}-digest", NAME), Some(self.digest.clone())),
            ("revision".to_string(), self.revision.clone()),
            ("command".to_string(), Some(self.command.clone())),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOrigin {
    None,
    Stdout,
    Stderr,
    All,
}

impl LogOrigin {
    pub fn from_args(no_log: bool, no_stdout: bool, no_stderr: bool) -> Self {
        if no_log {
            return Self::None;
        }

        match (no_stdout, no_stderr) {
            (true, true) => Self::None,
            (true, false) => Self::Stderr,
            (false, true) => Self::Stdout,
            (false, false) => Self::All,
        }
    }

    pub fn is_out(&self) -> bool {
        matches!(self, Self::Stdout | Self::All)
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Self::Stderr | Self::All)
    }
}

#[derive(Serialize)]
pub struct LogMessage {
    group: String,
    timestamp: String,
    severity: LogSeverity,
    message: String,
    hostname: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    attributes: BTreeMap<String, String>,
}

impl LogMessage {
    pub fn new(
        config: &LogConfig,
        timestamp: &mut impl Timestamp,
        severity: LogSeverity,
        message: String,
    ) -> Self {
        Self {
            group: config.group.clone(),
            timestamp: timestamp.as_rfc3339(),
            severity,
            message,
            hostname: config.hostname.clone(),
            attributes: config.tags(),
        }
    }
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogSeverity {
    Info,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timestamp::tests::{timestamp, EXPECTED_RFC3339};

    fn log_config() -> LogConfig {
        LogConfig {
            api_key: "some_api_key".to_string(),
            endpoint: "https://some-endpoint.com".to_string(),
            hostname: "some-hostname".to_string(),
            group: "some-group".to_string(),
            origin: LogOrigin::All,
            digest: "some-digest".to_string(),
            revision: Some("some-revision".to_string()),
            command: "some-command".to_string(),
        }
    }

    #[test]
    fn log_config_request() {
        let config = log_config();
        let first_message = LogMessage::new(
            &config,
            &mut timestamp(),
            LogSeverity::Info,
            "first-message".to_string(),
        );
        let second_message = LogMessage::new(
            &config,
            &mut timestamp(),
            LogSeverity::Error,
            "second-message".to_string(),
        );

        let request = config.request(vec![first_message, second_message]).unwrap();

        assert_eq!(request.method().as_str(), "POST");
        assert_eq!(
            request.url().as_str(),
            "https://some-endpoint.com/logs/json?api_key=some_api_key"
        );
        assert_eq!(
            request.headers().get("Content-Type").unwrap(),
            "application/x-ndjson"
        );
        assert_eq!(
            String::from_utf8_lossy(request.body().unwrap().as_bytes().unwrap()),
            format!(
                concat!(
                    "{{",
                    r#""group":"some-group","#,
                    r#""timestamp":"{}","#,
                    r#""severity":"info","#,
                    r#""message":"first-message","#,
                    r#""hostname":"some-hostname","#,
                    r#""attributes":{{"#,
                    r#""{}-digest":"some-digest","#,
                    r#""command":"some-command","#,
                    r#""revision":"some-revision""#,
                    r#"}}"#,
                    "}}\n",
                    "{{",
                    r#""group":"some-group","#,
                    r#""timestamp":"{}","#,
                    r#""severity":"error","#,
                    r#""message":"second-message","#,
                    r#""hostname":"some-hostname","#,
                    r#""attributes":{{"#,
                    r#""{}-digest":"some-digest","#,
                    r#""command":"some-command","#,
                    r#""revision":"some-revision""#,
                    r#"}}"#,
                    "}}\n"
                ),
                EXPECTED_RFC3339, NAME, EXPECTED_RFC3339, NAME
            )
        );
    }
}
