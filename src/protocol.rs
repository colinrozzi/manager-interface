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
pub enum BuildEventType {
    #[serde(rename = "Log")]
    Log,
    #[serde(rename = "Progress")]
    Progress,
    #[serde(rename = "CommandStarted")]
    CommandStarted,
    #[serde(rename = "CommandOutput")]
    CommandOutput,
    #[serde(rename = "BuildComplete")]
    BuildComplete,
    #[serde(rename = "FileExtracted")]
    FileExtracted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildEventDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent_complete: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_path: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_hash: Option<String>,
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
    OperationProgress {
        operation_id: String,
        description: String,
        percent_complete: f32,
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
    BuildEvent {
        operation_id: String,
        event_type: BuildEventType,
        message: String,
        details: BuildEventDetails,
    },
}

