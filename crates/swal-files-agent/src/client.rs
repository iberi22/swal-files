#![deny(unsafe_code)]

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Configuration for the Xavier Cognitive Memory Client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XavierClientConfig {
    pub api_base_url: String,
    pub ws_url: String,
    pub auth_token: Option<String>,
    pub timeout_ms: u64,
}

impl Default for XavierClientConfig {
    fn default() -> Self {
        Self {
            api_base_url: "http://127.0.0.1:8080".to_string(),
            ws_url: "ws://127.0.0.1:8080/ws".to_string(),
            auth_token: None,
            timeout_ms: 5000,
        }
    }
}

/// Represents a single memory entry stored within Xavier Cognitive Memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub score: Option<f32>,
    pub created_at: u64,
}

impl MemoryEntry {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            metadata: HashMap::new(),
            tags: Vec::new(),
            score: None,
            created_at: 0,
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Query parameters for searching Xavier Cognitive Memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub query: String,
    pub limit: usize,
    pub min_score: Option<f32>,
    pub tags: Vec<String>,
}

impl MemoryQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            limit: 10,
            min_score: None,
            tags: Vec::new(),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = Some(min_score);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Search query result from Xavier Cognitive Memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub entries: Vec<MemoryEntry>,
    pub total_matches: usize,
    pub latency_ms: u64,
}

/// WebSocket Event Type for real-time memory synchronization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncEventType {
    Created,
    Updated,
    Deleted,
    BatchSynced,
}

/// Real-time sync event broadcast over WebSocket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySyncEvent {
    pub event_type: SyncEventType,
    pub entry_id: String,
    pub payload: Option<MemoryEntry>,
    pub timestamp: u64,
}

/// Action type for WebSocket synchronization control messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncAction {
    Subscribe,
    Unsubscribe,
    Ping,
    Pong,
    Event,
}

/// WebSocket protocol envelope message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySyncMessage {
    pub action: SyncAction,
    pub payload: Option<serde_json::Value>,
}

/// Custom error type for Xavier Cognitive Memory operations.
#[derive(Debug)]
pub enum XavierClientError {
    HttpError(String),
    JsonError(String),
    WebSocketError(String),
    ConfigError(String),
    ServerError { status: u16, message: String },
}

impl fmt::Display for XavierClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpError(e) => write!(f, "HTTP request failed: {}", e),
            Self::JsonError(e) => write!(f, "JSON serialization error: {}", e),
            Self::WebSocketError(e) => write!(f, "WebSocket error: {}", e),
            Self::ConfigError(e) => write!(f, "Configuration error: {}", e),
            Self::ServerError { status, message } => {
                write!(f, "Server returned status {}: {}", status, message)
            }
        }
    }
}

impl std::error::Error for XavierClientError {}

impl From<reqwest::Error> for XavierClientError {
    fn from(err: reqwest::Error) -> Self {
        Self::HttpError(err.to_string())
    }
}

impl From<serde_json::Error> for XavierClientError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err.to_string())
    }
}

/// Xavier Cognitive Memory REST and WebSocket Client.
pub struct XavierClient {
    config: XavierClientConfig,
    http_client: reqwest::Client,
}

