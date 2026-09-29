use reqwest::{header::RETRY_AFTER, Method, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

const MAX_ATTEMPTS: usize = 4;

#[derive(Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<ApiErrorBody>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("Infrai rejected the operation ({status}): {code}: {message}")]
    Rejected {
        status: u16,
        code: String,
        message: String,
    },
    #[error("Infrai response could not be decoded: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("Infrai transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai returned an unsuccessful response without an error envelope ({0})")]
    UnexpectedStatus(u16),
    #[error("Infrai response omitted data")]
    MissingData,
    #[error("Infrai session list had an unexpected shape")]
    InvalidSessionList,
}

impl InfraiClient {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key: api_key.into(),
        }
    }

    pub async fn list_session_ids(&self, user_id: &str) -> Result<Vec<String>, InfraiError> {
        let path = format!("/v1/auth/session/list_for_user/{user_id}");
        let data = self
            .request(Method::GET, &path)
            .await?
            .ok_or(InfraiError::MissingData)?;
        session_ids(&data)
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), InfraiError> {
        let path = format!("/v1/auth/session/revoke/{session_id}");
        self.request(Method::POST, &path).await?;
        Ok(())
    }

    pub async fn revoke_account_key(&self, credential_id: &str) -> Result<(), InfraiError> {
        let path = format!("/v1/account/keys/revoke/{credential_id}");
        self.request(Method::DELETE, &path).await?;
        Ok(())
    }

    async fn request(&self, method: Method, path: &str) -> Result<Option<Value>, InfraiError> {
        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .http
                .request(method.clone(), format!("{}{path}", self.base_url))
                .bearer_auth(&self.api_key)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response.headers().get(RETRY_AFTER).and_then(|value| {
                value
                    .to_str()
                    .ok()?
                    .parse::<u64>()
                    .ok()
                    .map(Duration::from_secs)
            });
            let bytes = response.bytes().await?;
            let envelope: Envelope = serde_json::from_slice(&bytes)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                let delay =
                    retry_after.unwrap_or_else(|| Duration::from_millis(250 * (1 << attempt)));
                tokio::time::sleep(delay).await;
                continue;
            }

            if !envelope.ok {
                if let Some(error) = envelope.error {
                    return Err(InfraiError::Rejected {
                        status: status.as_u16(),
                        code: error.code,
                        message: error.message,
                    });
                }
                return Err(InfraiError::UnexpectedStatus(status.as_u16()));
            }
            if !status.is_success() {
                return Err(InfraiError::UnexpectedStatus(status.as_u16()));
            }
            return Ok(envelope.data);
        }
        unreachable!("retry loop always returns on its final attempt")
    }
}

fn session_ids(data: &Value) -> Result<Vec<String>, InfraiError> {
    let entries = data
        .as_array()
        .or_else(|| data.get("sessions").and_then(Value::as_array))
        .ok_or(InfraiError::InvalidSessionList)?;

    entries
        .iter()
        .map(|entry| {
            entry
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or(InfraiError::InvalidSessionList)
        })
        .collect()
}
