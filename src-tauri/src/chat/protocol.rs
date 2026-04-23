//! JSON payloads for `chat-agent-event` (envelope includes `character`).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatEnvelope {
    pub character: String,
    pub event: ChatEventBody,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEventBody {
    UserEcho { text: String },
    AssistantDelta { text: String },
    AssistantDone,
    ToolUse { name: String, summary: String },
    ToolResult { summary: String, is_error: bool },
    Error { message: String },
    SessionReady,
    TurnComplete,
    ProcessExit { code: Option<i32> },
}
