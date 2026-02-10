use base64::prelude::*;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Commands sent from the main thread to the WebSocket write task.
enum ClientCommand {
    SendAudio(Vec<u8>),
    SendText(String),
    ActivityStart,
    ActivityEnd,
    Close,
}

/// Events sent from the WebSocket read task back to the winit event loop.
#[derive(Debug)]
pub enum GeminiEvent {
    Ready,
    Audio(Vec<u8>),
    Text(String),
    Interrupted,
    TurnComplete,
    Error(String),
    Closed(String),
}

/// Async Gemini Live WebSocket client.
///
/// Spawns two tokio tasks (read + write) and communicates via channels.
/// The `send_*` methods are non-blocking and safe to call from any thread.
pub struct GeminiLiveClient {
    cmd_tx: mpsc::UnboundedSender<ClientCommand>,
    open: Arc<AtomicBool>,
    send_count: Arc<std::sync::atomic::AtomicU64>,
}

impl GeminiLiveClient {
    /// Connect to Gemini Live and start the read/write tasks.
    ///
    /// `event_tx` is a callback that delivers parsed server events back to the caller.
    /// In practice this is wired to `EventLoopProxy::send_event()`.
    pub async fn connect(
        api_key: &str,
        voice_name: &str,
        system_instruction: &str,
        event_tx: mpsc::UnboundedSender<GeminiEvent>,
    ) -> Result<Self, String> {
        let url = format!(
            "wss://generativelanguage.googleapis.com/ws/\
             google.ai.generativelanguage.v1beta.GenerativeService.\
             BidiGenerateContent?key={}",
            api_key
        );

        let mut ws_config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        ws_config.max_message_size = Some(64 * 1024 * 1024);
        ws_config.max_frame_size = Some(16 * 1024 * 1024);

        let (ws_stream, _response) =
            tokio_tungstenite::connect_async_with_config(&url, Some(ws_config), true)
                .await
                .map_err(|e| format!("WebSocket connect failed: {}", e))?;

        log::info!("[Gemini] WebSocket connected");

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Send setup message — matches the reference Node.js implementation exactly
        let setup = json!({
            "setup": {
                "model": format!("models/{}", "gemini-2.5-flash-native-audio-preview-12-2025"),
                "generationConfig": {
                    "responseModalities": ["AUDIO"],
                    "speechConfig": {
                        "voiceConfig": {
                            "prebuiltVoiceConfig": {
                                "voiceName": voice_name
                            }
                        }
                    }
                },
                "systemInstruction": {
                    "parts": [{ "text": system_instruction }]
                }
            }
        });
        log::info!("[Gemini] Setup JSON: {}", serde_json::to_string_pretty(&setup).unwrap_or_default());

        ws_write
            .send(Message::Text(setup.to_string().into()))
            .await
            .map_err(|e| format!("Failed to send setup: {}", e))?;

        log::info!("[Gemini] Setup message sent");

        let open = Arc::new(AtomicBool::new(true));
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ClientCommand>();

        // Write task — receives commands and sends them as JSON over WebSocket
        let write_open = open.clone();
        let write_handle = tokio::spawn(async move {
            log::info!("[Gemini] Write task started, waiting for commands...");
            while let Some(cmd) = cmd_rx.recv().await {
                if !write_open.load(Ordering::Relaxed) {
                    break;
                }

                let msg = match cmd {
                    ClientCommand::SendAudio(pcm_bytes) => {
                        let encoded = BASE64_STANDARD.encode(&pcm_bytes);
                        json!({
                            "realtimeInput": {
                                "audio": {
                                    "mimeType": "audio/pcm;rate=16000",
                                    "data": encoded
                                }
                            }
                        })
                    }
                    ClientCommand::SendText(text) => {
                        json!({
                            "clientContent": {
                                "turns": [{ "role": "user", "parts": [{ "text": text }] }],
                                "turnComplete": true
                            }
                        })
                    }
                    ClientCommand::ActivityStart => {
                        json!({ "realtimeInput": { "activityStart": {} } })
                    }
                    ClientCommand::ActivityEnd => {
                        json!({ "realtimeInput": { "activityEnd": {} } })
                    }
                    ClientCommand::Close => {
                        write_open.store(false, Ordering::Relaxed);
                        let _ = ws_write.close().await;
                        break;
                    }
                };

                if let Err(e) = ws_write.send(Message::Text(msg.to_string().into())).await {
                    log::error!("[Gemini] Write error: {}", e);
                    write_open.store(false, Ordering::Relaxed);
                    break;
                }
            }

            log::info!("[Gemini] Write task exiting");
        });

        // Read task — parses server messages and sends GeminiEvents
        let read_open = open.clone();
        let read_event_tx = event_tx.clone();
        log::info!("[Gemini] About to spawn read task...");
        let read_handle = tokio::spawn(async move {
            log::info!("[Gemini] Read task started, waiting for server messages...");
            while let Some(msg_result) = ws_read.next().await {
                log::info!("[Gemini] Read task received a message");
                match msg_result {
                    Ok(Message::Text(text)) => {
                        let preview: String = text.chars().take(200).collect();
                        log::debug!("[Gemini] Text msg: {}", preview);
                        Self::parse_server_message(&text, &read_event_tx);
                    }
                    Ok(Message::Binary(data)) => {
                        // Gemini sends JSON as binary frames
                        match std::str::from_utf8(&data) {
                            Ok(text) => {
                                let preview: String = text.chars().take(200).collect();
                                log::debug!("[Gemini] Binary msg (as text): {}", preview);
                                Self::parse_server_message(text, &read_event_tx);
                            }
                            Err(_) => {
                                log::warn!("[Gemini] Received non-UTF8 binary frame ({} bytes)", data.len());
                            }
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        let reason = frame
                            .map(|f| format!("code={}, reason={}", f.code, f.reason))
                            .unwrap_or_else(|| "no frame".to_string());
                        log::info!("[Gemini] WebSocket closed: {}", reason);
                        read_open.store(false, Ordering::Relaxed);
                        let _ = read_event_tx.send(GeminiEvent::Closed(reason));
                        break;
                    }
                    Err(e) => {
                        log::error!("[Gemini] Read error: {}", e);
                        read_open.store(false, Ordering::Relaxed);
                        let _ = read_event_tx.send(GeminiEvent::Error(e.to_string()));
                        break;
                    }
                    _ => {} // Ping/Pong handled by tungstenite
                }
            }

            read_open.store(false, Ordering::Relaxed);
            log::info!("[Gemini] Read task exiting");
        });

        Ok(Self {
            cmd_tx,
            open,
            send_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    /// Parse a server JSON message and emit the appropriate GeminiEvent.
    fn parse_server_message(text: &str, tx: &mpsc::UnboundedSender<GeminiEvent>) {
        let msg: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("[Gemini] Malformed server message: {}", e);
                return;
            }
        };

        // Log all server message keys for debugging
        if let Some(obj) = msg.as_object() {
            let keys: Vec<&String> = obj.keys().collect();
            log::debug!("[Gemini] Server message keys: {:?}", keys);
        }

        // setupComplete
        if msg.get("setupComplete").is_some() {
            log::info!("[Gemini] Setup complete");
            let _ = tx.send(GeminiEvent::Ready);
            return;
        }

        // goAway — server will disconnect soon
        if let Some(go_away) = msg.get("goAway") {
            let time_left = go_away
                .get("timeLeft")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            log::warn!("[Gemini] goAway received, timeLeft={}", time_left);
            return;
        }

        // toolCall — not handling tools in maVoice (yet)
        if msg.get("toolCall").is_some() {
            log::debug!("[Gemini] Tool call received (ignored in maVoice)");
            return;
        }

        // serverContent
        if let Some(content) = msg.get("serverContent") {
            // Interruption (barge-in)
            if content.get("interrupted").and_then(|v| v.as_bool()) == Some(true) {
                log::info!("[Gemini] Interrupted (barge-in)");
                let _ = tx.send(GeminiEvent::Interrupted);
                return;
            }

            // Turn complete
            if content.get("turnComplete").and_then(|v| v.as_bool()) == Some(true) {
                let _ = tx.send(GeminiEvent::TurnComplete);
                return;
            }

            // Model turn parts — audio and text
            if let Some(parts) = content
                .get("modelTurn")
                .and_then(|mt| mt.get("parts"))
                .and_then(|p| p.as_array())
            {
                for part in parts {
                    // Audio data (base64 PCM 24kHz s16le)
                    if let Some(b64_data) = part
                        .get("inlineData")
                        .and_then(|d| d.get("data"))
                        .and_then(|d| d.as_str())
                    {
                        match BASE64_STANDARD.decode(b64_data) {
                            Ok(pcm_bytes) => {
                                let _ = tx.send(GeminiEvent::Audio(pcm_bytes));
                            }
                            Err(e) => {
                                log::warn!("[Gemini] Base64 decode error: {}", e);
                            }
                        }
                    }

                    // Text transcription
                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                        let _ = tx.send(GeminiEvent::Text(text.to_string()));
                    }
                }
            }

            // Input/output transcription (separate from modelTurn)
            if let Some(text) = content
                .get("outputTranscription")
                .and_then(|t| t.get("text"))
                .and_then(|t| t.as_str())
            {
                log::debug!("[Gemini] Output transcription: {}", text);
            }
            if let Some(text) = content
                .get("inputTranscription")
                .and_then(|t| t.get("text"))
                .and_then(|t| t.as_str())
            {
                log::debug!("[Gemini] Input transcription: {}", text);
            }
        }
    }

    /// Send raw PCM audio (16kHz mono s16le) to Gemini.
    pub fn send_audio(&self, pcm_s16le_16khz: &[u8]) {
        if self.open.load(Ordering::Relaxed) {
            let count = self.send_count.fetch_add(1, Ordering::Relaxed);
            if count == 0 || count % 50 == 0 {
                log::info!("[Gemini] Audio chunks sent: {} ({}KB total)", count + 1,
                    (count + 1) * pcm_s16le_16khz.len() as u64 / 1024);
            }
            let _ = self.cmd_tx.send(ClientCommand::SendAudio(pcm_s16le_16khz.to_vec()));
        }
    }

    /// Signal that the user started speaking (manual VAD).
    pub fn send_activity_start(&self) {
        if self.open.load(Ordering::Relaxed) {
            let _ = self.cmd_tx.send(ClientCommand::ActivityStart);
        }
    }

    /// Signal that the user stopped speaking (manual VAD).
    pub fn send_activity_end(&self) {
        if self.open.load(Ordering::Relaxed) {
            let _ = self.cmd_tx.send(ClientCommand::ActivityEnd);
        }
    }

    /// Send a text message to Gemini (it will respond with audio).
    pub fn send_text(&self, text: &str) {
        if self.open.load(Ordering::Relaxed) {
            let _ = self.cmd_tx.send(ClientCommand::SendText(text.to_string()));
        }
    }

    /// Close the WebSocket connection.
    pub fn close(&self) {
        self.open.store(false, Ordering::Relaxed);
        let _ = self.cmd_tx.send(ClientCommand::Close);
    }

    /// Check if the connection is still open.
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Relaxed)
    }
}

impl Drop for GeminiLiveClient {
    fn drop(&mut self) {
        self.close();
    }
}
