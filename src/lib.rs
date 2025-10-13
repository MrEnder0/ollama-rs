use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;

use serde_json::Value;


pub struct Ollama;

impl Ollama {
    pub fn new() -> Result<Ollama, Box<dyn std::error::Error>> {
        Command::new("ollama")
            .arg("serve");

        Ok(Ollama)
    }

    //pub fn preload_model(&mut self, model: String) {
    //    Command::new("ollama")
    //        .arg("serve");
    //}

    pub fn prompt(&self, model: String, prompt: String) -> Result<String, std::io::Error> {
        // Default port 11434
        let mut stream = TcpStream::connect("127.0.0.1:11434")?;

        let body = format!(
            r#"{{
                "model": "{}",
                "prompt": "{}",
                "stream": false
            }}"#,
            model,
            prompt
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

        // Send request
        stream.write_all(request.as_bytes())?;

        // Read full response
        let mut response = String::new();
        let mut buffer = [0u8; 1024];
        loop {
            let n = stream.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            response.push_str(&String::from_utf8_lossy(&buffer[..n]));
        }

        // Extract JSON body from HTTP response
        let body_start = response.find("\r\n\r\n")
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Invalid HTTP response"))? + 4;
        let json_body = &response[body_start..];

        // Parse JSON
        let parsed: Value = serde_json::from_str(json_body)?;

        if let Some(text) = parsed["response"].as_str() {
            Ok(text.to_string())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No 'response' field in response",
        ))
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream() {
        let ollama = Ollama::new().unwrap();
        let reply = ollama.prompt("gemma3:12b".to_string(), "Hello, world!".to_string()).unwrap();
        println!("Ollama reply: {}", reply);
        assert!(!reply.is_empty());
    }
}