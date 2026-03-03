use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures::future::LocalBoxFuture;
use std::future::{ready, Ready};

/// Middleware to validate API token
pub struct ApiTokenAuth {
    token: String,
}

impl ApiTokenAuth {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiTokenAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiTokenAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiTokenAuthMiddleware {
            service,
            token: self.token.clone(),
        }))
    }
}

pub struct ApiTokenAuthMiddleware<S> {
    service: S,
    token: String,
}

impl<S, B> Service<ServiceRequest> for ApiTokenAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip auth for health check endpoint
        if req.path() == "/health" {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        // Get Authorization header
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok());

        let expected_token = format!("Bearer {}", self.token);

        match auth_header {
            Some(token) if token == expected_token => {
                // Token is valid, proceed with request
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            _ => {
                // Token is missing or invalid
                let (req, _) = req.into_parts();
                let response = HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Invalid or missing API token"
                }));

                Box::pin(
                    async move { Ok(ServiceResponse::new(req, response).map_into_right_body()) },
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
    }

    #[actix_web::test]
    async fn test_valid_token() {
        let app = test::init_service(
            App::new()
                .wrap(ApiTokenAuth::new("test-token".to_string()))
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", "Bearer test-token"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_invalid_token() {
        let app = test::init_service(
            App::new()
                .wrap(ApiTokenAuth::new("test-token".to_string()))
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", "Bearer wrong-token"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_missing_token() {
        let app = test::init_service(
            App::new()
                .wrap(ApiTokenAuth::new("test-token".to_string()))
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/test").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_health_check_bypasses_auth() {
        let app = test::init_service(
            App::new()
                .wrap(ApiTokenAuth::new("test-token".to_string()))
                .route("/health", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/health").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }
}
