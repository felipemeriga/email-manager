use actix_web::{middleware as actix_middleware, web, App, HttpServer};
use anyhow::Result;
use email_manager::config::Settings;
use email_manager::handlers;
use email_manager::handlers::emails as email_handlers;
use email_manager::services::imap_service::ImapService;
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
        email: email_manager::config::EmailConfig {
            email_address: std::env::var("GMAIL_EMAIL")
                .unwrap_or_else(|_| "your-email@gmail.com".to_string()),
            app_password: std::env::var("GMAIL_APP_PASSWORD")
                .unwrap_or_else(|_| "your-app-password".to_string()),
        },
    });

    info!(
        "Configuration loaded: {}:{}",
        settings.server.host, settings.server.port
    );
    info!(
        "Using IMAP authentication for email: {}",
        settings.email.email_address
    );

    // Initialize IMAP service
    info!("Initializing IMAP service...");
    info!("Connecting to Gmail IMAP server (imap.gmail.com:993)");

    let email_service = Arc::new(Mutex::new(ImapService::new(
        settings.email.email_address.clone(),
        settings.email.app_password.clone(),
    )));

    info!("IMAP service initialized successfully");
    info!("Note: Make sure you're using an App Password, not your regular Gmail password");
    info!("Create one at: https://myaccount.google.com/apppasswords");

    let server_host = settings.server.host.clone();
    let server_port = settings.server.port;

    info!("Email service ready");

    // Create and run HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(email_service.clone()))
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
            .route(
                "/emails/bulk-mark-read",
                web::post().to(email_handlers::bulk_mark_as_read),
            )
    })
    .bind((&server_host[..], server_port))?
    .run()
    .await?;

    Ok(())
}
