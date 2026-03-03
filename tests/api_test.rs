use actix_web::{test, web, App};
use email_manager::handlers;
use email_manager::middleware::auth::ApiTokenAuth;

#[actix_rt::test]
async fn test_health_endpoint_without_auth() {
    // Health endpoint should work without authentication
    let app = test::init_service(
        App::new()
            .wrap(ApiTokenAuth::new("test-token".to_string()))
            .route("/health", web::get().to(handlers::health)),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Health check should succeed without auth"
    );
}

#[actix_rt::test]
async fn test_protected_endpoint_without_token() {
    use email_manager::handlers::emails as email_handlers;
    use email_manager::services::imap_service::ImapService;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let email_service = Arc::new(Mutex::new(ImapService::new(
        "test@gmail.com".to_string(),
        "test-password".to_string(),
    )));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(email_service))
            .wrap(ApiTokenAuth::new("test-token".to_string()))
            .route(
                "/emails/recent",
                web::get().to(email_handlers::get_recent_emails),
            ),
    )
    .await;

    // Request without token should be rejected
    let req = test::TestRequest::get().uri("/emails/recent").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "Should return 401 Unauthorized without token"
    );
}

#[actix_rt::test]
async fn test_protected_endpoint_with_invalid_token() {
    use email_manager::handlers::emails as email_handlers;
    use email_manager::services::imap_service::ImapService;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let email_service = Arc::new(Mutex::new(ImapService::new(
        "test@gmail.com".to_string(),
        "test-password".to_string(),
    )));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(email_service))
            .wrap(ApiTokenAuth::new("correct-token".to_string()))
            .route(
                "/emails/recent",
                web::get().to(email_handlers::get_recent_emails),
            ),
    )
    .await;

    // Request with wrong token should be rejected
    let req = test::TestRequest::get()
        .uri("/emails/recent")
        .insert_header(("Authorization", "Bearer wrong-token"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "Should return 401 Unauthorized with wrong token"
    );
}

#[actix_rt::test]
async fn test_protected_endpoint_with_valid_token() {
    // Test that with a valid token, we can authenticate successfully
    // We test this with a simple test endpoint that doesn't require external services

    async fn test_protected_handler() -> actix_web::HttpResponse {
        actix_web::HttpResponse::Ok().json(serde_json::json!({"status": "authenticated"}))
    }

    let app = test::init_service(
        App::new()
            .wrap(ApiTokenAuth::new("test-token".to_string()))
            .route("/test-protected", web::get().to(test_protected_handler)),
    )
    .await;

    // Request with correct token should be accepted
    let req = test::TestRequest::get()
        .uri("/test-protected")
        .insert_header(("Authorization", "Bearer test-token"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Should return 200 OK with valid token");
}
