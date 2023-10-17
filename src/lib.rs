use std::net::SocketAddr;

use num_bigint::BigInt;
use num_prime::nt_funcs::is_prime;
use serde::{de::Error, Deserialize, Serialize};
use serde_json::Number;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use tracing::Instrument;

#[derive(Error, Debug)]
pub enum PrimeTimeError {
    #[error("Invalid JSON: {0}")]
    DeserializeError(#[from] serde_json::Error),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Tokio Error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

#[derive(Deserialize, Debug, PartialEq)]
struct Request {
    method: String,
    #[serde(deserialize_with = "deserialize_number")]
    number: RequestNumber,
}

#[derive(Debug, PartialEq)]
enum RequestNumber {
    Float(f64),
    BigInt(BigInt),
}

fn deserialize_number<'de, D>(deserializer: D) -> Result<RequestNumber, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let num = Number::deserialize(deserializer)?;

    if let Some(n) = BigInt::parse_bytes(num.to_string().as_bytes(), 10) {
        return Ok(RequestNumber::BigInt(n));
    }

    if let Some(f) = num.as_f64() {
        return Ok(RequestNumber::Float(f));
    }

    Err(D::Error::custom("Invalid number value"))
}

#[derive(Serialize, Debug, PartialEq)]
struct Response {
    method: String,
    prime: bool,
}

pub async fn run(socket: SocketAddr) -> Result<(), PrimeTimeError> {
    tracing::info!("Listening on {}", socket);

    let listener = TcpListener::bind(socket).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        let span = tracing::span!(
            tracing::Level::INFO,
            "Connection", client = %stream.peer_addr()?
        );

        tokio::spawn(hanndle_connection(stream).instrument(span));
    }
}

async fn hanndle_connection(mut stream: TcpStream) -> Result<(), PrimeTimeError> {
    tracing::info!("Connected");

    let (mut reader, mut writer) = stream.split();

    let mut buf_reader = BufReader::new(&mut reader);

    loop {
        let mut line = String::new();

        let bytes_read = buf_reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            tracing::info!("Disconnected");
            return Ok(());
        }

        let response = match handle_request(line) {
            Ok(r) => r,
            Err(_) => "Invalid JSON\n".to_string(),
        };

        tracing::info!(sending = ?response);

        match writer.write_all(response.as_bytes()).await {
            Ok(_) => (),
            Err(e) => {
                tracing::error!("Failed to write to socket: {}", e);
                return Ok(());
            }
        }
    }
}

fn handle_request(json: String) -> Result<String, PrimeTimeError> {
    tracing::info!(received = ?json);

    let request: Request = serde_json::from_str(&json)?;

    let prime = match request.number {
        RequestNumber::Float(_) => false,
        RequestNumber::BigInt(n) => match n.into_parts() {
            (num_bigint::Sign::Minus, _) => false,
            (_, n) => is_prime(&n, None).probably(),
        },
    };

    let response = Response {
        method: request.method,
        prime,
    };

    let mut response = serde_json::to_string(&response)?;
    response.push('\n');

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_request_composite() {
        let input = r#"{ "method": "isPrime", "number": 18 }"#.to_string();
        let mut output = r#"{"method":"isPrime","prime":false}"#.to_string();
        output.push('\n');

        assert_eq!(handle_request(input).unwrap(), output);
    }

    #[test]
    fn test_handle_request_prime() {
        let input = r#"{ "method": "isPrime", "number": 178417 }"#.to_string();
        let mut output = r#"{"method":"isPrime","prime":true}"#.to_string();
        output.push('\n');

        assert_eq!(handle_request(input).unwrap(), output);
    }

    #[test]
    fn test_handle_request_extra_fields() {
        let input = r#"{ "method": "isPrime", "number": 30, "yolo": "swag" }"#.to_string();
        let mut output = r#"{"method":"isPrime","prime":false}"#.to_string();
        output.push('\n');

        assert_eq!(handle_request(input).unwrap(), output);
    }

    #[test]
    fn test_handle_request_bigint() {
        let input = r#"{ "method": "isPrime", "number": 529830422160613455916930483453466154480529308265681626708 }"#.to_string();
        let mut output = r#"{"method":"isPrime","prime":false}"#.to_string();
        output.push('\n');

        assert_eq!(handle_request(input).unwrap(), output);
    }

    #[test]
    fn test_handle_request_float() {
        let input = r#"{ "method": "isPrime", "number": 1.234 }"#.to_string();
        let mut output = r#"{"method":"isPrime","prime":false}"#.to_string();
        output.push('\n');

        assert_eq!(handle_request(input).unwrap(), output);
    }

    #[test]
    fn test_handle_request_string() {
        let input = r#"{ "method": "isPrime", "number": "6017832" }"#.to_string();

        assert!(handle_request(input).is_err());
    }
}