impl XavierClient {
    /// Creates a new `XavierClient` with the given configuration.
    pub fn new(config: XavierClientConfig) -> Result<Self, XavierClientError> {
        if config.api_base_url.is_empty() {
            return Err(XavierClientError::ConfigError(
                "API base URL cannot be empty".to_string(),
            ));
        }

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &XavierClientConfig {
        &self.config
    }

    /// Checks server health status.
    pub async fn check_health(&self) -> Result<bool, XavierClientError> {
        let url = format!("{}/health", self.config.api_base_url.trim_end_matches('/'));
        let mut req = self.http_client.get(&url);

        if let Some(token) = &self.config.auth_token {
            req = req.bearer_auth(token);
        }

        let res = req.send().await?;
        if res.status().is_success() {
            Ok(true)
        } else {
            Err(XavierClientError::ServerError {
                status: res.status().as_u16(),
                message: "Health check endpoint returned non-success".to_string(),
            })
        }
    }

    /// Queries the Xavier Cognitive Memory REST service.
    pub async fn query(&self, query: &MemoryQuery) -> Result<MemorySearchResult, XavierClientError> {
        let url = format!("{}/api/v1/memories/query", self.config.api_base_url.trim_end_matches('/'));
        let mut req = self.http_client.post(&url).json(query);

        if let Some(token) = &self.config.auth_token {
            req = req.bearer_auth(token);
        }

        let res = req.send().await?;
        if res.status().is_success() {
            let result = res.json::<MemorySearchResult>().await?;
            Ok(result)
        } else {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            Err(XavierClientError::ServerError {
                status,
                message: text,
            })
        }
    }

    /// Stores a new or updated memory entry in Xavier Cognitive Memory.
    pub async fn store(&self, entry: &MemoryEntry) -> Result<MemoryEntry, XavierClientError> {
        let url = format!("{}/api/v1/memories", self.config.api_base_url.trim_end_matches('/'));
        let mut req = self.http_client.post(&url).json(entry);

        if let Some(token) = &self.config.auth_token {
            req = req.bearer_auth(token);
        }

        let res = req.send().await?;
        if res.status().is_success() {
            let result = res.json::<MemoryEntry>().await?;
            Ok(result)
        } else {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            Err(XavierClientError::ServerError {
                status,
                message: text,
            })
        }
    }

    /// Deletes a memory entry by ID from Xavier Cognitive Memory.
    pub async fn delete(&self, id: &str) -> Result<bool, XavierClientError> {
        let url = format!(
            "{}/api/v1/memories/{}",
            self.config.api_base_url.trim_end_matches('/'),
            id
        );
        let mut req = self.http_client.delete(&url);

        if let Some(token) = &self.config.auth_token {
            req = req.bearer_auth(token);
        }

        let res = req.send().await?;
        if res.status().is_success() {
            Ok(true)
        } else {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            Err(XavierClientError::ServerError {
                status,
                message: text,
            })
        }
    }

    /// Batch synchronizes multiple memory entries.
    pub async fn batch_sync(&self, entries: &[MemoryEntry]) -> Result<usize, XavierClientError> {
        let url = format!(
            "{}/api/v1/memories/batch",
            self.config.api_base_url.trim_end_matches('/')
        );
        let mut req = self.http_client.post(&url).json(&entries);

        if let Some(token) = &self.config.auth_token {
            req = req.bearer_auth(token);
        }

        let res = req.send().await?;
        if res.status().is_success() {
            #[derive(Deserialize)]
            struct BatchResponse {
                synced_count: usize,
            }
            let resp = res.json::<BatchResponse>().await?;
            Ok(resp.synced_count)
        } else {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            Err(XavierClientError::ServerError {
                status,
                message: text,
            })
        }
    }

    /// Serializes a `MemorySyncMessage` into a JSON string for WebSocket transmission.
    pub fn encode_ws_message(msg: &MemorySyncMessage) -> Result<String, XavierClientError> {
        serde_json::to_string(msg).map_err(XavierClientError::from)
    }

    /// Deserializes a WebSocket text frame into a `MemorySyncMessage`.
    pub fn parse_ws_frame(frame: &str) -> Result<MemorySyncMessage, XavierClientError> {
        serde_json::from_str(frame).map_err(XavierClientError::from)
    }

    /// Extracts a `MemorySyncEvent` from a `MemorySyncMessage` if applicable.
    pub fn extract_sync_event(msg: &MemorySyncMessage) -> Result<MemorySyncEvent, XavierClientError> {
        if msg.action != SyncAction::Event {
            return Err(XavierClientError::WebSocketError(
                "Message action is not SyncAction::Event".to_string(),
            ));
        }

        let payload = msg.payload.as_ref().ok_or_else(|| {
            XavierClientError::WebSocketError("Missing payload in sync message".to_string())
        })?;

        serde_json::from_value::<MemorySyncEvent>(payload.clone()).map_err(XavierClientError::from)
    }

    /// Spawns a Tokio task that receives simulated/parsed WebSocket sync messages from an async stream channel
    /// and forwards processed `MemorySyncEvent`s to an event receiver.
    pub fn spawn_event_handler(
        mut frame_rx: mpsc::Receiver<String>,
        event_tx: mpsc::Sender<MemorySyncEvent>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(frame) = frame_rx.recv().await {
                if let Ok(msg) = Self::parse_ws_frame(&frame) {
                    if let Ok(event) = Self::extract_sync_event(&msg) {
                        let _ = event_tx.send(event).await;
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = XavierClientConfig::default();
        assert_eq!(config.api_base_url, "http://127.0.0.1:8080");
        assert_eq!(config.ws_url, "ws://127.0.0.1:8080/ws");
        assert!(config.auth_token.is_none());
        assert_eq!(config.timeout_ms, 5000);
    }

    #[test]
    fn test_memory_entry_builder() {
        let entry = MemoryEntry::new("m1", "Test Memory Content")
            .with_tag("cognitive")
            .with_tag("swal")
            .with_metadata("source", "user_prompt");

        assert_eq!(entry.id, "m1");
        assert_eq!(entry.content, "Test Memory Content");
        assert_eq!(entry.tags, vec!["cognitive", "swal"]);
        assert_eq!(entry.metadata.get("source").map(|s| s.as_str()), Some("user_prompt"));
    }

    #[test]
    fn test_memory_query_builder() {
        let q = MemoryQuery::new("rust concurrency")
            .with_limit(20)
            .with_min_score(0.85)
            .with_tag("tech");

        assert_eq!(q.query, "rust concurrency");
        assert_eq!(q.limit, 20);
        assert_eq!(q.min_score, Some(0.85));
        assert_eq!(q.tags, vec!["tech"]);
    }

    #[test]
    fn test_client_creation_invalid_url() {
        let config = XavierClientConfig {
            api_base_url: "".to_string(),
            ..Default::default()
        };
        let client_res = XavierClient::new(config);
        assert!(client_res.is_err());
        if let Err(XavierClientError::ConfigError(msg)) = client_res {
            assert!(msg.contains("API base URL cannot be empty"));
        } else {
            panic!("Expected ConfigError");
        }
    }

    #[test]
    fn test_ws_message_encode_decode() {
        let sync_event = MemorySyncEvent {
            event_type: SyncEventType::Created,
            entry_id: "m_123".to_string(),
            payload: Some(MemoryEntry::new("m_123", "Indexed file item")),
            timestamp: 1700000000,
        };

        let msg = MemorySyncMessage {
            action: SyncAction::Event,
            payload: Some(serde_json::to_value(&sync_event).unwrap()),
        };

        let encoded = XavierClient::encode_ws_message(&msg).unwrap();
        let parsed_msg = XavierClient::parse_ws_frame(&encoded).unwrap();
        assert_eq!(parsed_msg.action, SyncAction::Event);

        let extracted_event = XavierClient::extract_sync_event(&parsed_msg).unwrap();
        assert_eq!(extracted_event, sync_event);
    }

    #[test]
    fn test_ws_message_non_event_action() {
        let msg = MemorySyncMessage {
            action: SyncAction::Ping,
            payload: None,
        };
        let res = XavierClient::extract_sync_event(&msg);
        assert!(res.is_err());
        if let Err(XavierClientError::WebSocketError(err)) = res {
            assert!(err.contains("Message action is not SyncAction::Event"));
        } else {
            panic!("Expected WebSocketError");
        }
    }

    #[tokio::test]
    async fn test_spawn_event_handler_stream() {
        let (frame_tx, frame_rx) = mpsc::channel(10);
        let (event_tx, mut event_rx) = mpsc::channel(10);

        let handle = XavierClient::spawn_event_handler(frame_rx, event_tx);

        let event = MemorySyncEvent {
            event_type: SyncEventType::Updated,
            entry_id: "m_999".to_string(),
            payload: None,
            timestamp: 1700000050,
        };

        let msg = MemorySyncMessage {
            action: SyncAction::Event,
            payload: Some(serde_json::to_value(&event).unwrap()),
        };

        let frame_str = XavierClient::encode_ws_message(&msg).unwrap();
        frame_tx.send(frame_str).await.unwrap();
        drop(frame_tx);

        let received = event_rx.recv().await;
        assert_eq!(received, Some(event));

        handle.await.unwrap();
    }

    #[test]
    fn test_error_formatting() {
        let err1 = XavierClientError::HttpError("connection refused".to_string());
        assert_eq!(err1.to_string(), "HTTP request failed: connection refused");

        let err2 = XavierClientError::ServerError {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(err2.to_string(), "Server returned status 404: Not Found");
    }
}
