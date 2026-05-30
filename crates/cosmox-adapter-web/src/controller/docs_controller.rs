use actix_web::{HttpResponse, Responder, get};

/// Serve the hand-written OpenAPI YAML spec file
#[get("/openapi.yaml")]
pub async fn openapi_yaml() -> impl Responder {
    match std::fs::read_to_string("docs/openapi.yaml") {
        Ok(content) => HttpResponse::Ok()
            .content_type("text/yaml; charset=utf-8")
            .body(content),
        Err(_) => HttpResponse::NotFound()
            .content_type("text/plain; charset=utf-8")
            .body("OpenAPI spec file not found"),
    }
}

/// Render Scalar API documentation UI
#[get("/docs")]
pub async fn scalar_docs() -> impl Responder {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Cosmox API Documentation</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <style>
        body { margin: 0; padding: 0; }
    </style>
</head>
<body>
    <script id="api-reference" data-url="/api/openapi.yaml"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#;
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}
