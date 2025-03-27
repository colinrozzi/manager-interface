use anyhow::Result;
use bytes::Bytes;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[derive(Debug, Serialize)]
pub enum ManagementCommand {
    StartActor {
        manifest: String,
        initial_state: Option<Vec<u8>>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to the theater server
    let socket = TcpStream::connect("127.0.0.1:9000").await?;

    // Set up the framed connection
    let mut codec = LengthDelimitedCodec::new();
    codec.set_max_frame_length(32 * 1024 * 1024); // 32MB max frame size
    let mut framed = Framed::new(socket, codec);

    // Create the start actor command
    let start_command = ManagementCommand::StartActor {
        manifest: "/Users/colinrozzi/work/actors/runtime-content-fs/actor.toml".to_string(),
        initial_state: Some(
            json!({ "store_id": "c25fb0da-17fd-4d24-8aea-35b58ac1bcc8" })
                .to_string()
                .into_bytes(),
        ),
    };

    // Send the command
    framed
        .send(serde_json::to_vec(&start_command)?.into())
        .await?;

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

