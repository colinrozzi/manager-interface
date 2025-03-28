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
    NewStore {},
}

#[derive(Debug, Deserialize)]
pub enum ManagementResponse {
    StoreCreated { store_id: String },
    ActorStarted { id: String },
}

async fn create_store(framed: &mut Framed<TcpStream, LengthDelimitedCodec>) -> Result<String> {
    // Send NewStore command
    let store_command = ManagementCommand::NewStore {};
    framed.send(Bytes::from(serde_json::to_vec(&store_command)?)).await?;

    // Wait for response
    if let Some(response) = framed.next().await {
        match response {
            Ok(bytes) => {
                let response: ManagementResponse = serde_json::from_slice(&bytes)?;
                match response {
                    ManagementResponse::StoreCreated { store_id } => {
                        println!("Created new store with ID: {}", store_id);
                        Ok(store_id)
                    }
                    _ => Err(anyhow::anyhow!("Unexpected response type")),
                }
            }
            Err(e) => Err(e.into()),
        }
    } else {
        Err(anyhow::anyhow!("No response received"))
    }
}

async fn start_content_fs(
    framed: &mut Framed<TcpStream, LengthDelimitedCodec>,
    store_id: &str,
) -> Result<String> {
    let start_command = ManagementCommand::StartActor {
        manifest: "/Users/colinrozzi/work/actors/runtime-content-fs/actor.toml".to_string(),
        initial_state: Some(
            json!({ "store_id": store_id })
                .to_string()
                .into_bytes(),
        ),
    };

    framed.send(Bytes::from(serde_json::to_vec(&start_command)?)).await?;

    if let Some(response) = framed.next().await {
        match response {
            Ok(bytes) => {
                let response: ManagementResponse = serde_json::from_slice(&bytes)?;
                match response {
                    ManagementResponse::ActorStarted { id } => {
                        println!("Started runtime-content-fs with ID: {}", id);
                        Ok(id)
                    }
                    _ => Err(anyhow::anyhow!("Unexpected response type")),
                }
            }
            Err(e) => Err(e.into()),
        }
    } else {
        Err(anyhow::anyhow!("No response received"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to the theater server
    let socket = TcpStream::connect("127.0.0.1:9000").await?;

    // Set up the framed connection
    let mut codec = LengthDelimitedCodec::new();
    codec.set_max_frame_length(32 * 1024 * 1024); // 32MB max frame size
    let mut framed = Framed::new(socket, codec);

    // First create a new store
    let store_id = create_store(&mut framed).await?;

    // Then start the content-fs actor with the new store id
    let actor_id = start_content_fs(&mut framed, &store_id).await?;

    println!("Successfully created store {} and started actor {}", store_id, actor_id);

    Ok(())
}