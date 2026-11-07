use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::auth::jwt::verify_token;
use crate::auth::role::{Role, role_of};
use crate::error::AppError;
use crate::state::AppState;

/// Extractor que exige el header `x-token` (el mismo nombre que usa el frontend,
/// ver `calendarApi.js`) con un JWT válido, e inyecta el usuario autenticado.
pub struct AuthUser {
    pub uid: String,
    pub name: String,
    pub role: Role,
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

        // El rol se lee de la base de datos: si el usuario ya no existe, el token no vale.
        let role = role_of(&state.pool, &claims.uid)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Token no válido".to_string()))?;

        Ok(AuthUser {
            uid: claims.uid,
            name: claims.name,
            role,
        })
    }
}

/// Usuario con al menos el rol `MIN`; con uno inferior la petición se rechaza con 403.
pub struct AtLeast<const MIN: u8>(pub AuthUser);

/// Puede modificar eventos y configuración.
pub type Planner = AtLeast<{ Role::Planificador as u8 }>;
/// Puede además controlar el scheduler y gestionar usuarios.
pub type Admin = AtLeast<{ Role::Administrador as u8 }>;

impl<const MIN: u8> std::ops::Deref for AtLeast<MIN> {
    type Target = AuthUser;

    fn deref(&self) -> &AuthUser {
        &self.0
    }
}

impl<const MIN: u8> FromRequestParts<AppState> for AtLeast<MIN> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role as u8 >= MIN {
            Ok(AtLeast(user))
        } else {
            Err(AppError::Forbidden("No tienes permiso para esta operación".to_string()))
        }
    }
}
