use actix_web::HttpRequest;
use serde_json::json;

pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    json_error(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, e)
}

pub fn e400<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    json_error(actix_web::http::StatusCode::BAD_REQUEST, e)
}

pub fn e404<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    json_error(actix_web::http::StatusCode::NOT_FOUND, e)
}

pub fn error_handler<T>(e: T, _req: &HttpRequest) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    e400(e)
}

pub fn json_error<T>(status: actix_web::http::StatusCode, e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    let error_json = json!({ "error": e.to_string() });
    actix_web::error::InternalError::from_response(
        "",
        actix_web::HttpResponse::build(status)
            .content_type("application/json")
            .json(error_json),
    )
    .into()
}
