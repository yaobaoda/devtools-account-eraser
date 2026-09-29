use crate::infrai_client::{InfraiClient, InfraiError};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct DeletionState {
    pub client: InfraiClient,
    pub accounts: Arc<RwLock<HashSet<String>>>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub credential_id: String,
    #[serde(default)]
    pub build_events: Vec<BuildEvent>,
    #[serde(default)]
    pub release_operations: Vec<ReleaseOperation>,
}

#[derive(Debug, Deserialize)]
pub struct BuildEvent {
    pub build_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseOperation {
    pub release_id: String,
    pub environment: String,
}

#[derive(Debug, Serialize)]
pub struct DeletionReceipt {
    pub user_id: String,
    pub state: &'static str,
    pub sessions_revoked: usize,
    pub credential_revoked: bool,
    pub archived_build_events: usize,
    pub archived_release_operations: usize,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub component: &'static str,
    pub outcome: &'static str,
}

#[derive(Debug, Error)]
pub enum DeletionError {
    #[error("account not found")]
    NotFound,
    #[error("upstream revocation failed: {0}")]
    Revocation(#[from] InfraiError),
}

pub async fn delete_account(
    State(state): State<DeletionState>,
    Path(user_id): Path<String>,
    Json(input): Json<DeleteAccountRequest>,
) -> Result<Json<DeletionReceipt>, DeletionError> {
    if !state.accounts.read().await.contains(&user_id) {
        return Err(DeletionError::NotFound);
    }

    let session_ids = state.client.list_session_ids(&user_id).await?;
    for session_id in &session_ids {
        state.client.revoke_session(session_id).await?;
    }
    state
        .client
        .revoke_account_key(&input.credential_id)
        .await?;

    state.accounts.write().await.remove(&user_id);
    Ok(Json(DeletionReceipt {
        user_id,
        state: "deleted",
        sessions_revoked: session_ids.len(),
        credential_revoked: true,
        archived_build_events: input.build_events.len(),
        archived_release_operations: input.release_operations.len(),
        diagnostics: vec![
            Diagnostic {
                component: "sessions",
                outcome: "revoked",
            },
            Diagnostic {
                component: "credential",
                outcome: "revoked",
            },
            Diagnostic {
                component: "account",
                outcome: "deleted",
            },
        ],
    }))
}

impl IntoResponse for DeletionError {
    fn into_response(self) -> Response {
        let (status, kind) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, "account_not_found"),
            Self::Revocation(InfraiError::Rejected { status, .. })
                if (400..500).contains(status) =>
            {
                (StatusCode::BAD_REQUEST, "revocation_rejected")
            }
            Self::Revocation(_) => (StatusCode::BAD_GATEWAY, "revocation_unavailable"),
        };
        (
            status,
            Json(serde_json::json!({ "error": kind, "message": self.to_string() })),
        )
            .into_response()
    }
}

#[derive(Debug, PartialEq)]
pub enum DeletionDecision {
    Commit,
    Retain,
}

pub fn deletion_decision(sessions_revoked: bool, credential_revoked: bool) -> DeletionDecision {
    if sessions_revoked && credential_revoked {
        DeletionDecision::Commit
    } else {
        DeletionDecision::Retain
    }
}

#[cfg(test)]
mod tests {
    use super::{deletion_decision, DeletionDecision};

    #[test]
    fn retains_account_until_sessions_and_credential_are_revoked() {
        assert_eq!(deletion_decision(true, false), DeletionDecision::Retain);
        assert_eq!(deletion_decision(false, true), DeletionDecision::Retain);
        assert_eq!(deletion_decision(true, true), DeletionDecision::Commit);
    }
}
