use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::models::user::Claims;
use actix_web::{HttpMessage, Error};

pub async fn validate_jwt(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    const SECRET: &[u8] = b"secret";

    let token_data = decode::<Claims>(
        credentials.token(),
        &DecodingKey::from_secret(SECRET),
        &Validation::default(),
    );

    match token_data {
        Ok(data) => {
            // user_idなどをリクエストに保持したければここで挿入可能
            req.extensions_mut().insert(data.claims);
            Ok(req)
        }
        Err(_) => Err((actix_web::error::ErrorUnauthorized("Invalid token"), req)),
    }
}
