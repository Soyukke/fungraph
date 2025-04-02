use async_trait::async_trait;
use futures::Stream;
use log::debug;
use std::{collections::HashMap, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::types::{ChatCompletionResponseStream, StreamData, TokenUsage};

use super::{LLMError, Message, Messages};

#[async_trait]
pub trait LLM: Send + Sync {
    async fn generate(&self, prompt: &[Message]) -> Result<GenerateResult, LLMError>;
    async fn invoke(&self, messages: &Messages) -> Result<GenerateResult, LLMError>;
    async fn invoke_stream_one_result(
        &self,
        messages: &[Message],
    ) -> Result<GenerateResult, LLMError>;
    async fn invoke_stream(
        &self,
        messages: &[Message],
    ) -> Result<ChatCompletionResponseStream, LLMError>;
    fn add_options(&mut self, options: &CallOptions);
}

#[derive(Clone, Debug)]
pub struct CallOptions {}

impl CallOptions {
    pub fn merge(&self, other: &CallOptions) -> CallOptions {
        debug!("Merging options: {:?} and {:?}", self, other);
        CallOptions {}
    }
}

impl Default for CallOptions {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GenerateResult {
    tokens: Option<TokenUsage>,
    generation: String,
}

impl GenerateResult {
    pub fn new(generation: String, tokens: Option<TokenUsage>) -> Self {
        Self { generation, tokens }
    }

    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        // Insert the 'generation' field into the hashmap
        map.insert("generation".to_string(), self.generation.clone());

        // Check if 'tokens' is Some and insert its fields into the hashmap
        if let Some(ref tokens) = self.tokens {
            map.insert(
                "prompt_tokens".to_string(),
                tokens.prompt_tokens.to_string(),
            );
            map.insert(
                "completion_tokens".to_string(),
                tokens.completion_tokens.to_string(),
            );
            map.insert("total_tokens".to_string(), tokens.total_tokens.to_string());
        }

        map
    }

    pub fn generation(&self) -> &str {
        &self.generation
    }

    pub fn set_generation(&mut self, generation: &str) {
        self.generation = generation.to_string();
    }

    pub fn push_generation(&mut self, generation: &str) {
        self.generation.push_str(generation);
    }
}
