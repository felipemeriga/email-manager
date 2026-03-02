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

    // Determine authentication method
    let use_oauth2 = std::env::var("USE_OAUTH2")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Initialize Gmail service with appropriate authentication
    let gmail_service = if use_oauth2 {
        info!("Using OAuth2 authentication (for personal Gmail accounts)");
        info!("First run will open browser for authentication");

        let client_secret_path = std::env::var("OAUTH2_CLIENT_SECRET_PATH")
            .unwrap_or_else(|_| "client_secret.json".to_string());

        match GmailService::new_with_oauth2(&client_secret_path).await {
            Ok(service) => Arc::new(Mutex::new(service)),
            Err(e) => {
                tracing::error!("Failed to initialize Gmail service with OAuth2: {}", e);
                tracing::error!("Make sure you have:");
                tracing::error!("1. Created OAuth2 credentials in Google Cloud Console");
                tracing::error!("2. Downloaded the client_secret.json file");
                tracing::error!("3. Set USE_OAUTH2=true and OAUTH2_CLIENT_SECRET_PATH=path/to/client_secret.json");
                return Err(anyhow::anyhow!("OAuth2 initialization failed: {}", e));
            }
        }
    } else {
        info!("Using Service Account authentication");

        // Check if user email is configured for impersonation
        if let Ok(user_email) = std::env::var("GMAIL_USER_EMAIL") {
            info!("Will impersonate user: {}", user_email);
            info!("Note: This requires Google Workspace with domain-wide delegation configured");
        } else {
            info!("No GMAIL_USER_EMAIL set");
            info!("Service accounts cannot access personal Gmail accounts!");
            info!("For personal Gmail, set USE_OAUTH2=true instead");
        }

        match GmailService::new(&settings.gmail.service_account_path).await {
            Ok(service) => Arc::new(Mutex::new(service)),
            Err(e) => {
                tracing::error!("Failed to initialize Gmail service with Service Account: {}", e);
                tracing::error!("For personal Gmail accounts, use OAuth2 instead:");
                tracing::error!("Set USE_OAUTH2=true and provide client_secret.json");
                return Err(anyhow::anyhow!("Service Account initialization failed: {}", e));
            }
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
