use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

const BROADCAST_CAPACITY: usize = 256;

/// Lightweight WebSocket broadcast server for the claudegram dashboard.
///
/// Accepts clients on `ws://127.0.0.1:{port}` and fans out JSON events
/// via a `tokio::sync::broadcast` channel. Clients are read-only — any
/// incoming messages are ignored.
pub struct DashboardBroadcaster {
    tx: broadcast::Sender<String>,
    running: Arc<AtomicBool>,
}

impl DashboardBroadcaster {
    /// Start the broadcast server in a background tokio task.
    pub async fn start(port: u16) -> Result<Self, String> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;

        log::info!("[Dashboard] Server listening on ws://127.0.0.1:{}", port);

        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let running = Arc::new(AtomicBool::new(true));

        let accept_tx = tx.clone();
        let accept_running = running.clone();

        tokio::spawn(async move {
            while accept_running.load(Ordering::Relaxed) {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        log::info!("[Dashboard] Client connected: {}", addr);
                        let client_rx = accept_tx.subscribe();
                        tokio::spawn(handle_client(stream, client_rx));
                    }
                    Err(e) => {
                        if accept_running.load(Ordering::Relaxed) {
                            log::warn!("[Dashboard] Accept error: {}", e);
                        }
                    }
                }
            }
        });

        Ok(Self { tx, running })
    }

    /// Broadcast a JSON event to all connected dashboard clients.
    ///
    /// Format: `{ "type": "<event_type>", "payload": { ... } }`
    ///
    /// Non-blocking. Silently drops if no clients are connected.
    pub fn broadcast(&self, event_type: &str, payload: Value) {
        let msg = json!({
            "type": event_type,
            "payload": payload,
        });
        // Ignore send errors (no active receivers)
        let _ = self.tx.send(msg.to_string());
    }

    /// Shut down the server.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Handle a single dashboard WebSocket client.
async fn handle_client(
    stream: tokio::net::TcpStream,
    mut rx: broadcast::Receiver<String>,
) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::warn!("[Dashboard] WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (mut ws_write, mut ws_read) = ws_stream.split();

    // Read task: drain incoming messages (we don't use them, but must consume
    // to keep the connection alive and handle close/ping frames).
    let mut read_task = tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {} // ignore everything else
            }
        }
    });

    // Write task: forward broadcasts to this client.
    let mut write_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(text) => {
                    if ws_write
                        .send(Message::Text(text.into()))
                        .await
                        .is_err()
                    {
                        break; // client disconnected
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("[Dashboard] Client lagged, dropped {} events", n);
                    // Continue — client will get next event
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Wait for either task to finish, then abort the other
    tokio::select! {
        _ = &mut read_task => { write_task.abort(); },
        _ = &mut write_task => { read_task.abort(); },
    }

    log::info!("[Dashboard] Client disconnected");
}
