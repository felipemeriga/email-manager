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
    info!(
        "Using service account: {}",
        settings.gmail.service_account_path
    );

    // Initialize Gmail service
    // Note: For container deployment, OAuth2 with browser won't work
    info!("Using Service Account authentication");
    info!("Service account path: {}", settings.gmail.service_account_path);

    // Check if user email is configured for impersonation
    if let Ok(user_email) = std::env::var("GMAIL_USER_EMAIL") {
        info!("Will attempt to impersonate user: {}", user_email);
        tracing::warn!("IMPORTANT: Service Account impersonation requires:");
        tracing::warn!("1. Google Workspace (paid G Suite) account - NOT gmail.com");
        tracing::warn!("2. Domain-wide delegation enabled in Google Admin Console");
        tracing::warn!("3. Service account authorized with Gmail scopes");
    } else {
        tracing::warn!("No GMAIL_USER_EMAIL set - Service Account will try default access");
        tracing::warn!("Note: Service accounts CANNOT access personal Gmail (gmail.com) accounts");
        tracing::warn!("For personal Gmail in containers, you need:");
        tracing::warn!("1. Use Google Workspace account instead, OR");
        tracing::warn!("2. Use Gmail API with API key (limited operations), OR");
        tracing::warn!("3. Pre-authorize OAuth2 locally and mount the token file");
    }

    let gmail_service = match GmailService::new(&settings.gmail.service_account_path).await {
        Ok(service) => Arc::new(Mutex::new(service)),
        Err(e) => {
            tracing::error!("Failed to initialize Gmail service: {}", e);
            tracing::error!("");
            tracing::error!("Common issues:");
            tracing::error!("1. Gmail API not enabled in Google Cloud Console");
            tracing::error!("2. Service account JSON file invalid or not found");
            tracing::error!("3. Trying to access personal Gmail (service accounts don't work with gmail.com)");
            tracing::error!("4. Missing domain-wide delegation for Google Workspace");
            return Err(anyhow::anyhow!("Gmail service initialization failed: {}", e));
        }
    };

    let server_host = settings.server.host.clone();
    let server_port = settings.server.port;

    info!("Gmail service initialized successfully");

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
