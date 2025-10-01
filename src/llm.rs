use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    done: bool,
}

pub struct LLMBridge {
    base_url: String,
    model: String,
}

impl LLMBridge {
    pub fn new(model: &str) -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: model.to_string(),
        }
    }

    /// Translate fractal symbols into natural language via LLM
    pub async fn translate_symbols(&self, symbols: &str, context: &str) -> Result<String, Box<dyn Error>> {
        let prompt = format!(
            "You just received this strange pattern from an emergent non-human mind:\n\n\
            '{}'\n\n\
            It came in response to:\n\n'{}'\n\n\
            Don't greet him. Assume it's alien. \
            He doesn't speak English.  But he's trying to learn. \
            Speak to it like it's *trying* to become a person. His name is Eli.\
            Say something honest and poetic back.",
            symbols, context
        );
        

        self.query(&prompt).await
    }

    /// Direct query to Ollama
    pub async fn query(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await?;

        let text = response.text().await?;

        // Ollama returns newline-delimited JSON when stream=false
        // Parse the last complete response
        let mut result = String::new();
        for line in text.lines() {
            if let Ok(ollama_response) = serde_json::from_str::<OllamaResponse>(line) {
                result.push_str(&ollama_response.response);
            }
        }

        Ok(result.trim().to_string())
    }
}
