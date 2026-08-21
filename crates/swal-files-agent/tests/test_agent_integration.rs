#![deny(unsafe_code)]

use std::net::SocketAddr;
use swal_files_agent::client::{
    MemoryEntry, MemoryQuery, MemorySearchResult, XavierClient, XavierClientConfig,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Mock HTTP server simulating Xavier Cognitive Memory GraphRAG service.
pub struct AgentMockServer {
    addr: SocketAddr,
    _shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl AgentMockServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind mock server listener");
        let addr = listener.local_addr().expect("Failed to get local address");
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept_res = listener.accept() => {
                        if let Ok((mut socket, _)) = accept_res {
                            tokio::spawn(async move {
                                let mut buf = [0u8; 4096];
                                let n = match socket.read(&mut buf).await {
                                    Ok(n) if n > 0 => n,
                                    _ => return,
                                };
                                let request_str = String::from_utf8_lossy(&buf[..n]);
                                let response = handle_http_request(&request_str);
                                let _ = socket.write_all(response.as_bytes()).await;
                            });
                        }
                    }
                }
            }
        });

        Self {
            addr,
            _shutdown_tx: shutdown_tx,
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

fn handle_http_request(req: &str) -> String {
    let lines: Vec<&str> = req.split("\r\n").collect();
    if lines.is_empty() {
        return "HTTP/1.1 400 Bad Request\r\n\r\n".to_string();
    }
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 {
        return "HTTP/1.1 400 Bad Request\r\n\r\n".to_string();
    }
    let method = parts[0];
    let path = parts[1];
    let body = req.split("\r\n\r\n").nth(1).unwrap_or("");

    match (method, path) {
        ("GET", "/health") => json_response(200, &serde_json::json!({"status": "ok"})),
        ("POST", "/api/v1/memories/query") => {
            let q_str = serde_json::from_str::<MemoryQuery>(body)
                .map(|q| q.query)
                .unwrap_or_default();
            let entry = MemoryEntry::new(
                "graphrag_node_1",
                format!("GraphRAG cognitive node for query: {}", q_str),
            )
            .with_tag("graphrag")
            .with_metadata("source", "xavier_agent");

            let result = MemorySearchResult {
                entries: vec![entry],
                total_matches: 1,
                latency_ms: 8,
            };
            json_response(200, &result)
        }
        ("POST", "/api/v1/memories") => {
            if let Ok(entry) = serde_json::from_str::<MemoryEntry>(body) {
                json_response(200, &entry)
            } else {
                json_response(400, &serde_json::json!({"error": "invalid payload"}))
            }
        }
        ("POST", "/api/v1/memories/batch") => {
            if let Ok(entries) = serde_json::from_str::<Vec<MemoryEntry>>(body) {
                json_response(200, &serde_json::json!({"synced_count": entries.len()}))
            } else {
                json_response(400, &serde_json::json!({"error": "invalid batch"}))
            }
        }
        ("DELETE", p) if p.starts_with("/api/v1/memories/") => {
            json_response(200, &serde_json::json!({"deleted": true}))
        }
        _ => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string(),
    }
}

fn json_response<T: serde::Serialize>(status: u16, data: &T) -> String {
    let body = serde_json::to_string(data).unwrap_or_default();
    format!(
        "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    )
}

#[tokio::test]
async fn test_xavier_semantic_query_flow() {
    let mock_server = AgentMockServer::start().await;
    let config = XavierClientConfig {
        api_base_url: mock_server.url(),
        ws_url: format!("{}/ws", mock_server.url().replace("http", "ws")),
        auth_token: Some("test_token_123".to_string()),
        timeout_ms: 3000,
    };

    let client = XavierClient::new(config).expect("Client creation failed");

    // 1. Health check
    let is_healthy = client.check_health().await.unwrap();
    assert!(is_healthy);

    // 2. Query flow simulating GraphRAG semantic search
    let query = MemoryQuery::new("rust concurrency patterns").with_limit(5);
    let search_res = client.query(&query).await.unwrap();
    assert_eq!(search_res.total_matches, 1);
    assert_eq!(search_res.entries.len(), 1);
    assert!(search_res.entries[0]
        .content
        .contains("rust concurrency patterns"));

    // 3. Store new memory
    let entry = MemoryEntry::new("mem_001", "Vector embedding node")
        .with_tag("vector")
        .with_metadata("author", "xavier");
    let stored = client.store(&entry).await.unwrap();
    assert_eq!(stored.id, "mem_001");

    // 4. Batch sync
    let entries = vec![
        MemoryEntry::new("m1", "Content 1"),
        MemoryEntry::new("m2", "Content 2"),
    ];
    let synced = client.batch_sync(&entries).await.unwrap();
    assert_eq!(synced, 2);

    // 5. Delete memory
    let deleted = client.delete("mem_001").await.unwrap();
    assert!(deleted);
}

#[tokio::test]
async fn test_agent_mock_server_health_check() {
    let mock_server = AgentMockServer::start().await;
    let config = XavierClientConfig {
        api_base_url: mock_server.url(),
        ..Default::default()
    };
    let client = XavierClient::new(config).unwrap();
    assert!(client.check_health().await.unwrap());
}

#[tokio::test]
async fn test_agent_error_handling() {
    let mock_server = AgentMockServer::start().await;
    let config = XavierClientConfig {
        api_base_url: mock_server.url(),
        ..Default::default()
    };
    let client = XavierClient::new(config).unwrap();

    let res = client.query(&MemoryQuery::new("")).await;
    assert!(res.is_ok());
}
