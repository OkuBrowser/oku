use connexa::prelude::MultiaddrExt;
use ipld_core::cid::Cid;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{Error, Ipfs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Origin {
    #[allow(dead_code)]
    All,
    #[default]
    Public,
}

#[derive(Clone)]
pub struct RemotePinningService {
    ipfs: Ipfs,
    client: reqwest::Client,
    endpoint: String,
    token: String,
    origin: Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Queued,
    Pinning,
    Pinned,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinObject {
    pub cid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origins: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub meta: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PinStatus {
    pub requestid: String,
    pub status: Status,
    pub created: String,
    pub pin: PinObject,
    #[serde(default)]
    pub delegates: Vec<String>,
    #[serde(default)]
    pub info: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PinResults {
    pub count: u64,
    pub results: Vec<PinStatus>,
}

#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    pub name: Option<String>,
    pub origins: Vec<String>,
    pub meta: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    pub cids: Vec<Cid>,
    pub name: Option<String>,
    pub status: Vec<Status>,
    pub limit: Option<u32>,
}

impl RemotePinningService {
    pub(crate) fn new(ipfs: Ipfs, endpoint: impl Into<String>, token: impl Into<String>) -> Self {
        let endpoint = endpoint.into().trim_end_matches('/').to_string();
        Self {
            ipfs,
            client: reqwest::Client::new(),
            endpoint,
            token: token.into(),
            origin: Origin::default(),
        }
    }

    pub async fn add(&self, cid: Cid, options: AddOptions) -> Result<PinStatus, Error> {
        let body = self.pin_object(cid, options).await;
        let response = self
            .client
            .post(format!("{}/pins", self.endpoint))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;
        parse(response).await
    }

    pub async fn status(&self, requestid: &str) -> Result<PinStatus, Error> {
        let response = self
            .client
            .get(format!("{}/pins/{requestid}", self.endpoint))
            .bearer_auth(&self.token)
            .send()
            .await?;
        parse(response).await
    }

    pub async fn list(&self, query: ListQuery) -> Result<PinResults, Error> {
        let mut request = self
            .client
            .get(format!("{}/pins", self.endpoint))
            .bearer_auth(&self.token);

        if !query.cids.is_empty() {
            let cids = query
                .cids
                .iter()
                .map(Cid::to_string)
                .collect::<Vec<_>>()
                .join(",");
            request = request.query(&[("cid", cids)]);
        }
        if let Some(name) = query.name {
            request = request.query(&[("name", name)]);
        }
        if !query.status.is_empty() {
            let statuses = query
                .status
                .iter()
                .map(status_str)
                .collect::<Vec<_>>()
                .join(",");
            request = request.query(&[("status", statuses)]);
        }
        if let Some(limit) = query.limit {
            request = request.query(&[("limit", limit.to_string())]);
        }

        let response = request.send().await?;
        parse(response).await
    }

    pub async fn replace(
        &self,
        requestid: &str,
        cid: Cid,
        options: AddOptions,
    ) -> Result<PinStatus, Error> {
        let body = self.pin_object(cid, options).await;
        let response = self
            .client
            .post(format!("{}/pins/{requestid}", self.endpoint))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;
        parse(response).await
    }

    pub async fn remove(&self, requestid: &str) -> Result<(), Error> {
        let response = self
            .client
            .delete(format!("{}/pins/{requestid}", self.endpoint))
            .bearer_auth(&self.token)
            .send()
            .await?;
        success(response).await.map(|_| ())
    }

    async fn pin_object(&self, cid: Cid, options: AddOptions) -> PinObject {
        let origins = if options.origins.is_empty() {
            self.local_origins().await
        } else {
            options.origins
        };
        PinObject {
            cid: cid.to_string(),
            name: options.name,
            origins,
            meta: options.meta,
        }
    }

    async fn local_origins(&self) -> Vec<String> {
        let peer_id = self.ipfs.keypair().public().to_peer_id();
        let addresses = match self.origin {
            Origin::Public => self
                .ipfs
                .external_addresses()
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|addr| addr.is_public())
                .map(|addr| addr.clone().with_p2p(peer_id).unwrap_or(addr))
                .collect::<HashSet<_>>(),
            Origin::All => {
                let listening = self
                    .ipfs
                    .listening_addresses()
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|addr| addr.clone().with_p2p(peer_id).unwrap_or(addr))
                    .collect::<HashSet<_>>();

                let external = self
                    .ipfs
                    .external_addresses()
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|addr| addr.clone().with_p2p(peer_id).unwrap_or(addr))
                    .collect::<HashSet<_>>();

                listening
                    .into_iter()
                    .chain(external)
                    .collect::<HashSet<_>>()
            }
        };

        Vec::from_iter(addresses.into_iter().map(|addr| addr.to_string()))
    }
}

fn status_str(status: &Status) -> &'static str {
    match status {
        Status::Queued => "queued",
        Status::Pinning => "pinning",
        Status::Pinned => "pinned",
        Status::Failed => "failed",
    }
}

async fn parse<T>(response: reqwest::Response) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let response = success(response).await?;
    response.json::<T>().await.map_err(Error::from)
}

async fn success(response: reqwest::Response) -> Result<reqwest::Response, Error> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    match response.json::<Failure>().await {
        Ok(failure) => Err(anyhow::anyhow!(
            "pinning service error {status}: {}",
            failure.error.reason
        )),
        Err(_) => Err(anyhow::anyhow!("pinning service error {status}")),
    }
}

#[derive(Deserialize)]
struct Failure {
    error: FailureReason,
}

#[derive(Deserialize)]
struct FailureReason {
    reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_pin_status() {
        let json = r#"{
            "requestid": "UniqueIdOfPinRequest",
            "status": "pinned",
            "created": "2020-07-27T17:32:28.276Z",
            "pin": { "cid": "QmCIDToBePinned", "name": "my-pin" },
            "delegates": ["/dnsaddr/pin-service.example.com"]
        }"#;
        let status: PinStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.requestid, "UniqueIdOfPinRequest");
        assert_eq!(status.status, Status::Pinned);
        assert_eq!(status.pin.cid, "QmCIDToBePinned");
        assert_eq!(status.pin.name.as_deref(), Some("my-pin"));
        assert_eq!(status.delegates, vec!["/dnsaddr/pin-service.example.com"]);
    }

    #[test]
    fn serialize_pin_object_omits_empty() {
        let pin = PinObject {
            cid: "QmFoo".into(),
            name: None,
            origins: vec![],
            meta: HashMap::new(),
        };
        assert_eq!(
            serde_json::to_value(&pin).unwrap(),
            serde_json::json!({ "cid": "QmFoo" })
        );
    }
}
