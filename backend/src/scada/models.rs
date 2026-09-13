use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Json;

// ---------- Enums ----------
// Se representan como TEXT en SQLite (sin `type_name`, ya que sqlx los
// codifica/decodifica como string por variante para backends sin enum nativo).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Protocol {
    Opcua,
    Mqtt,
    S7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Unknown,
    Online,
    Offline,
    Error,
}

/// Naturaleza del punto SCADA: orden discreta al PLC, referencia analógica, o
/// solo lectura de estado/medida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum TagKind {
    Telemando,
    Consigna,
    Medida,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum DataType {
    Bool,
    Int,
    Float,
    Enum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ReadWrite {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum TriggerType {
    Once,
    Cron,
    CalendarEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum InterlockOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum InterlockAction {
    Block,
    Warn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Sent,
    Ack,
    Failed,
    Timeout,
}

fn default_true() -> bool {
    true
}

fn default_read_write() -> ReadWrite {
    ReadWrite::Write
}

fn default_interlock_action() -> InterlockAction {
    InterlockAction::Block
}

// ---------- Sites ----------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SiteRow {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SitePayload {
    pub name: String,
}

// ---------- Assets ----------
// Árbol planta -> área -> equipo -> PLC, vía `parent_asset_id` autorreferente.

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AssetRow {
    pub id: String,
    pub site_id: String,
    pub parent_asset_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetPayload {
    pub site_id: String,
    #[serde(default)]
    pub parent_asset_id: Option<String>,
    pub name: String,
    pub kind: String,
}

// ---------- Connections ----------

#[derive(Debug, sqlx::FromRow)]
pub struct ConnectionRow {
    pub id: String,
    pub asset_id: String,
    pub name: String,
    pub protocol: Protocol,
    pub config: Json<Value>,
    pub status: ConnectionStatus,
    pub last_seen: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    pub id: String,
    pub asset_id: String,
    pub name: String,
    pub protocol: Protocol,
    pub config: Value,
    pub status: ConnectionStatus,
    pub last_seen: Option<String>,
}

impl From<ConnectionRow> for ConnectionResponse {
    fn from(row: ConnectionRow) -> Self {
        ConnectionResponse {
            id: row.id,
            asset_id: row.asset_id,
            name: row.name,
            protocol: row.protocol,
            config: row.config.0,
            status: row.status,
            last_seen: row.last_seen,
        }
    }
}

/// `config` depende del protocolo:
/// - opcua: `{ "endpoint_url", "security_policy", "credentials_ref" }`
/// - mqtt: `{ "broker_url", "base_topic", "qos", "tls" }`
/// - s7: `{ "ip", "rack", "slot" }`
#[derive(Debug, Deserialize)]
pub struct ConnectionPayload {
    pub asset_id: String,
    pub name: String,
    pub protocol: Protocol,
    pub config: Value,
}

// ---------- Tags ----------

#[derive(Debug, sqlx::FromRow)]
pub struct TagRow {
    pub id: String,
    pub connection_id: String,
    pub asset_id: String,
    pub name: String,
    pub description: String,
    pub tag_kind: TagKind,
    pub data_type: DataType,
    pub unit: Option<String>,
    pub address: Json<Value>,
    pub read_write: ReadWrite,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub allowed_values: Option<Json<Value>>,
    pub requires_sbo: bool,
    pub requires_ack: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub id: String,
    pub connection_id: String,
    pub asset_id: String,
    pub name: String,
    pub description: String,
    pub tag_kind: TagKind,
    pub data_type: DataType,
    pub unit: Option<String>,
    pub address: Value,
    pub read_write: ReadWrite,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub allowed_values: Option<Value>,
    pub requires_sbo: bool,
    pub requires_ack: bool,
}

impl From<TagRow> for TagResponse {
    fn from(row: TagRow) -> Self {
        TagResponse {
            id: row.id,
            connection_id: row.connection_id,
            asset_id: row.asset_id,
            name: row.name,
            description: row.description,
            tag_kind: row.tag_kind,
            data_type: row.data_type,
            unit: row.unit,
            address: row.address.0,
            read_write: row.read_write,
            min_value: row.min_value,
            max_value: row.max_value,
            allowed_values: row.allowed_values.map(|v| v.0),
            requires_sbo: row.requires_sbo,
            requires_ack: row.requires_ack,
        }
    }
}

/// `address` depende del protocolo de la `connection` asociada:
/// - opcua: `{ "node_id" }`
/// - mqtt: `{ "topic", "json_pointer" }`
/// - s7: `{ "db_number", "offset", "bit", "s7_type" }`
#[derive(Debug, Deserialize)]
pub struct TagPayload {
    pub connection_id: String,
    pub asset_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub tag_kind: TagKind,
    pub data_type: DataType,
    #[serde(default)]
    pub unit: Option<String>,
    pub address: Value,
    #[serde(default = "default_read_write")]
    pub read_write: ReadWrite,
    #[serde(default)]
    pub min_value: Option<f64>,
    #[serde(default)]
    pub max_value: Option<f64>,
    #[serde(default)]
    pub allowed_values: Option<Value>,
    #[serde(default)]
    pub requires_sbo: bool,
    #[serde(default)]
    pub requires_ack: bool,
}

// ---------- Schedules ----------
// Generaliza `events::EventPayload`: en vez de solo título/notas, una
// programación apunta a un tag y a un valor objetivo.

#[derive(Debug, sqlx::FromRow)]
pub struct ScheduleRow {
    pub id: String,
    pub name: String,
    pub tag_id: String,
    pub target_value: Json<Value>,
    pub trigger_type: TriggerType,
    pub cron_expr: Option<String>,
    pub event_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub enabled: bool,
    pub requires_confirmation: bool,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ScheduleResponse {
    pub id: String,
    pub name: String,
    pub tag_id: String,
    pub target_value: Value,
    pub trigger_type: TriggerType,
    pub cron_expr: Option<String>,
    pub event_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub enabled: bool,
    pub requires_confirmation: bool,
    pub created_by: String,
}

impl From<ScheduleRow> for ScheduleResponse {
    fn from(row: ScheduleRow) -> Self {
        ScheduleResponse {
            id: row.id,
            name: row.name,
            tag_id: row.tag_id,
            target_value: row.target_value.0,
            trigger_type: row.trigger_type,
            cron_expr: row.cron_expr,
            event_id: row.event_id,
            start_date: row.start_date,
            end_date: row.end_date,
            enabled: row.enabled,
            requires_confirmation: row.requires_confirmation,
            created_by: row.created_by,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SchedulePayload {
    pub name: String,
    pub tag_id: String,
    /// Debe respetar el `data_type` del tag: booleano, número, o uno de sus
    /// `allowed_values` si es de tipo `enum`.
    pub target_value: Value,
    pub trigger_type: TriggerType,
    #[serde(default)]
    pub cron_expr: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub requires_confirmation: bool,
}

// ---------- Interlocks ----------

#[derive(Debug, sqlx::FromRow)]
pub struct InterlockRow {
    pub id: String,
    pub tag_id: String,
    pub condition_tag_id: String,
    pub operator: InterlockOperator,
    pub condition_value: Json<Value>,
    pub action: InterlockAction,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct InterlockResponse {
    pub id: String,
    pub tag_id: String,
    pub condition_tag_id: String,
    pub operator: InterlockOperator,
    pub condition_value: Value,
    pub action: InterlockAction,
}

impl From<InterlockRow> for InterlockResponse {
    fn from(row: InterlockRow) -> Self {
        InterlockResponse {
            id: row.id,
            tag_id: row.tag_id,
            condition_tag_id: row.condition_tag_id,
            operator: row.operator,
            condition_value: row.condition_value.0,
            action: row.action,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InterlockPayload {
    pub tag_id: String,
    pub condition_tag_id: String,
    pub operator: InterlockOperator,
    pub condition_value: Value,
    #[serde(default = "default_interlock_action")]
    pub action: InterlockAction,
}

// ---------- Command executions ----------
// Auditoría de cada intento de escritura a un tag (programado o manual).

#[derive(Debug, sqlx::FromRow)]
pub struct CommandExecutionRow {
    pub id: String,
    pub schedule_id: Option<String>,
    pub tag_id: String,
    pub requested_value: Json<Value>,
    pub requested_by: String,
    pub status: ExecutionStatus,
    pub sent_at: Option<String>,
    pub ack_at: Option<String>,
    pub response: Option<Json<Value>>,
    pub error_message: Option<String>,
    pub latency_ms: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CommandExecutionResponse {
    pub id: String,
    pub schedule_id: Option<String>,
    pub tag_id: String,
    pub requested_value: Value,
    pub requested_by: String,
    pub status: ExecutionStatus,
    pub sent_at: Option<String>,
    pub ack_at: Option<String>,
    pub response: Option<Value>,
    pub error_message: Option<String>,
    pub latency_ms: Option<i64>,
}

impl From<CommandExecutionRow> for CommandExecutionResponse {
    fn from(row: CommandExecutionRow) -> Self {
        CommandExecutionResponse {
            id: row.id,
            schedule_id: row.schedule_id,
            tag_id: row.tag_id,
            requested_value: row.requested_value.0,
            requested_by: row.requested_by,
            status: row.status,
            sent_at: row.sent_at,
            ack_at: row.ack_at,
            response: row.response.map(|v| v.0),
            error_message: row.error_message,
            latency_ms: row.latency_ms,
        }
    }
}

/// Dispara un comando manual sobre un tag (fuera de un `schedule`), p. ej.
/// desde un botón de telemando en la UI.
#[derive(Debug, Deserialize)]
pub struct CommandExecutionPayload {
    pub tag_id: String,
    pub requested_value: Value,
}
