use crate::client::client;
use crate::timestamp::Timestamp;
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
    pub fn from_cron(config: &CronConfig, timestamp: &mut impl Timestamp, kind: CronKind) -> Self {
        Self::from_config(
            &config.check_in,
            timestamp,
            Some(kind),
            Some(config.digest.clone()),
        )
    }

    pub fn from_heartbeat(config: &HeartbeatConfig, timestamp: &mut impl Timestamp) -> Self {
        Self::from_config(&config.check_in, timestamp, None, None)
    }

    fn from_config(
        config: &CheckInConfig,
        timestamp: &mut impl Timestamp,
        kind: Option<CronKind>,
        digest: Option<String>,
    ) -> Self {
        Self {
            api_key: config.api_key.clone(),
            identifier: config.identifier.clone(),
            timestamp: timestamp.as_secs(),
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
    pub fn request(
        &self,
        timestamp: &mut impl Timestamp,
        kind: CronKind,
    ) -> Result<Request, reqwest::Error> {
        let url = format!("{}/check_ins/cron", self.check_in.endpoint);

        client()
            .post(url)
            .query(&CheckInQuery::from_cron(self, timestamp, kind))
            .build()
    }
}

pub struct HeartbeatConfig {
    pub check_in: CheckInConfig,
}

impl HeartbeatConfig {
    pub fn request(&self, timestamp: &mut impl Timestamp) -> Result<Request, reqwest::Error> {
        let url = format!("{}/check_ins/heartbeats", self.check_in.endpoint);

        client()
            .post(url)
            .query(&CheckInQuery::from_heartbeat(self, timestamp))
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timestamp::tests::{timestamp, EXPECTED_SECS};

    fn check_in_config() -> CheckInConfig {
        CheckInConfig {
            api_key: "some_api_key".to_string(),
            endpoint: "https://some-endpoint.com".to_string(),
            identifier: "some-identifier".to_string(),
        }
    }

    #[test]
    fn cron_config_request() {
        let config = CronConfig {
            check_in: check_in_config(),
            digest: "some-digest".to_string(),
        };

        let request = config.request(&mut timestamp(), CronKind::Start).unwrap();

        assert_eq!(request.method().as_str(), "POST");
        assert_eq!(
            request.url().as_str(),
            format!(
                concat!(
                    "https://some-endpoint.com/check_ins/cron",
                    "?api_key=some_api_key",
                    "&identifier=some-identifier",
                    "&timestamp={}",
                    "&kind=start",
                    "&digest=some-digest"
                ),
                EXPECTED_SECS
            )
        );
    }

    #[test]
    fn heartbeat_config_request() {
        let config = HeartbeatConfig {
            check_in: check_in_config(),
        };

        let request = config.request(&mut timestamp()).unwrap();

        assert_eq!(request.method().as_str(), "POST");
        assert_eq!(
            request.url().as_str(),
            format!(
                concat!(
                    "https://some-endpoint.com/check_ins/heartbeats",
                    "?api_key=some_api_key",
                    "&identifier=some-identifier",
                    "&timestamp={}"
                ),
                EXPECTED_SECS
            )
        );
    }
}
