use anyhow::Result;
use bytes::Bytes;
use clap::Parser;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Create a new store
    #[arg(long)]
    new_store: bool,

    /// Use existing store ID
    #[arg(long)]
    store_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub enum ManagementCommand {
    StartActor {
        manifest: String,
        initial_state: Option<Vec<u8>>,
    },
    NewStore {},
    RequestActorMessage {
        id: String,
        data: Vec<u8>,
    },
}

#[derive(Debug, Deserialize)]
pub enum ManagementResponse {
    StoreCreated { store_id: String },
    ActorStarted { id: String },
    RequestedMessage { id: String, message: Vec<u8> },
}

async fn create_store(framed: &mut Framed<TcpStream, LengthDelimitedCodec>) -> Result<String> {
    // Send NewStore command
    let store_command = ManagementCommand::NewStore {};
    framed
        .send(Bytes::from(serde_json::to_vec(&store_command)?))
        .await?;

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
        initial_state: Some(json!({ "store_id": store_id }).to_string().into_bytes()),
    };

    framed
        .send(Bytes::from(serde_json::to_vec(&start_command)?))
        .await?;

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

async fn check_actor_health(
    framed: &mut Framed<TcpStream, LengthDelimitedCodec>,
    actor_id: &str,
) -> Result<()> {
    let get_info_command = ManagementCommand::RequestActorMessage {
        id: actor_id.to_string(),
        data: json!({
            "action": "get-info",
            "params": []
        })
        .to_string()
        .into_bytes(),
    };

    framed
        .send(Bytes::from(serde_json::to_vec(&get_info_command)?))
        .await?;

    if let Some(response) = framed.next().await {
        match response {
            Ok(bytes) => {
                let response: ManagementResponse = serde_json::from_slice(&bytes)?;
                match response {
                    ManagementResponse::RequestedMessage { message, .. } => {
                        let response_str = String::from_utf8(message)?;
                        let response_json: serde_json::Value = serde_json::from_str(&response_str)?;

                        if response_json.get("status") == Some(&"success".into()) {
                            println!("Actor health check successful: {}", response_str);
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "Actor health check failed: {}",
                                response_str
                            ))
                        }
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
    let args = Args::parse();

    // Connect to the theater server
    let socket = TcpStream::connect("127.0.0.1:9000").await?;

    // Set up the framed connection
    let mut codec = LengthDelimitedCodec::new();
    codec.set_max_frame_length(32 * 1024 * 1024); // 32MB max frame size
    let mut framed = Framed::new(socket, codec);

    // Get store ID either from arg or by creating new store
    let store_id = match (args.new_store, args.store_id) {
        (true, _) => create_store(&mut framed).await?,
        (false, Some(id)) => id,
        (false, None) => {
            return Err(anyhow::anyhow!(
                "Please provide either --new-store flag or --store-id <ID>"
            ))
        }
    };

    // Start the content-fs actor with the store id
    let actor_id = start_content_fs(&mut framed, &store_id).await?;

    // Check actor health
    println!("Checking actor health...");
    check_actor_health(&mut framed, &actor_id).await?;

    if args.new_store {
        println!(
            "Successfully created store {} and started actor {}",
            store_id, actor_id
        );
    } else {
        println!(
            "Successfully started actor {} with existing store {}",
            actor_id, store_id
        );
    }

    Ok(())
}

