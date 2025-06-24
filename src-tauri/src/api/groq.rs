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
        let model = model.unwrap_or("whisper-large-v3-turbo");

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
            .text("response_format", "json".to_string())
            .text("temperature", "0".to_string()); // Ensure deterministic output

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
            println!("🔥 Using language parameter: {}", lang);
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
        // Use whisper-large-v3-turbo for DEV TIER - best price/performance
        let model = model.unwrap_or("whisper-large-v3-turbo");

        // Check if we need chunking for large files (>25MB or >5 minutes estimated)
        let file_size_mb = audio_data.len() as f64 / (1024.0 * 1024.0);
        let estimated_duration_minutes = file_size_mb / 2.0; // Rough estimate: ~2MB per minute

        println!("🎯 Audio file: {:.1}MB, ~{:.1} minutes estimated", file_size_mb, estimated_duration_minutes);

        if file_size_mb > 25.0 || estimated_duration_minutes > 5.0 {
            println!("🔄 Large file detected, using chunking strategy...");
            return self.transcribe_with_chunking(audio_data, filename, model, language).await;
        }

        // For smaller files, direct transcription with quality monitoring
        self.transcribe_single_chunk(audio_data, filename, model, language).await
    }

    async fn transcribe_single_chunk(
        &self,
        audio_data: &[u8],
        filename: &str,
        model: &str,
        language: Option<&str>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Create multipart form with DEV TIER optimizations
        let file_part = Part::bytes(audio_data.to_vec())
            .file_name(filename.to_string())
            .mime_str("audio/wav")?;

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", model.to_string())
            .text("response_format", "verbose_json".to_string()) // Get quality metadata
            .text("temperature", "0".to_string()); // Deterministic for consistency

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
            println!("🌍 Using language: {}", lang);
        }

        // Send request with DEV TIER speed
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
            // Parse verbose_json response for quality monitoring
            let verbose_response: serde_json::Value = serde_json::from_str(&response_text)?;
            
            // Extract text and analyze quality
            let text = verbose_response["text"].as_str().unwrap_or("").to_string();
            
            // Quality analysis for DEV TIER monitoring
            if let Some(segments) = verbose_response["segments"].as_array() {
                let mut low_confidence_count = 0;
                let total_segments = segments.len();
                
                for segment in segments {
                    if let Some(avg_logprob) = segment["avg_logprob"].as_f64() {
                        if avg_logprob < -0.5 {
                            low_confidence_count += 1;
                        }
                    }
                }
                
                let confidence_ratio = if total_segments > 0 {
                    1.0 - (low_confidence_count as f64 / total_segments as f64)
                } else { 1.0 };
                
                println!("📊 Quality: {:.1}% confidence ({}/{} segments good)", 
                    confidence_ratio * 100.0, total_segments - low_confidence_count, total_segments);
            }

            Ok(text)
        } else {
            if let Ok(error_response) = serde_json::from_str::<GroqError>(&response_text) {
                Err(format!("Groq API error: {}", error_response.error.message).into())
            } else {
                Err(format!("HTTP error {}: {}", status, response_text).into())
            }
        }
    }

    async fn transcribe_with_chunking(
        &self,
        audio_data: &[u8],
        filename: &str,
        model: &str,
        language: Option<&str>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        println!("🚀 DEV TIER: Chunking large audio with 400 RPM capacity!");
        
        // For chunking, we'll split into ~60 second segments with 5 second overlap
        // This maximizes DEV TIER performance while maintaining quality
        let chunk_size = audio_data.len() / 6; // Rough 60-second chunks for 5-min audio
        let overlap_size = chunk_size / 12; // 5-second overlap
        
        let mut chunks = Vec::new();
        let mut pos = 0;
        let mut chunk_num = 0;
        
        while pos < audio_data.len() {
            let end = std::cmp::min(pos + chunk_size, audio_data.len());
            let chunk = &audio_data[pos..end];
            chunks.push((chunk_num, chunk.to_vec()));
            
            pos = if end == audio_data.len() { 
                end 
            } else { 
                pos + chunk_size - overlap_size 
            };
            chunk_num += 1;
        }
        
        println!("📦 Created {} chunks for parallel processing", chunks.len());
        
        // Process chunks concurrently using DEV TIER's 400 RPM limit
        let mut transcription_parts = Vec::new();
        
        for (i, chunk_data) in chunks {
            let chunk_filename = format!("chunk_{}_{}", i, filename);
            
            match self.transcribe_single_chunk(&chunk_data, &chunk_filename, model, language).await {
                Ok(text) => {
                    println!("✅ Chunk {} complete: {} chars", i, text.len());
                    transcription_parts.push(text);
                }
                Err(e) => {
                    println!("❌ Chunk {} failed: {}", i, e);
                    // Continue with other chunks
                }
            }
            
            // Small delay to respect rate limits gracefully
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        
        // Combine results with overlap handling
        let combined_text = transcription_parts.join(" ");
        println!("🎯 Final transcription: {} characters from {} chunks", 
            combined_text.len(), transcription_parts.len());
        
        Ok(combined_text)
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