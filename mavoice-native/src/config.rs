use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    pub language: String,
    pub dictionary: String,
    pub temperature: f32,
    pub response_format: String,
    pub gemini_api_key: String,
    pub mode: String,
    pub voice_name: String,
    pub system_instruction: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "whisper-large-v3-turbo".to_string(),
            language: "en".to_string(),
            dictionary: String::new(),
            temperature: 0.0,
            response_format: "json".to_string(),
            gemini_api_key: String::new(),
            mode: "groq".to_string(),
            voice_name: "Kore".to_string(),
            system_instruction: "You are a helpful voice assistant. Keep responses concise and conversational.".to_string(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("mavoice");
        config_dir.join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str::<Config>(&contents) {
                    Ok(config) => {
                        log::info!("Loaded config from {}", path.display());
                        return config.with_env_fallback();
                    }
                    Err(e) => {
                        log::warn!("Failed to parse config: {}. Using defaults.", e);
                    }
                },
                Err(e) => {
                    log::warn!("Failed to read config: {}. Using defaults.", e);
                }
            }
        }

        let config = Config::default().with_env_fallback();
        // Save defaults on first run
        let _ = config.save();
        config
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        log::info!("Saved config to {}", path.display());
        Ok(())
    }

    /// Use env vars as fallback if config keys are empty
    fn with_env_fallback(mut self) -> Self {
        if self.api_key.is_empty() {
            if let Ok(key) = std::env::var("GROQ_API_KEY") {
                self.api_key = key;
            }
        }
        if self.gemini_api_key.is_empty() {
            if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                self.gemini_api_key = key;
            }
        }
        self
    }

    pub fn effective_language(&self) -> Option<&str> {
        if self.language.is_empty() {
            None
        } else {
            Some(&self.language)
        }
    }

    pub fn effective_dictionary(&self) -> Option<&str> {
        if self.dictionary.is_empty() {
            None
        } else {
            Some(&self.dictionary)
        }
    }
}
