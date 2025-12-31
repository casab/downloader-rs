use crate::configuration::JwtSettings;
use crate::session_state::TypedSession;
use crate::utils::{e401, e500};
use actix_web::FromRequest;
use actix_web::HttpMessage;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::HeaderValue;
use actix_web::middleware::Next;
use actix_web::web::Data;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct UserId(pub Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

enum AuthMethod {
    Session(TypedSession),
    Jwt(HeaderValue, Data<JwtSettings>),
}

impl AuthMethod {
    async fn validate(&self) -> Result<UserId, actix_web::Error> {
        match self {
            AuthMethod::Session(session) => validate_session(session).await,
            AuthMethod::Jwt(auth_header, jwt_config) => validate_jwt(auth_header, jwt_config).await,
        }
    }
}

pub fn create_jwt_token(
    user_id: Uuid,
    config: &JwtSettings,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(config.expiration_hours))
        .expect("Failed to calculate expiration date")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.expose_secret().as_bytes()),
    )
}

async fn validate_jwt(
    auth_header: &HeaderValue,
    config: &JwtSettings,
) -> Result<UserId, actix_web::Error> {
    let auth_type_prefix = "Bearer ";
    if let Ok(auth_str) = auth_header.to_str() {
        if let Some(token) = auth_str.strip_prefix(auth_type_prefix) {
            match decode::<Claims>(
                token,
                &DecodingKey::from_secret(config.secret.expose_secret().as_bytes()),
                &Validation::default(),
            ) {
                Ok(token_data) => {
                    return Ok(UserId(token_data.claims.sub));
                }
                Err(e) => return Err(e401(e)),
            }
        }
    }

    Err(e401("Missing or invalid Authorization header"))
}

async fn validate_session(session: &TypedSession) -> Result<UserId, actix_web::Error> {
    match session.get_user_id().map_err(e500)? {
        Some(user_id) => Ok(UserId(user_id)),
        None => Err(e401("The user has not logged in")),
    }
}

pub async fn reject_anonymous_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let auth_method = if let Some(auth_header) = req.headers().get("Authorization") {
        let jwt_settings = req
            .app_data::<Data<JwtSettings>>()
            .expect("JWT configuration must be set")
            .clone();
        AuthMethod::Jwt(auth_header.clone(), jwt_settings)
    } else {
        let (http_request, payload) = req.parts_mut();
        AuthMethod::Session(TypedSession::from_request(http_request, payload).await?)
    };

    let user_id = auth_method.validate().await?;
    req.extensions_mut().insert(user_id);
    next.call(req).await
}
