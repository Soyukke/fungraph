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
