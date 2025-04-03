use crate::{
    llm::Message,
    types::openai::{CompletionTokensDetails, FinishReason, PromptTokensDetails, Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatCompletionMessageToolCall {
    pub id: String,
    pub r#type: ChatCompletionToolType,
    pub function: FunctionCall,
}

#[derive(Clone, Serialize, Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatCompletionToolType {
    #[default]
    Function,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    #[default]
    User,
    Assistant,
    Tool,
    Function,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatCompletionResponseMessage {
    pub content: Option<String>,
    pub refusal: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,

    pub role: Role,

    #[deprecated]
    pub function_call: Option<FunctionCall>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct TopLogprobs {
    pub token: String,
    pub logprob: f32,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatCompletionTokenLogprob {
    pub token: String,
    pub logprob: f32,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Vec<TopLogprobs>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatChoiceLogprobs {
    pub content: Option<Vec<ChatCompletionTokenLogprob>>,
    pub refusal: Option<Vec<ChatCompletionTokenLogprob>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatCompletionResponseMessage,
    pub finish_reason: Option<FinishReason>,
    pub logprobs: Option<ChatChoiceLogprobs>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiResponse {
    /// gemini open ai api will not contain id.
    pub id: Option<String>,
    pub choices: Vec<ChatChoice>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<String>,
    pub system_fingerprint: Option<String>,
    pub object: String,
    pub usage: Option<CompletionUsage>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct CompletionUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIContent {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiInlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiSafetyRating {
    category: String,
    probability: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiPromptFeedback {
    safety_ratings: Vec<GeminiSafetyRating>,
}

#[derive(Debug, Serialize)]
pub struct GeminiRequest {
    pub messages: Vec<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}
