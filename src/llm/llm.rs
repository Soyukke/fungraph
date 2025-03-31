use async_trait::async_trait;
use futures::Stream;
use log::debug;
use std::{collections::HashMap, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::types::{StreamData, TokenUsage};

use super::{LLMError, Message};

#[async_trait]
pub trait LLM: Send + Sync {
    async fn generate(&self, prompt: &[Message]) -> Result<GenerateResult, LLMError>;
    async fn invoke(&self, prompt: &str) -> Result<String, LLMError>;
    async fn stream(
        &self,
        messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError>;
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

//TODO: check if its this should have a data:serde::Value to save all other things, like OpenAI
//function responses
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GenerateResult {
    pub tokens: Option<TokenUsage>,
    pub generation: String,
}

impl GenerateResult {
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
}
