//! Cliente mínimo de Ollama (/api/chat, stream=true, NDJSON).
//!
//! Cada línea del body es un JSON: {"message": {"role": "assistant",
//! "content": "...", "thinking": "..."}, "done": false}.

use futures::Stream;
use serde::Deserialize;
use std::pin::Pin;

#[derive(Debug, Clone, serde::Serialize)]
pub struct OllamaMessage {
    pub role: String,      // "system" | "user" | "assistant"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum OllamaStreamItem {
    Thinking(String),
    Chunk(String),
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaMsgInner>,
    #[allow(dead_code)]
    done: Option<bool>,
    #[allow(dead_code)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaMsgInner {
    content: Option<String>,
    thinking: Option<String>,
}

/// Error de Ollama (no corre, timeout, respuesta inválida...).
#[derive(Debug)]
pub struct OllamaError(pub String);

impl std::fmt::Display for OllamaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for OllamaError {}

/// Versión legacy de compatibilidad (solo chunks de texto, think=false).
pub async fn chat_stream_ollama(
    ollama_url: &str,
    model: &str,
    system_prompt: &str,
    messages: Vec<OllamaMessage>,
) -> Result<Pin<Box<dyn Stream<Item = Result<String, OllamaError>> + Send>>, OllamaError> {
    let mut inner = chat_stream_ollama_opts(
        ollama_url,
        model,
        system_prompt,
        messages,
        false,
        "rapido",
        "muy_bajo",
    )
    .await?;

    let text_stream = async_stream::try_stream! {
        use futures::StreamExt;
        while let Some(item) = inner.next().await {
            if let OllamaStreamItem::Chunk(txt) = item? {
                yield txt;
            }
        }
    };

    Ok(Box::pin(text_stream))
}

/// Llama a Ollama /api/chat con stream=true, control de modo, pensamiento y nivel de razonamiento.
pub async fn chat_stream_ollama_opts(
    ollama_url: &str,
    model: &str,
    system_prompt: &str,
    messages: Vec<OllamaMessage>,
    think: bool,
    mode: &str,
    thinking_level: &str,
) -> Result<Pin<Box<dyn Stream<Item = Result<OllamaStreamItem, OllamaError>> + Send>>, OllamaError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| OllamaError(format!("client: {}", e)))?;

    let mut full_messages = vec![OllamaMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
        images: None,
    }];
    full_messages.extend(messages);

    let temperature: f64 = if mode == "instantaneo" { 0.5 } else { 0.7 };
    let num_predict: i64 = if !think {
        if mode == "instantaneo" { 384 } else { 768 }
    } else {
        match thinking_level {
            "muy_bajo" => 512,
            "bajo" => 768,
            "medio" => 1152,
            "alto" => 1792,
            "muy_alto" => 2560,
            "extremo" => 4096,
            _ => 1024,
        }
    };

    let body = serde_json::json!({
        "model": model,
        "messages": full_messages,
        "stream": true,
        "think": think,
        "options": {
            "temperature": temperature,
            "num_predict": num_predict,
        }
    });

    let trimmed = ollama_url.trim().trim_end_matches('/');
    let chat_endpoint = if trimmed.ends_with("/api/chat") {
        trimmed.to_string()
    } else {
        format!("{}/api/chat", trimmed)
    };

    let response = client
        .post(&chat_endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|e| OllamaError(format!("connect: {}", e)))?;

    if !response.status().is_success() {
        return Err(OllamaError(format!("status: {}", response.status())));
    }

    let byte_stream = response.bytes_stream();

    let out_stream = async_stream::try_stream! {
        use futures::StreamExt;
        let mut buf = String::new();
        let mut stream = byte_stream;
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| OllamaError(format!("read: {}", e)))?;
            buf.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(newline_pos) = buf.find('\n') {
                let line: String = buf.drain(..=newline_pos).collect();
                let line = line.trim();
                if line.is_empty() { continue; }

                if let Ok(parsed) = serde_json::from_str::<OllamaChatResponse>(line) {
                    if let Some(err) = parsed.error {
                        Err(OllamaError(format!("ollama: {}", err)))?;
                    }
                    if let Some(msg) = parsed.message {
                        if let Some(thinking) = msg.thinking {
                            if !thinking.is_empty() {
                                yield OllamaStreamItem::Thinking(thinking);
                            }
                        }
                        if let Some(content) = msg.content {
                            if !content.is_empty() {
                                yield OllamaStreamItem::Chunk(content);
                            }
                        }
                    }
                    if parsed.done == Some(true) {
                        return;
                    }
                }
            }
        }
    };

    Ok(Box::pin(out_stream))
}
