use serde_json::Value;

use std::{
    io::{Error, ErrorKind, Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
};

pub struct Ollama {
    pub version: String,
}

impl Ollama {
    // Ollama default port is 11434
    pub fn new() -> Result<Ollama, Box<dyn std::error::Error>> {
        let _ = Command::new("ollama")
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        let version = Self::version();
        Ok(Ollama { version })
    }

    pub fn version() -> String {
        if let Ok(mut stream) = TcpStream::connect("127.0.0.1:11434") {
            let request = "GET /api/version HTTP/1.1\r\n\
                       Host: localhost\r\n\
                       Connection: close\r\n\r\n";

            if stream.write_all(request.as_bytes()).is_err() {
                return "write error".to_string();
            }

            let mut response = String::new();
            if stream.read_to_string(&mut response).is_err() {
                return "read error".to_string();
            }

            if let Some(start) = response.find("\r\n\r\n") {
                let json_body = &response[start + 4..].trim();
                if let Ok(parsed) = serde_json::from_str::<Value>(json_body) {
                    if let Some(version) = parsed["version"].as_str() {
                        return version.to_string();
                    }
                }
            }

            response
                .lines()
                .find(|l| l.contains("version"))
                .unwrap_or("invalid response")
                .to_string()
        } else {
            "not connected".to_string()
        }
    }

    pub fn available_models() -> Result<Vec<String>, std::io::Error> {
        let mut stream = TcpStream::connect("127.0.0.1:11434")?;
        let request = "GET /api/tags HTTP/1.1\r\n\
                   Host: localhost\r\n\
                   Connection: close\r\n\r\n";

        stream.write_all(request.as_bytes())?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        let body_start = response
            .find("\r\n\r\n")
            .ok_or_else(|| Error::new(ErrorKind::Other, "Invalid HTTP response (missing body)"))?
            + 4;

        let json_body = &response[body_start..];
        let parsed: Value = serde_json::from_str(json_body)
            .map_err(|e| Error::new(ErrorKind::Other, format!("JSON parse error: {}", e)))?;

        let models_arr = parsed["models"]
            .as_array()
            .ok_or_else(|| Error::new(ErrorKind::Other, "Invalid models format in response"))?;

        let models = models_arr
            .iter()
            .filter_map(|m| {
                m.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(models)
    }

    //pub fn preload_model(&mut self, model: String) {
    //    Command::new("ollama")
    //        .arg("serve");
    //}

    pub fn prompt(&self, model: String, prompt: String) -> Result<String, std::io::Error> {
        let mut stream = TcpStream::connect("127.0.0.1:11434")?;

        let body = format!(
            r#"{{
            "model": "{}",
            "prompt": "{}",
            "stream": false
        }}"#,
            model, prompt
        );

        let request = format!(
            "POST /api/generate HTTP/1.1\r\n\
         Host: localhost\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {}",
            body.len(),
            body
        );

        stream.write_all(request.as_bytes())?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        let body_start = response.find("\r\n\r\n").ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid HTTP response (missing body)",
            )
        })? + 4;
        let json_body = &response[body_start..];

        let mut full_text = String::new();
        for line in json_body.lines() {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                if let Some(chunk) = value["response"].as_str() {
                    full_text.push_str(chunk);
                }
            }
        }

        if full_text.is_empty() {
            if let Ok(parsed) = serde_json::from_str::<Value>(json_body) {
                if let Some(text) = parsed["response"].as_str() {
                    full_text = text.to_string();
                }
            }
        }

        if full_text.is_empty() {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("No 'response' field in response: {}", json_body),
            ))
        } else {
            Ok(full_text)
        }
    }
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let ollama = Ollama::new().unwrap();
        println!("Ollama version: {}", ollama.version);
        assert!(!ollama.version.is_empty());
    }

    #[test]
    fn test_available_models() {
        let models = Ollama::available_models().unwrap();
        println!("Available models: {:?}", models);
        assert!(!models.is_empty());
    }

    #[test]
    fn test_prompt() {
        let ollama = Ollama::new().unwrap();
        let available_models = Ollama::available_models().unwrap();
        if available_models.is_empty() {
            panic!("No available models to test prompt");
        }

        let model = &available_models[0];
        println!("Testing prompt with model: {}", model);

        let reply = ollama
            .prompt(model.to_string(), "Hello, world!".to_string())
            .unwrap();
        println!("Ollama reply: {}", reply);
        assert!(!reply.is_empty());
    }
}
