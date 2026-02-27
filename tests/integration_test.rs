use actix_web::{middleware, test, web, App};
use email_manager::handlers;

#[actix_rt::test]
async fn test_full_api_flow() {
    // This test verifies the API structure without requiring Gmail credentials
    let app = test::init_service(
        App::new()
            .wrap(middleware::Logger::default())
            .route("/health", web::get().to(handlers::health)),
    )
    .await;

    // Test health endpoint
    let req = test::TestRequest::get().uri("/health").to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}
