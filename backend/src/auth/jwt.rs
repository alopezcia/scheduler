use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub uid: String,
    pub name: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn generate_token(
    uid: &str,
    name: &str,
    secret: &str,
    expires_seconds: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp() as usize;

    let claims = Claims {
        uid: uid.to_string(),
        name: name.to_string(),
        iat: now,
        exp: now + expires_seconds as usize,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;

    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_a_token_that_verifies_back_to_the_same_claims() {
        let token = generate_token("uid-123", "Fernando", "secreto", 3600).unwrap();
        let claims = verify_token(&token, "secreto").unwrap();

        assert_eq!(claims.uid, "uid-123");
        assert_eq!(claims.name, "Fernando");
        assert_eq!(claims.exp - claims.iat, 3600);
    }

    #[test]
    fn rejects_a_token_signed_with_a_different_secret() {
        let token = generate_token("uid-123", "Fernando", "secreto", 3600).unwrap();
        assert!(verify_token(&token, "otro-secreto").is_err());
    }

    #[test]
    fn rejects_an_expired_token() {
        let claims = Claims {
            uid: "uid-123".to_string(),
            name: "Fernando".to_string(),
            iat: 0,
            exp: 1,
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"secreto"),
        )
        .unwrap();

        assert!(verify_token(&token, "secreto").is_err());
    }
}
