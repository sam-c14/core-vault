use cv_transaction_api::{create_router_with_state, AppState};
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Initialize structured JSON logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,corevault=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("Initializing CoreVault V1 Monolith...");

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://cv_user:cv_password@localhost:5432/corevault_db".to_string()
    });

    info!("Establishing PostgreSQL database connection pool...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;
    info!("PostgreSQL connection pool established successfully");

    let state = AppState::new_with_pool(pool);
    let app = create_router_with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("CoreVault HTTP server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
