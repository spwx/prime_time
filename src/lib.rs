use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
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
    number: i64,
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

    let prime = is_prime(request.number);

    let response = Response {
        method: request.method,
        prime,
    };

    let mut response = serde_json::to_string(&response)?;
    response.push('\n');

    Ok(response)
}

fn is_prime(n: i64) -> bool {
    match n {
        0 | 1 => false,
        2 => true,
        _ if n < 0 => false,      // negative numbers are not prime
        _ if n % 2 == 0 => false, // early return for even numbers
        _ => {
            let sqrt = (n as f64).sqrt() as i64;
            (3..=sqrt).step_by(2).all(|i| n % i != 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::{io::AsyncReadExt, net::TcpListener};

    use super::*;

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(5));
        assert!(!is_prime(16));
        assert!(is_prime(17));
        assert!(!is_prime(18));
        assert!(is_prime(19));
        assert!(is_prime(13));
        assert!(!is_prime(-13));
    }

    #[test]
    fn test_deserialize_request() {
        let json_data = r#"
            {
                "method": "isPrime",
                "number": 30
            }
        "#;

        let expected_request = Request {
            method: "isPrime".to_string(),
            number: 30,
        };

        let request: Request = serde_json::from_str(json_data).unwrap();

        assert_eq!(request, expected_request);
    }

    #[test]
    fn test_deserialize_request_extra_fields() {
        let json = r#"
            {
                "method": "isPrime",
                "number": 30,
                "yolo": "swag"
            }
        "#;

        let expected_request = Request {
            method: "isPrime".to_string(),
            number: 30,
        };

        let request: Request = serde_json::from_str(json).unwrap();

        assert_eq!(request, expected_request);
    }

    #[test]
    fn test_serialize_response() {
        let response = Response {
            method: "isPrime".to_string(),
            prime: true,
        };

        let expected_response = r#"{"method":"isPrime","prime":true}"#;

        let json = serde_json::to_string(&response).unwrap();

        assert_eq!(json, expected_response);
    }

    #[test]
    fn test_handle_request() {
        let input = r#"{ "method": "isPrime", "number": 30, "yolo": "swag" }"#.to_string();
        let mut output = r#"{"method":"isPrime","prime":false}"#.to_string();
        output.push('\n');

        assert_eq!(handle_request(input).unwrap(), output);
    }

    #[tokio::test]
    #[ignore]
    async fn test_run() {
        // Spin up the server using the run function
        tokio::spawn(async {
            let socket = SocketAddr::from(([127, 0, 0, 1], 8282));
            run(socket).await.expect("Could not bind to socket");
        });

        // Give the server some time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Connect to the server and send a request
        let mut connection = TcpStream::connect("127.0.0.1:58282")
            .await
            .expect("Failed to connect");

        let (mut reader, mut writer) = connection.split();

        let input = br#"{ "method": "isPrime", "number": 30, "yolo": "swag" }"#;
        writer.write_all(input).await.unwrap();

        let mut buf = String::new();
        let mut buffered_reader = BufReader::new(&mut reader);
        buffered_reader.read_line(&mut buf).await.unwrap();

        let output = r#"{"method":"isPrime","prime":false}"#.to_string();

        assert_eq!(buf, output);
    }

    #[tokio::test]
    #[ignore]
    async fn test_run_2() {
        let socket = SocketAddr::from(([127, 0, 0, 1], 8282));

        run(socket).await.expect("Could not bind to socket");

        let mut connection = TcpStream::connect("127.0.0.1:8282")
            .await
            .expect("Failed to connect");

        let (mut reader, mut writer) = connection.split();

        let input = br#"{ "method": "isPrime", "number": 30, "yolo": "swag" }"#;
        writer.write_all(input).await.unwrap();

        let mut buf = String::new();
        let mut buffered_reader = BufReader::new(&mut reader);
        buffered_reader.read_line(&mut buf).await.unwrap();

        let output = r#"{"method":"isPrime","prime":false}"#.to_string();

        assert_eq!(buf, output);
    }

    #[tokio::test]
    #[ignore]
    async fn test_handle_connection() {
        // Create a server
        let server = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind");

        // Get the address of the server
        let socket = server.local_addr().expect("Failed to get socket address");

        // Accept and handle a connection
        tokio::spawn(async move {
            let (connection, _) = server
                .accept()
                .await
                .expect("Failed to accept a connection");

            hanndle_connection(connection)
                .await
                .expect("Failed to handle connection");
        });

        let mut buf = String::new();

        // Create a client
        {
            let mut client = TcpStream::connect(&socket)
                .await
                .expect("Client failed to connect");

            let input = br#"{ "method": "isPrime", "number": 30, "yolo": "swag" }"#;
            client.write_all(input).await.unwrap();

            client.read_to_string(&mut buf).await.unwrap();
        }

        let output = r#"{"method":"isPrime","prime":false}"#.to_string();

        assert_eq!(buf, output);
    }
}
