use crate::protocol::*;
use anyhow::Result;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct ChannelRepl {
    command_tx: mpsc::Sender<FrontendCommand>,
    message_rx: mpsc::Receiver<FrontendMessage>,
}

impl ChannelRepl {
    pub async fn new(addr: &str, actor_id: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;

        let mut codec = LengthDelimitedCodec::new();
        codec.set_max_frame_length(32 * 1024 * 1024);
        let framed = Framed::new(stream, codec);

        let (command_tx, mut command_rx) = mpsc::channel::<FrontendCommand>(32);
        let (message_tx, message_rx) = mpsc::channel::<FrontendMessage>(32);

        // Start connection handler task
        let actor_id_clone = actor_id.to_string();
        tokio::spawn(async move {
            if let Err(e) =
                handle_connection(framed, actor_id_clone, &mut command_rx, message_tx).await
            {
                eprintln!("Connection error: {}", e);
            }
        });

        Ok(Self {
            command_tx,
            message_rx,
        })
    }
}

async fn handle_connection(
    mut framed: Framed<TcpStream, LengthDelimitedCodec>,
    actor_id: String,
    command_rx: &mut mpsc::Receiver<FrontendCommand>,
    message_tx: mpsc::Sender<FrontendMessage>,
) -> Result<()> {
    // Open the channel first
    let initial_message = serde_json::json!({
        "client_type": "frontend"
    });

    let cmd = ManagementCommand::OpenChannel {
        actor_id: ChannelParticipant::Actor(actor_id),
        initial_message: serde_json::to_vec(&initial_message)?,
    };

    let cmd_bytes = serde_json::to_vec(&cmd)?;
    framed.send(Bytes::from(cmd_bytes)).await?;

    let response = framed
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("Connection closed"))??;

    let response: ManagementResponse = serde_json::from_slice(&response)?;

    let channel_id = match response {
        ManagementResponse::ChannelOpened { channel_id, .. } => channel_id,
        ManagementResponse::Error { message } => {
            anyhow::bail!("Failed to open channel: {}", message)
        }
        _ => anyhow::bail!("Unexpected response"),
    };

    // Now handle the message loop
    loop {
        tokio::select! {
            Some(command) = command_rx.recv() => {
                // Handle disconnect command
                if matches!(command, FrontendCommand::Disconnect) {
                    let close_cmd = ManagementCommand::CloseChannel {
                        channel_id: channel_id.clone(),
                    };
                    let cmd_bytes = serde_json::to_vec(&close_cmd)?;
                    framed.send(Bytes::from(cmd_bytes)).await?;
                    break;
                }

                // Send the command
                let cmd = ManagementCommand::SendOnChannel {
                    channel_id: channel_id.clone(),
                    message: serde_json::to_vec(&command)?,
                };
                let cmd_bytes = serde_json::to_vec(&cmd)?;
                framed.send(Bytes::from(cmd_bytes)).await?;
            }
            result = framed.next() => {
                match result {
                    Some(Ok(bytes)) => {
                        if let Ok(response) = serde_json::from_slice::<ManagementResponse>(&bytes) {
                            match response {
                                ManagementResponse::ChannelMessage { message, .. } => {
                                    if let Ok(msg) = serde_json::from_slice::<FrontendMessage>(&message) {
                                        if message_tx.send(msg).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                ManagementResponse::ChannelClosed { .. } => break,
                                _ => {}
                            }
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("Frame error: {}", e);
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}

fn parse_command(line: &str) -> Result<FrontendCommand> {
    match line.trim() {
        "start" => Ok(FrontendCommand::StartActor),
        "stop" => Ok(FrontendCommand::StopActor),
        "build" => Ok(FrontendCommand::BuildActor),
        "status" => Ok(FrontendCommand::GetStatus),
        "exit" | "quit" => Ok(FrontendCommand::Disconnect),
        cmd if cmd.starts_with("change ") => {
            let desc = cmd.trim_start_matches("change ").to_string();
            Ok(FrontendCommand::ChangeRequest { description: desc })
        }
        "help" => {
            println!("\nAvailable commands:");
            println!("  start         - Start the managed actor");
            println!("  stop          - Stop the managed actor");
            println!("  build         - Build the managed actor");
            println!("  change <desc> - Submit a change request");
            println!("  status        - Get current status");
            println!("  help          - Show this help message");
            println!("  exit/quit     - Exit the REPL");
            println!("");
            anyhow::bail!("") // Use error to skip command sending
        }
        _ => anyhow::bail!("Unknown command. Type 'help' for available commands."),
    }
}

fn display_message(msg: &FrontendMessage) {
    match msg {
        FrontendMessage::Status {
            child_running,
            active_operations,
        } => {
            println!("Status:");
            println!("  Child running: {}", child_running);
            if !active_operations.is_empty() {
                println!("  Active operations:");
                for op in active_operations {
                    println!("    - {} ({})", op.operation_type, op.operation_id);
                }
            }
        }
        FrontendMessage::OperationStarted {
            operation_id,
            operation_type,
            description,
        } => {
            println!("→ {} operation started: {}", operation_type, description);
            println!("  Operation ID: {}", operation_id);
        }
        FrontendMessage::OperationCompleted {
            operation_id,
            success,
            message,
        } => {
            let status = if *success { "✓" } else { "✗" };
            println!(
                "{} Operation {} complete: {}",
                status, operation_id, message
            );
        }
        FrontendMessage::ChildStarted { child_id } => {
            println!("✓ Child actor started: {}", child_id);
        }
        FrontendMessage::ChildStopped { child_id } => {
            println!("✓ Child actor stopped: {}", child_id);
        }
        FrontendMessage::Log { level, message } => {
            println!("[{}] {}", level, message);
        }
        FrontendMessage::Error { code, message } => {
            println!("Error {}: {}", code, message);
        }
    }
}

pub async fn run_repl(actor_id: &str, address: &str) -> Result<()> {
    println!(
        "Connecting to {} and opening channel to actor {}",
        address, actor_id
    );

    let repl = ChannelRepl::new(address, actor_id).await?;
    println!("Channel opened successfully");

    println!("\nType 'help' for available commands\n");

    let mut rl = rustyline::DefaultEditor::new()?;

    // Create channels for coordinating shutdown
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let _shutdown_tx_clone = shutdown_tx.clone();

    // Start message display task
    let mut message_rx = repl.message_rx;
    let display_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = message_rx.recv() => {
                    display_message(&msg);
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    });

    // Store command sender for the input loop
    let command_tx = repl.command_tx;

    // Handle user input
    loop {
        let readline = rl.readline("repl> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());

                match parse_command(&line) {
                    Ok(cmd) => {
                        if matches!(cmd, FrontendCommand::Disconnect) {
                            if let Err(e) = command_tx.send(cmd).await {
                                println!("Error sending disconnect: {}", e);
                            }
                            break;
                        }
                        if let Err(e) = command_tx.send(cmd).await {
                            println!("Error sending command: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        if !e.to_string().is_empty() {
                            println!("Error: {}", e);
                        }
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted)
            | Err(rustyline::error::ReadlineError::Eof) => {
                // Send disconnect on Ctrl-C or Ctrl-D
                if let Err(e) = command_tx.send(FrontendCommand::Disconnect).await {
                    println!("Error sending disconnect: {}", e);
                }
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }

    // Signal display task to shut down
    let _ = shutdown_tx.send(()).await;

    // Wait for display task to finish
    if let Err(e) = display_handle.await {
        println!("Error in message display task: {}", e);
    }

    println!("Goodbye!");
    Ok(())
}