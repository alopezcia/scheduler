use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::auth::jwt::verify_token;
use crate::error::AppError;
use crate::state::AppState;

/// Extractor que exige el header `x-token` (el mismo nombre que usa el frontend,
/// ver `calendarApi.js`) con un JWT válido, e inyecta el usuario autenticado.
pub struct AuthUser {
    pub uid: String,
    pub name: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("x-token")
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Unauthorized("No hay token en la petición".to_string()))?;

        let claims = verify_token(token, &state.jwt_secret)
            .map_err(|_| AppError::Unauthorized("Token no válido".to_string()))?;

        Ok(AuthUser {
            uid: claims.uid,
            name: claims.name,
        })
    }
}
