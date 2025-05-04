use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use jsonwebtoken::{decode, errors::ErrorKind as JwtErrorKind, DecodingKey, Validation};
use crate::models::user::Claims;
use actix_web::{HttpMessage, Error};

const SECRET: &[u8] = b"secret";

pub async fn validate_jwt(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {

    let token_data = decode::<Claims>(
        credentials.token(),
        &DecodingKey::from_secret(SECRET),
        &Validation::default(),
    );

    match token_data {
        Ok(data) => {
            req.extensions_mut().insert(data.claims);
            Ok(req)
        }
        Err(_) => Err((actix_web::error::ErrorUnauthorized("Invalid token"), req)),
    }
}

pub fn decode_jwt(token: &str) -> Result<i32, String> {
    let decoding_key = DecodingKey::from_secret(SECRET);
    let validation = Validation::default();

    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(c) => {
            c.claims.sub.parse::<i32>().map_err(|_| "Invalid user_id format".to_string())
        },
        Err(e) => match e.kind() {
            JwtErrorKind::ExpiredSignature => Err("Token expired".to_string()),
            JwtErrorKind::InvalidToken => Err("Invalid token".to_string()),
            JwtErrorKind::ImmatureSignature => Err("Token is not yet valid".to_string()),
            _ => Err("Invalid token".to_string()),
        },
    }
}
