mod app;
mod config;
mod error;
mod routes;
mod state;
mod store;

use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;
    let bind = config.bind.clone();
    let state = state::AppState::new(config)?;
    let startup_store = state.store.clone();
    tokio::spawn(async move {
        if let Err(err) = startup_store.prime_startup_caches().await {
            eprintln!("startup cache prewarm failed: {:?}", err);
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
