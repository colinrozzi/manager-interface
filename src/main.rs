use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use bytes::Bytes;
use serde_json::{json, Value};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to the theater server
    let socket = TcpStream::connect("127.0.0.1:9000").await?;
    
    // Set up the framed connection
    let mut codec = LengthDelimitedCodec::new();
    codec.set_max_frame_length(32 * 1024 * 1024); // 32MB max frame size
    let mut framed = Framed::new(socket, codec);
    
    // Create the start actor command
    let start_command = json!({
        "StartActor": {
            "manifest": "runtime-content-fs",
            "initial_state": null
        }
    });
    
    // Send the command
    framed.send(Bytes::from(start_command.to_string())).await?;
    
    // Wait for the response
    if let Some(response) = framed.next().await {
        match response {
            Ok(bytes) => {
                let response: Value = serde_json::from_slice(&bytes)?;
                if let Some(actor_started) = response.get("ActorStarted") {
                    if let Some(id) = actor_started.get("id") {
                        println!("Started runtime-content-fs with ID: {}", id);
                        // Here you could save the ID to a file or environment variable
                    }
                }
            }
            Err(e) => println!("Error receiving response: {}", e),
        }
    }
    
    Ok(())
}