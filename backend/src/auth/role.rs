use sqlx::SqlitePool;

/// Roles ordenados de menos a más permisos: cada uno incluye los del anterior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Ve todo lo registrado.
    Consultor,
    /// Además crea, modifica y borra eventos y su configuración.
    Planificador,
    /// Además pausa/reanuda el scheduler y cancela ejecuciones en curso.
    Administrador,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Consultor => "consultor",
            Role::Planificador => "planificador",
            Role::Administrador => "administrador",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "consultor" => Some(Role::Consultor),
            "planificador" => Some(Role::Planificador),
            "administrador" => Some(Role::Administrador),
            _ => None,
        }
    }
}

/// Rol actual del usuario, leído de la base de datos (no del token) para que un cambio de rol
/// o el borrado del usuario surtan efecto sin esperar a que caduque el JWT.
pub async fn role_of(pool: &SqlitePool, uid: &str) -> Result<Option<Role>, sqlx::Error> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(uid)
        .fetch_optional(pool)
        .await?;
    Ok(role.as_deref().and_then(Role::parse))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_role_includes_the_previous_ones() {
        assert!(Role::Consultor < Role::Planificador);
        assert!(Role::Planificador < Role::Administrador);
    }

    #[test]
    fn parses_what_it_prints() {
        for role in [Role::Consultor, Role::Planificador, Role::Administrador] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        assert_eq!(Role::parse("root"), None);
    }
}
