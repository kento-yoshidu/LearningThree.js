use actix_web::{dev::ServiceRequest, HttpRequest, HttpResponse, HttpMessage, Error};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use crate::models::user::Claims;

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

pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret(SECRET);
    let validation = Validation::new(Algorithm::HS256);
    let token_data: TokenData<Claims> = decode(token, &key, &validation)?;

    Ok(token_data.claims)
}

pub fn extract_user_from_jwt(req: &HttpRequest) -> Result<Claims, HttpResponse> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header_str| header_str.strip_prefix("Bearer "))
        .map(String::from);

    let token = match token {
        Some(t) => t,
        None => return Err(HttpResponse::Unauthorized().body("Missing Authorization token")),
    };

    match decode_jwt(&token) {
        Ok(claims) => Ok(claims),
        Err(_) => Err(HttpResponse::Unauthorized().body("Invalid token")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::test::TestRequest;
    use chrono::{Utc, Duration};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use crate::models::user::Claims;

    const SECRET: &[u8] = b"secret";

    fn generate_token(user_id: i32) -> String {
        let claims = Claims {
            user_id: user_id,
            exp: (Utc::now() + Duration::minutes(10)).timestamp() as usize,
            root_folder: 123,
        };

        encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET)).unwrap()
    }

    #[test]
    fn test_decode_jwt_valid_token() {
        let token = generate_token(1);
        let result = decode_jwt(&token);
        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.user_id, 1);
        assert_eq!(claims.root_folder, 123);
    }

        #[test]
    fn test_decode_jwt_invalid_token() {
        let token = "invalid.token.string";
        let result = decode_jwt(token);
        assert!(result.is_err());
    }

    #[actix_web::test]
    async fn test_extract_user_from_jwt_valid() {
        let token = generate_token(1);

        let req = TestRequest::default()
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_http_request();

        let result = extract_user_from_jwt(&req);
        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.user_id, 1);
    }

    #[actix_web::test]
    async fn test_extract_user_from_jwt_missing() {
        let req = TestRequest::default().to_http_request();
        let result = extract_user_from_jwt(&req);
        assert!(result.is_err());
        let res = result.err().unwrap();
        assert_eq!(res.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_extract_user_from_jwt_invalid_token() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer invalid.token"))
            .to_http_request();

        let result = extract_user_from_jwt(&req);
        assert!(result.is_err());
        let res = result.err().unwrap();
        assert_eq!(res.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }
}
