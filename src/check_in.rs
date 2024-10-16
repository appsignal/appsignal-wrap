use reqwest::Request;
use serde::Serialize;
use std::ops::Deref;
use crate::timestamp;
use crate::client::client;

pub struct CheckInConfig {
  pub api_key: String,
  pub endpoint: String,
  pub identifier: String,
}

#[derive(Serialize)]
struct CheckInQuery {
  api_key: String,
  identifier: String,
  timestamp: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  kind: Option<CronKind>,
}

impl CheckInQuery {
  pub fn from_cron(config: &CronConfig, kind: CronKind) -> Self {
      Self::from_config(config, Some(kind))
  }

  pub fn from_heartbeat(config: &HeartbeatConfig) -> Self {
      Self::from_config(config, None)
  }

  fn from_config(config: &CheckInConfig, kind: Option<CronKind>) -> Self {
      Self {
          api_key: config.api_key.clone(),
          identifier: config.identifier.clone(),
          timestamp: timestamp::as_secs(),
          kind,
      }
  }
}

#[derive(Copy, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CronKind {
    Start,
    Finish,
}

pub struct CronConfig(pub CheckInConfig);

impl Deref for CronConfig {
  type Target = CheckInConfig;

  fn deref(&self) -> &Self::Target {
      &self.0
  }
}

impl CronConfig {
  pub fn request(&self, kind: CronKind) -> Result<Request, reqwest::Error> {
    let url = format!("{}/check_ins/cron", self.endpoint);

    client().post(url)
      .query(&CheckInQuery::from_cron(self, kind))
      .build()
  }
}

pub struct HeartbeatConfig(pub CheckInConfig);

impl Deref for HeartbeatConfig {
  type Target = CheckInConfig;

  fn deref(&self) -> &Self::Target {
      &self.0
  }
}

impl HeartbeatConfig {
  pub fn request(&self) -> Result<Request, reqwest::Error> {
    let url = format!("{}/check_ins/heartbeats", self.endpoint);

    client().post(url)
      .query(&CheckInQuery::from_heartbeat(self))
      .build()
  }
}
