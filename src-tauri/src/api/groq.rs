use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqTranscriptionRequest {
    file: String,
    model: String,
    language: Option<String>,
    prompt: Option<String>,
    response_format: Option<String>,
    temperature: Option<f32>,
}

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

    pub async fn transcribe_audio_file(
        &self,
        file_path: &str,
        model: Option<&str>,
        language: Option<&str>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let model = model.unwrap_or("whisper-large-v3");

        // Read the audio file
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Create multipart form
        let file_part = Part::bytes(buffer)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", model.to_string())
            .text("response_format", "json".to_string());

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        // Send request
        let response = self
            .client
            .post(&format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        let response_text: String = response.text().await?;

        if status.is_success() {
            let transcription: GroqTranscriptionResponse = serde_json::from_str(&response_text)?;
            Ok(transcription.text)
        } else {
            // Try to parse as error response
            if let Ok(error_response) = serde_json::from_str::<GroqError>(&response_text) {
                Err(format!("Groq API error: {}", error_response.error.message).into())
            } else {
                Err(format!("HTTP error {}: {}", status, response_text).into())
            }
        }
    }

    pub async fn transcribe_audio_bytes(
        &self,
        audio_data: &[u8],
        filename: &str,
        model: Option<&str>,
        language: Option<&str>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let model = model.unwrap_or("whisper-large-v3");

        // Create multipart form
        let file_part = Part::bytes(audio_data.to_vec())
            .file_name(filename.to_string())
            .mime_str("audio/wav")?;

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", model.to_string())
            .text("response_format", "json".to_string());

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        // Send request
        let response = self
            .client
            .post(&format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        let response_text: String = response.text().await?;

        if status.is_success() {
            let transcription: GroqTranscriptionResponse = serde_json::from_str(&response_text)?;
            Ok(transcription.text)
        } else {
            if let Ok(error_response) = serde_json::from_str::<GroqError>(&response_text) {
                Err(format!("Groq API error: {}", error_response.error.message).into())
            } else {
                Err(format!("HTTP error {}: {}", status, response_text).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_groq_client_creation() {
        let client = GroqClient::new("test_api_key".to_string());
        assert_eq!(client.api_key, "test_api_key");
        assert_eq!(client.base_url, "https://api.groq.com/openai/v1");
    }
}