use serde::{Deserialize, Serialize};

// Theater Server Management Commands
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
    OpenChannel {
        actor_id: ChannelParticipant,
        initial_message: Vec<u8>,
    },
    SendOnChannel {
        channel_id: String,
        message: Vec<u8>,
    },
    CloseChannel {
        channel_id: String,
    },
}

#[derive(Debug, Deserialize)]
pub enum ManagementResponse {
    StoreCreated {
        store_id: String,
    },
    ActorStarted {
        id: String,
    },
    RequestedMessage {
        id: String,
        message: Vec<u8>,
    },
    ChannelOpened {
        channel_id: String,
        actor_id: ChannelParticipant,
    },
    MessageSent {
        channel_id: String,
    },
    ChannelMessage {
        channel_id: String,
        sender_id: ChannelParticipant,
        message: Vec<u8>,
    },
    ChannelClosed {
        channel_id: String,
    },
    Error {
        message: String,
    },
}

// Channel participant types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelParticipant {
    Actor(String),
    External,
}

// Frontend Commands (for REPL)
#[derive(Debug, Serialize, Deserialize)]
pub enum FrontendCommand {
    StartActor,
    StopActor,
    BuildActor,
    ChangeRequest { description: String },
    GetStatus,
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    #[serde(rename = "Start")]
    Start,
    #[serde(rename = "Stop")]
    Stop,
    #[serde(rename = "Build")]
    Build,
    #[serde(rename = "Change")]
    Change,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Start => write!(f, "Start"),
            OperationType::Stop => write!(f, "Stop"),
            OperationType::Build => write!(f, "Build"),
            OperationType::Change => write!(f, "Change"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSummary {
    pub operation_id: String,
    pub operation_type: OperationType,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FrontendMessage {
    Status {
        child_running: bool,
        active_operations: Vec<OperationSummary>,
    },
    OperationStarted {
        operation_id: String,
        operation_type: OperationType,
        description: String,
    },
    OperationCompleted {
        operation_id: String,
        success: bool,
        message: String,
    },
    ChildStarted {
        child_id: String,
    },
    ChildStopped {
        child_id: String,
    },
    Log {
        level: String,
        message: String,
    },
    Error {
        code: String,
        message: String,
    },
}

