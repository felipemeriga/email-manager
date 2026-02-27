use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Result;
use tracing::info;
use tracing_subscriber;

#[actix_web::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    info!("Starting Gmail Manager API");

    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy"
    }))
}
