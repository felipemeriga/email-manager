use actix_web::{middleware as actix_middleware, web, App, HttpServer};
use anyhow::Result;
use email_manager::config::Settings;
use email_manager::handlers;
use email_manager::handlers::emails as email_handlers;
use email_manager::services::gmail::GmailService;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[actix_web::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    info!("Starting Gmail Manager API");

    // Load configuration
    let settings = Settings::from_env().unwrap_or_else(|_| Settings {
        server: email_manager::config::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
        },
        gmail: email_manager::config::GmailConfig {
            service_account_path: std::env::var("GMAIL_SERVICE_ACCOUNT_PATH")
                .unwrap_or_else(|_| "service-account.json".to_string()),
        },
    });

    info!(
        "Configuration loaded: {}:{}",
        settings.server.host, settings.server.port
    );

    // Initialize Gmail service
    let gmail_service = Arc::new(Mutex::new(
        GmailService::new(&settings.gmail.service_account_path).await?,
    ));

    let server_host = settings.server.host.clone();
    let server_port = settings.server.port;

    info!("Gmail service initialized");

    // Create and run HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(gmail_service.clone()))
            .wrap(actix_middleware::Logger::default())
            // Health endpoint
            .route("/health", web::get().to(handlers::health))
            // Email endpoints
            .route(
                "/emails/recent",
                web::get().to(email_handlers::get_recent_emails),
            )
            .route(
                "/emails/today",
                web::get().to(email_handlers::get_today_emails),
            )
            .route(
                "/emails/by-date/{date}",
                web::get().to(email_handlers::get_emails_by_date),
            )
            .route(
                "/emails/search",
                web::post().to(email_handlers::search_emails),
            )
            .route(
                "/emails/{id}/read",
                web::post().to(email_handlers::mark_as_read),
            )
            .route(
                "/emails/{id}/unread",
                web::post().to(email_handlers::mark_as_unread),
            )
            .route(
                "/emails/{id}",
                web::delete().to(email_handlers::delete_email),
            )
            .route(
                "/emails/bulk-delete",
                web::post().to(email_handlers::bulk_delete),
            )
    })
    .bind((&server_host[..], server_port))?
    .run()
    .await?;

    Ok(())
}
