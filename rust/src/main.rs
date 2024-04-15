use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
mod parser;
use parser::parse;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((mut stream, _)) => {
                println!("Accepted new connection!");
                let handle_result = tokio::spawn(async move {
                    let mut buf = [0; 512];
                    loop {
                        let read_count = stream.read(&mut buf).await;
                        match read_count {
                            Ok(0) => break, // Client disconnected gracefully
                            Ok(_) => {
                                // Echo back PONG
                                parse(&buf);
                                if let Err(e) = stream.write(b"+PONG\r\n").await {
                                    println!("Error writing to stream: {}", e);
                                    break; // Client disconnected abruptly
                                }
                            }
                            Err(e) => {
                                println!("Error reading from stream: {}", e);
                                break; // Client disconnected abruptly
                            }
                        }
                    }
                });
            }

            Err(e) => {
                println!("Error connecting {}", e);
            }
        }
    }
}
