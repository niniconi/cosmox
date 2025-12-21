use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize)]
struct OllamaPayload {
  pub model: String,
  pub prompt: String,
  pub stream: bool,
}

pub async fn ollama(api: &str, model: &str, prompt: &str) -> Result<String> {
  let payload = OllamaPayload {
    model: model.to_string(),
    prompt: prompt.to_string(),
    stream: false,
  };
  let result = reqwest::Client::new()
    .post(api)
    .body(serde_json::to_string(&payload)?)
    .send()
    .await?;
  let response: Value = result.json().await.unwrap();
  Ok(response.get("response").unwrap().to_string())
}

pub async fn call_llm(prompt: &str) -> Result<String> {
  ollama("http://localhost:11434/api/generate", "gemma3:270m", prompt).await.inspect(|k| println!("{k}"))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_ollama() {
    ollama("http://localhost:11434/api/generate", "gemma3:270m", "").await;
  }
}
