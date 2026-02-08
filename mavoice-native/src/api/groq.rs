use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct GroqTranscriptionResponse {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqError {
    pub error: GroqErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqErrorDetail {
    pub message: String,
    pub r#type: String,
    pub code: Option<String>,
}

#[derive(Clone)]
pub struct GroqClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GroqClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.groq.com/openai/v1".to_string(),
        }
    }

    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub async fn transcribe_audio_bytes(
        &self,
        audio_data: &[u8],
        filename: &str,
        model: Option<&str>,
        language: Option<&str>,
        prompt: Option<&str>,
        response_format: Option<&str>,
        temperature: Option<f32>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let model = model.unwrap_or("whisper-large-v3-turbo");

        // Check if we need chunking for large files (>25MB or >5 minutes estimated)
        let file_size_mb = audio_data.len() as f64 / (1024.0 * 1024.0);
        let estimated_duration_minutes = file_size_mb / 2.0;

        log::info!(
            "Audio file: {:.1}MB, ~{:.1} minutes estimated",
            file_size_mb,
            estimated_duration_minutes
        );

        if file_size_mb > 25.0 || estimated_duration_minutes > 5.0 {
            log::info!("Large file detected, using chunking strategy...");
            return self
                .transcribe_with_chunking(
                    audio_data,
                    filename,
                    model,
                    language,
                    prompt,
                    response_format,
                    temperature,
                )
                .await;
        }

        self.transcribe_single_chunk(
            audio_data,
            filename,
            model,
            language,
            prompt,
            response_format,
            temperature,
        )
        .await
    }

    async fn transcribe_single_chunk(
        &self,
        audio_data: &[u8],
        filename: &str,
        model: &str,
        language: Option<&str>,
        prompt: Option<&str>,
        response_format: Option<&str>,
        temperature: Option<f32>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response_format = response_format.unwrap_or("json");
        let temperature = temperature.unwrap_or(0.0);

        let file_part = Part::bytes(audio_data.to_vec())
            .file_name(filename.to_string())
            .mime_str("audio/wav")?;

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", model.to_string())
            .text("response_format", response_format.to_string())
            .text("temperature", temperature.to_string());

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
            log::info!("Using language: {}", lang);
        }

        if let Some(p) = prompt {
            if !p.trim().is_empty() {
                form = form.text("prompt", p.to_string());
                log::info!("Using prompt/dictionary: {}", p);
            }
        }

        let response = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        let response_text: String = response.text().await?;

        if status.is_success() {
            let parsed: serde_json::Value = serde_json::from_str(&response_text)?;
            let text = parsed["text"].as_str().unwrap_or("").to_string();

            // Quality monitoring via segment confidence
            if let Some(segments) = parsed["segments"].as_array() {
                let total = segments.len();
                let low_conf = segments
                    .iter()
                    .filter(|s| s["avg_logprob"].as_f64().unwrap_or(0.0) < -0.5)
                    .count();
                let ratio = if total > 0 {
                    1.0 - (low_conf as f64 / total as f64)
                } else {
                    1.0
                };
                log::info!(
                    "Quality: {:.1}% confidence ({}/{} segments good)",
                    ratio * 100.0,
                    total - low_conf,
                    total
                );
            }

            Ok(text)
        } else if let Ok(error_response) = serde_json::from_str::<GroqError>(&response_text) {
            Err(format!("Groq API error: {}", error_response.error.message).into())
        } else {
            Err(format!("HTTP error {}: {}", status, response_text).into())
        }
    }

    async fn transcribe_with_chunking(
        &self,
        audio_data: &[u8],
        filename: &str,
        model: &str,
        language: Option<&str>,
        prompt: Option<&str>,
        response_format: Option<&str>,
        temperature: Option<f32>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Split into ~60 second segments with 5 second overlap
        let chunk_size = audio_data.len() / 6;
        let overlap_size = chunk_size / 12;

        let mut chunks = Vec::new();
        let mut pos = 0;
        let mut chunk_num = 0;

        while pos < audio_data.len() {
            let end = std::cmp::min(pos + chunk_size, audio_data.len());
            chunks.push((chunk_num, audio_data[pos..end].to_vec()));
            pos = if end == audio_data.len() {
                end
            } else {
                pos + chunk_size - overlap_size
            };
            chunk_num += 1;
        }

        log::info!("Created {} chunks for processing", chunks.len());

        let mut parts = Vec::new();
        for (i, chunk_data) in chunks {
            let chunk_filename = format!("chunk_{}_{}", i, filename);
            match self
                .transcribe_single_chunk(
                    &chunk_data,
                    &chunk_filename,
                    model,
                    language,
                    prompt,
                    response_format,
                    temperature,
                )
                .await
            {
                Ok(text) => {
                    log::info!("Chunk {} complete: {} chars", i, text.len());
                    parts.push(text);
                }
                Err(e) => {
                    log::error!("Chunk {} failed: {}", i, e);
                }
            }
            // Rate limit respect
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        let combined = parts.join(" ");
        log::info!(
            "Final transcription: {} chars from {} chunks",
            combined.len(),
            parts.len()
        );
        Ok(combined)
    }
}
