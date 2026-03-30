mod app;
mod config;
mod error;
mod routes;
mod state;
mod store;

use anyhow::{Context, Result};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;
    let bind = config.bind.clone();
    let state = state::AppState::new(config)?;
    let startup_store = state.store.clone();
    tokio::spawn(async move {
        let mut last_err = None;
        for attempt in 0..5 {
            match startup_store.prime_startup_caches().await {
                Ok(()) => return,
                Err(err) => {
                    last_err = Some(err);
                    if attempt < 4 {
                        tokio::time::sleep(Duration::from_millis(250 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }
        if let Some(err) = last_err {
            eprintln!("startup cache prewarm failed after retries: {:?}", err);
        }
    });
    let app = app::build_router(state);

    let addr = bind
        .parse()
        .with_context(|| format!("invalid bind address {bind}"))?;
    println!("fishystuff_server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("serve axum")?;

    Ok(())
}
