use reqwest::multipart;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GroqResponse {
    pub text: String,
}

pub struct GroqClient {
    api_key: String,
    client: reqwest::Client,
}

impl GroqClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn transcribe_audio(&self, audio_data: Vec<u8>) -> Result<String, String> {
        println!("📤 Sending {:.2}KB WAV to Groq", audio_data.len() as f32 / 1024.0);
        
        // Create multipart form
        let part = multipart::Part::bytes(audio_data)
            .file_name("recording.wav")
            .mime_str("audio/wav")
            .map_err(|e| format!("Failed to create multipart: {}", e))?;
        
        let form = multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-large-v3")
            .text("response_format", "json")
            .text("language", "en")
            .text("temperature", "0.0");
        
        // Send request to Groq API
        let response = self.client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }
        
        // Parse response
        let groq_response: GroqResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        println!("🎯 === TRANSCRIPTION RESULT ===");
        println!("📝 Text: {}", groq_response.text);
        println!("📊 Length: {} characters", groq_response.text.len());
        println!("🎯 ===========================");
        
        Ok(groq_response.text)
    }
}

// Helper function to get API key from environment
pub fn get_groq_api_key() -> Result<String, String> {
    std::env::var("GROQ_API_KEY")
        .or_else(|_| std::env::var("VITE_GROQ_API_KEY"))
        .map_err(|_| "GROQ_API_KEY environment variable not set".to_string())
}