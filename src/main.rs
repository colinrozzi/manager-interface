mod protocol;
mod repl;

use anyhow::Result;
use bytes::Bytes;
use clap::Parser;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use protocol::*;
use serde_json::json;
use std::env;
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

    /// Use existing build store ID
    #[arg(long)]
    build_store_id: Option<String>,

    /// Server address
    #[arg(long, default_value = "127.0.0.1:9000")]
    address: String,
}

async fn create_store(framed: &mut Framed<TcpStream, LengthDelimitedCodec>) -> Result<String> {
    let store_command = ManagementCommand::NewStore {};
    framed.send(Bytes::from(serde_json::to_vec(&store_command)?)).await?;

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

async fn start_actor_uploader(
    framed: &mut Framed<TcpStream, LengthDelimitedCodec>,
    runtime_content_fs_id: &str,
) -> Result<String> {
    let start_command = ManagementCommand::StartActor {
        manifest: "/Users/colinrozzi/work/actors/actor-uploader/child-actor.toml".to_string(),
        initial_state: Some(
            json!({
                "runtime_content_fs_address": runtime_content_fs_id
            })
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
                        println!("Started actor-uploader with ID: {}", id);
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

async fn start_manager_actor(
    framed: &mut Framed<TcpStream, LengthDelimitedCodec>,
    build_store_id: &str,
    runtime_content_fs_id: &str,
) -> Result<String> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY environment variable not set"))?;

    let start_command = ManagementCommand::StartActor {
        manifest: "/Users/colinrozzi/work/actors/manager/actor.toml".to_string(),
        initial_state: Some(
            json!({
                "build_store_id": build_store_id,
                "runtime_content_fs_actor_id": runtime_content_fs_id,
                "anthropic_api_key": api_key
            })
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
                        println!("Started manager actor with ID: {}", id);
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
    let args = Args::parse();

    // Connect to the theater server
    let socket = TcpStream::connect(&args.address).await?;

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

    // Get or create build store ID
    let build_store_id = match args.build_store_id {
        Some(id) => id,
        None => {
            println!("Creating new build store...");
            create_store(&mut framed).await?
        }
    };

    // Start the content-fs actor with the store id
    let content_fs_id = start_content_fs(&mut framed, &store_id).await?;

    // Check actor health
    println!("Checking runtime-content-fs health...");
    let get_info_command = ManagementCommand::RequestActorMessage {
        id: content_fs_id.clone(),
        data: json!({
            "action": "get-info",
            "params": []
        })
        .to_string()
        .into_bytes(),
    };

    framed.send(Bytes::from(serde_json::to_vec(&get_info_command)?)).await?;

    // If we created a new store, start the actor uploader
    if args.new_store {
        println!("Starting actor uploader to upload template child actor...");
        let uploader_id = start_actor_uploader(&mut framed, &content_fs_id).await?;
        println!("Successfully created store {} and started actors:\n  runtime-content-fs: {}\n  actor-uploader: {}", 
            store_id, content_fs_id, uploader_id);
    } else {
        println!("Successfully started runtime-content-fs {} with existing store {}", content_fs_id, store_id);
    }

    // Start the manager actor
    println!("Starting manager actor...");
    let manager_id = start_manager_actor(&mut framed, &build_store_id, &content_fs_id).await?;

    println!("\nSystem is ready with:");
    println!("  Runtime Store ID: {}", store_id);
    println!("  Build Store ID: {}", build_store_id);
    println!("  Runtime Content FS Actor ID: {}", content_fs_id);
    println!("  Manager Actor ID: {}", manager_id);

    // Start the REPL connected to the manager actor
    println!("\nStarting REPL session...");
    repl::run_repl(&manager_id, &args.address).await?;

    Ok(())
}