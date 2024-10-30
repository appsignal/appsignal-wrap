use crate::client::client;
use crate::timestamp;
use reqwest::Request;
use serde::Serialize;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
}

impl CheckInQuery {
    pub fn from_cron(config: &CronConfig, kind: CronKind) -> Self {
        Self::from_config(&config.check_in, Some(kind), Some(config.digest.clone()))
    }

    pub fn from_heartbeat(config: &HeartbeatConfig) -> Self {
        Self::from_config(&config.check_in, None, None)
    }

    fn from_config(config: &CheckInConfig, kind: Option<CronKind>, digest: Option<String>) -> Self {
        Self {
            api_key: config.api_key.clone(),
            identifier: config.identifier.clone(),
            timestamp: timestamp::as_secs(),
            kind,
            digest,
        }
    }
}

#[derive(Copy, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CronKind {
    Start,
    Finish,
}

pub struct CronConfig {
    pub check_in: CheckInConfig,
    pub digest: String,
}

impl CronConfig {
    pub fn request(&self, kind: CronKind) -> Result<Request, reqwest::Error> {
        let url = format!("{}/check_ins/cron", self.check_in.endpoint);

        client()
            .post(url)
            .query(&CheckInQuery::from_cron(self, kind))
            .build()
    }
}

pub struct HeartbeatConfig {
    pub check_in: CheckInConfig,
}

impl HeartbeatConfig {
    pub fn request(&self) -> Result<Request, reqwest::Error> {
        let url = format!("{}/check_ins/heartbeats", self.check_in.endpoint);

        client()
            .post(url)
            .query(&CheckInQuery::from_heartbeat(self))
            .build()
    }
}
