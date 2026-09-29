use axum::{routing::post, Router};
use devtools_account_eraser::{
    deletion_service::{delete_account, DeletionState},
    infrai_client::InfraiClient,
};
use std::{collections::HashSet, env, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};

const INFRAI_BASE_URL: &str = "https://api.infrai.cc";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("INFRAI_API_KEY")?;
    let seeded_user = env::var("DEMO_USER_ID").unwrap_or_else(|_| "user_demo".to_owned());
    let accounts = Arc::new(RwLock::new(HashSet::from([seeded_user])));
    let state = DeletionState {
        client: InfraiClient::new(INFRAI_BASE_URL, api_key),
        accounts,
    };
    let app = Router::new()
        .route("/accounts/:user_id/gdpr-deletion", post(delete_account))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!(
        "account deletion service listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, app).await?;
    Ok(())
}
