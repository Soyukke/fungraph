use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::types::openai::Tool;

/// Enum `MessageType` represents the type of a message.
/// It can be a `SystemMessage`, `AIMessage`, or `HumanMessage`.
///
/// # Usage
/// ```rust,ignore
/// let system_message_type = MessageType::SystemMessage;
/// let ai_message_type = MessageType::AIMessage;
/// let human_message_type = MessageType::HumanMessage;
/// ```
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    #[serde(rename = "system")]
    SystemMessage,
    #[serde(rename = "ai")]
    AIMessage,
    #[serde(rename = "human")]
    HumanMessage,
    #[serde(rename = "tool")]
    ToolMessage,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::SystemMessage
    }
}

impl MessageType {
    pub fn to_string(&self) -> String {
        match self {
            MessageType::SystemMessage => "system".to_owned(),
            MessageType::AIMessage => "ai".to_owned(),
            MessageType::HumanMessage => "human".to_owned(),
            MessageType::ToolMessage => "tool".to_owned(),
        }
    }
}

/// Struct `ImageContent` represents an image provided to an LLM.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ImageContent {
    pub image_url: String,
    pub detail: Option<String>,
}

impl<S: AsRef<str>> From<S> for ImageContent {
    fn from(image_url: S) -> Self {
        ImageContent {
            image_url: image_url.as_ref().into(),
            detail: None,
        }
    }
}

/// Struct `Message` represents a message with its content and type.
///
/// # Usage
/// ```rust,ignore
/// let human_message = Message::new_human_message("Hello");
/// let system_message = Message::new_system_message("System Alert");
/// let ai_message = Message::new_ai_message("AI Response");
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Message {
    pub content: Option<String>,
    #[serde(rename = "role")]
    pub message_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageContent>>,
    /// tool name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    // Function to create a new Human message with a generic type that implements Display
    pub fn new_human_message<T: std::fmt::Display>(content: T) -> Self {
        Message {
            content: Some(content.to_string()),
            message_type: MessageType::HumanMessage,
            id: None,
            tool_calls: None,
            images: None,
            name: None,
        }
    }

    pub fn new_human_message_with_images<T: Into<ImageContent>>(images: Vec<T>) -> Self {
        Message {
            content: Some(String::default()),
            message_type: MessageType::HumanMessage,
            id: None,
            tool_calls: None,
            images: Some(images.into_iter().map(|i| i.into()).collect()),
            name: None,
        }
    }

    pub fn new_system_message<T: std::fmt::Display>(content: T) -> Self {
        Message {
            content: Some(content.to_string()),
            message_type: MessageType::SystemMessage,
            id: None,
            tool_calls: None,
            images: None,
            name: None,
        }
    }

    pub fn new_ai_message<T: std::fmt::Display>(content: T) -> Self {
        Message {
            content: Some(content.to_string()),
            message_type: MessageType::AIMessage,
            id: None,
            tool_calls: None,
            images: None,
            name: None,
        }
    }

    pub fn new_tool_message<T: std::fmt::Display, S: Into<String>>(content: T, id: S) -> Self {
        Message {
            content: Some(content.to_string()),
            message_type: MessageType::ToolMessage,
            id: Some(id.into()),
            tool_calls: None,
            images: None,
            name: None,
        }
    }

    pub fn with_tool_calls(mut self, tool_calls: Value) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    pub fn messages_from_value(value: &Value) -> Result<Vec<Message>, serde_json::error::Error> {
        serde_json::from_value(value.clone())
    }
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct Messages {
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
}

impl Messages {
    pub fn builder() -> MessagesBuilder {
        MessagesBuilder::new()
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }
}

pub struct MessagesBuilder {
    messages: Vec<Message>,
    tools: Vec<Tool>,
}

impl MessagesBuilder {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            tools: vec![],
        }
    }

    pub fn add_human_message(mut self, content: &str) -> Self {
        self.messages.push(Message::new_human_message(content));
        self
    }

    pub fn add_system_message(mut self, content: &str) -> Self {
        self.messages.push(Message::new_system_message(content));
        self
    }

    pub fn add_tool_message(mut self, content: &str, id: &str) -> Self {
        self.messages.push(Message::new_tool_message(content, id));
        self
    }

    pub fn add_ai_message(mut self, content: &str) -> Self {
        self.messages.push(Message::new_ai_message(content));
        self
    }

    pub fn add_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn build(self) -> Messages {
        Messages {
            messages: self.messages,
            tools: self.tools,
        }
    }
}
