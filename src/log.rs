use serde::Serialize;

use crate::timestamp;
use crate::client::client;
use crate::ndjson;

pub struct LogConfig {
  pub api_key: String,
  pub endpoint: String,
  pub hostname: String,
  pub group: String,
  pub origin: LogOrigin,
}

impl LogConfig {
    pub fn request(&self, messages: Vec<LogMessage>) -> Result<reqwest::Request, reqwest::Error> {
        let url = format!("{}/logs/json", self.endpoint);

        client().post(url)
            .query(&[("api_key", &self.api_key)])
            .header("Content-Type", "application/x-ndjson")
            .body(ndjson::to_string(messages).unwrap())
            .build()
    }
}

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
      match self {
          Self::Stdout | Self::All => true,
          _ => false,
      }
  }

  pub fn is_err(&self) -> bool {
      match self {
          Self::Stderr | Self::All => true,
          _ => false,
      }
  }
}

#[derive(Serialize)]
pub struct LogMessage {
    group: String,
    timestamp: String,
    severity: LogSeverity,
    message: String,
    hostname: String,
}

impl LogMessage {
    pub fn new(config: &LogConfig, severity: LogSeverity, message: String) -> Self {
        Self {
            group: config.group.clone(),
            timestamp: timestamp::as_rfc3339(),
            severity,
            message,
            hostname: config.hostname.clone(),
        }
    }
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogSeverity {
    Info,
    Error,
}
