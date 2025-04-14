use std::{collections::HashMap, pin::Pin};

use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::{GenerateResult, LLMError};

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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatCompletionMessageToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ChatCompletionToolType,
    pub function: FunctionCall,
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
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
struct OpenAIResponse {
    pub id: Option<String>,
    pub choices: Vec<ChatChoice>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<String>,
    pub system_fingerprint: Option<String>,
    pub object: String,
    pub usage: Option<String>,
}

#[derive(Clone, Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTierResponse {
    Scale,
    Default,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct CompletionUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct CompletionTokensDetails {
    pub accepted_prediction_tokens: Option<u32>,
    pub audio_tokens: Option<u32>,
    pub reasoning_tokens: Option<u32>,
    pub rejected_prediction_tokens: Option<u32>,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct PromptTokensDetails {
    pub audio_tokens: Option<u32>,
    pub cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
pub struct CreateChatCompletionStreamResponse {
    pub id: Option<String>,
    pub choices: Vec<ChatChoiceStream>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<ServiceTierResponse>,
    pub system_fingerprint: Option<String>,
    pub object: String,
    pub usage: Option<CompletionUsage>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct FunctionCallStream {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatCompletionMessageToolCallChunk {
    pub index: Option<i32>,
    pub id: Option<String>,
    pub r#type: Option<ChatCompletionToolType>,
    pub function: Option<FunctionCallStream>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatCompletionStreamResponseDelta {
    pub content: Option<String>,
    #[deprecated]
    pub function_call: Option<FunctionCallStream>,

    pub tool_calls: Option<Vec<ChatCompletionMessageToolCallChunk>>,
    pub role: Option<Role>,
    pub refusal: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ChatChoiceStream {
    pub index: u32,
    pub delta: ChatCompletionStreamResponseDelta,
    pub finish_reason: Option<FinishReason>,
    pub logprobs: Option<ChatChoiceLogprobs>,
}

pub type ChatCompletionResponseStream =
    Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, LLMError>> + Send>>;
pub type GenerateResultStream =
    Pin<Box<dyn Stream<Item = Result<GenerateResult, LLMError>> + Send>>;

#[derive(Debug, Serialize, Clone)]
pub enum ToolType {
    #[serde(rename = "function")]
    Function,
}

#[derive(Debug, Serialize, Clone)]
pub struct Tool {
    #[serde(rename = "type")]
    pub r#type: ToolType,
    pub function: FunctionDescription,
}

#[derive(Debug, Serialize, Clone)]
pub struct FunctionDescription {
    pub name: String,
    pub description: String,
    pub parameters: Parameters,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameters {
    #[serde(rename = "type")]
    pub r#type: String,
    pub properties: HashMap<String, Property>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Property {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Items>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Items {
    #[serde(rename = "type")]
    pub r#type: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::types::openai::{FunctionDescription, Parameters, ToolType};

    use super::*;

    #[test]
    fn test_tool_json() {
        let tool = Tool {
            r#type: ToolType::Function,
            function: FunctionDescription {
                name: "test_function".to_string(),
                description: "This is a test function".to_string(),
                parameters: Parameters {
                    r#type: "object".to_string(),
                    properties: HashMap::new(),
                    required: Some(vec!["param1".to_string()]),
                },
            },
        };
        let result = serde_json::to_value(&tool).unwrap();
        let expected = json!({
            "type": "function",
            "function": {
            "name": "test_function",
            "description": "This is a test function",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": ["param1"]
            }
            }
        });
        assert_eq!(result, expected);
    }
}
