//! Cliente REST hacia un sistema SCADA (Ignition, WinCC, AVEVA...). La forma de `config`
//! y `address` es la que valida el backend para `protocol = 'rest'`.

use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder};
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    None,
    Bearer,
    Basic,
    ApiKey,
}

#[derive(Debug, Deserialize)]
pub struct RestConfig {
    pub base_url: String,
    pub auth_type: AuthType,
    #[serde(default)]
    pub credentials_ref: String,
    pub timeout_ms: u64,
    /// Solo para `api_key`; por defecto `X-API-Key`.
    pub api_key_header: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestAddress {
    pub method: String,
    pub path: String,
    pub json_pointer: String,
}

#[derive(Debug)]
pub enum RestError {
    /// Configuración, dirección o credenciales inválidas: no se llegó a enviar nada.
    Invalid(String),
    Timeout(String),
    Transport(String),
    /// El SCADA respondió con un código no 2xx.
    Status(u16, String),
}

impl std::fmt::Display for RestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestError::Invalid(msg) | RestError::Timeout(msg) | RestError::Transport(msg) => f.write_str(msg),
            RestError::Status(code, body) => write!(f, "El SCADA respondió HTTP {code}: {body}"),
        }
    }
}

/// Nombre de la variable de entorno que guarda el secreto de `credentials_ref`.
pub fn credential_env_name(credentials_ref: &str) -> String {
    let suffix: String = credentials_ref
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect();
    format!("SCHEDULER_CRED_{suffix}")
}

pub fn build_url(base_url: &str, path: &str) -> String {
    format!("{}/{}", base_url.trim_end_matches('/'), path.trim_start_matches('/'))
}

fn parse_method(raw: &str) -> Result<Method, RestError> {
    match raw.to_ascii_uppercase().as_str() {
        "PUT" => Ok(Method::PUT),
        "POST" => Ok(Method::POST),
        "PATCH" => Ok(Method::PATCH),
        other => Err(RestError::Invalid(format!("address.method no soportado para escribir: {other}"))),
    }
}

fn unescape(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

/// Cuerpo de la escritura: un objeto JSON con `value` colocado en `json_pointer`
/// (`/value` -> `{"value": 1}`, `/a/b` -> `{"a":{"b":1}}`, `""` -> el valor solo).
pub fn build_body(json_pointer: &str, value: Value) -> Result<Value, RestError> {
    if json_pointer.is_empty() {
        return Ok(value);
    }
    let tokens = json_pointer
        .strip_prefix('/')
        .ok_or_else(|| RestError::Invalid("address.json_pointer debe ser vacío o empezar por '/'".to_string()))?;
    let mut body = value;
    for token in tokens.split('/').rev() {
        let mut obj = Map::new();
        obj.insert(unescape(token), body);
        body = Value::Object(obj);
    }
    Ok(body)
}

/// Compara el valor pedido con el leído; los números se comparan como f64 (`1` == `1.0`).
pub fn values_match(expected: &Value, actual: &Value) -> bool {
    match (expected.as_f64(), actual.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => expected == actual,
    }
}

pub struct RestClient {
    http: Client,
}

impl Default for RestClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RestClient {
    pub fn new() -> Self {
        Self { http: Client::new() }
    }

    fn request(&self, method: Method, cfg: &RestConfig, path: &str) -> Result<RequestBuilder, RestError> {
        let builder = self
            .http
            .request(method, build_url(&cfg.base_url, path))
            .timeout(Duration::from_millis(cfg.timeout_ms));

        if matches!(cfg.auth_type, AuthType::None) {
            return Ok(builder);
        }
        let env_name = credential_env_name(&cfg.credentials_ref);
        let secret = std::env::var(&env_name)
            .map_err(|_| RestError::Invalid(format!("Falta la variable de entorno {env_name} con las credenciales")))?;

        Ok(match cfg.auth_type {
            AuthType::None => builder,
            AuthType::Bearer => builder.bearer_auth(secret),
            AuthType::Basic => {
                let (user, pass) = secret.split_once(':').ok_or_else(|| {
                    RestError::Invalid(format!("{env_name} debe tener la forma usuario:contraseña"))
                })?;
                builder.basic_auth(user, Some(pass))
            }
            AuthType::ApiKey => builder.header(cfg.api_key_header.as_deref().unwrap_or("X-API-Key"), secret),
        })
    }

    async fn send(&self, builder: RequestBuilder) -> Result<String, RestError> {
        let response = builder.send().await.map_err(map_transport_error)?;
        let status = response.status();
        let text = response.text().await.map_err(map_transport_error)?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(RestError::Status(status.as_u16(), text))
        }
    }

    /// Escribe `value` en el tag. Devuelve el cuerpo de la respuesta del SCADA.
    pub async fn write(&self, cfg: &RestConfig, addr: &RestAddress, value: Value) -> Result<String, RestError> {
        let body = build_body(&addr.json_pointer, value)?;
        let builder = self.request(parse_method(&addr.method)?, cfg, &addr.path)?.json(&body);
        self.send(builder).await
    }

    /// Lee el valor actual del tag (`GET path` + `json_pointer` sobre la respuesta).
    pub async fn read(&self, cfg: &RestConfig, addr: &RestAddress) -> Result<Value, RestError> {
        let text = self.send(self.request(Method::GET, cfg, &addr.path)?).await?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| RestError::Transport(format!("La respuesta del SCADA no es JSON: {e}")))?;
        json.pointer(&addr.json_pointer)
            .cloned()
            .ok_or_else(|| RestError::Transport(format!("La respuesta no contiene {}", addr.json_pointer)))
    }
}

fn map_transport_error(e: reqwest::Error) -> RestError {
    // Sin URL: podría contener credenciales en la query.
    let e = e.without_url();
    if e.is_timeout() {
        RestError::Timeout(format!("Timeout llamando al SCADA: {e}"))
    } else {
        RestError::Transport(format!("Error llamando al SCADA: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_body_from_json_pointer() {
        assert_eq!(build_body("/value", json!(5)).unwrap(), json!({ "value": 5 }));
        assert_eq!(build_body("/a/b", json!(true)).unwrap(), json!({ "a": { "b": true } }));
        assert_eq!(build_body("/a~1b", json!(1)).unwrap(), json!({ "a/b": 1 }));
        assert_eq!(build_body("", json!("auto")).unwrap(), json!("auto"));
        assert!(build_body("value", json!(1)).is_err());
    }

    #[test]
    fn joins_url_without_double_slashes() {
        assert_eq!(build_url("https://h/api/", "/tags/1"), "https://h/api/tags/1");
        assert_eq!(build_url("https://h/api", "tags/1"), "https://h/api/tags/1");
    }

    #[test]
    fn derives_credential_env_name() {
        assert_eq!(credential_env_name("ignition-prod.1"), "SCHEDULER_CRED_IGNITION_PROD_1");
    }

    #[test]
    fn compares_numbers_loosely() {
        assert!(values_match(&json!(1), &json!(1.0)));
        assert!(!values_match(&json!(1), &json!(2)));
        assert!(values_match(&json!("auto"), &json!("auto")));
        assert!(!values_match(&json!(true), &json!(false)));
    }

    #[test]
    fn only_allows_write_methods() {
        assert!(parse_method("put").is_ok());
        assert!(parse_method("DELETE").is_err());
    }
}
