use std::{any::Any, collections::HashMap};

use async_trait::async_trait;
use log::{debug, info};

use crate::{
    llm::{self, GenerateResult, LLM, LLMError, LLMResult, Message, Messages, MessagesBuilder},
    tools::Tool,
};

pub type Conversations = Vec<Conversation>;

#[derive(Debug)]
pub struct Conversation {
    pub request: Messages,
    pub response: LLMResult,
}

#[async_trait]
pub trait LLMAgentable<T: LLM> {
    fn get_name(&self) -> String;
    fn get_llm(&self) -> &T;
    async fn chat(&self, message: &str) -> Result<Conversations, anyhow::Error> {
        let messages = MessagesBuilder::new().add_human_message(message).build();
        let llm = self.get_llm();
        let result = llm.invoke(&messages).await?;
        let conversations = vec![Conversation {
            request: messages,
            response: result,
        }];
        Ok(conversations)
    }
}

pub struct LLMAgent<T>
where
    T: LLM,
{
    llm: T,
    system_prompt: Option<String>,
    tools: HashMap<String, Box<dyn Tool>>,
}

impl<T> LLMAgent<T>
where
    T: LLM,
{
    pub fn builder(llm: T) -> LLMAgentBuilder<T> {
        LLMAgentBuilder::new(llm)
    }

    fn build_messages(&self, message: &str) -> Messages {
        let mut builder = MessagesBuilder::new();
        if let Some(system_prompt) = &self.system_prompt {
            builder = builder.add_system_message(system_prompt);
        }

        let tools = self
            .tools
            .iter()
            .map(|(_, tool)| tool.to_openai_tool())
            .collect::<Vec<_>>();

        if !tools.is_empty() {
            builder = builder.add_tools(tools);
        }

        builder.add_human_message(message).build()
    }

    pub async fn chat(&self, message: &str) -> Result<Conversations, LLMError> {
        debug!("LLMAgent: Chat: {}", message);
        let mut messages = self.build_messages(message);
        let result = self.llm.invoke(&messages).await?;
        let mut conversations = vec![Conversation {
            request: messages.clone(),
            response: result.clone(),
        }];

        debug!("LLMAgent: Chat: {:?}", messages);
        debug!("LLMAgent: Chat result: {:?}", result);
        match result {
            LLMResult::Generate(generate_result) => {
                info!("LLMAgent: Stop: {:?}", generate_result);
                // Stop
            }
            LLMResult::ToolCall(tool_call_result) => {
                messages.add_message(tool_call_result.ai_message.clone());
                // call tool
                debug!("LLMAgent: Tool call: {:?}", tool_call_result);
                let target_tool = self.tools.get(&tool_call_result.name);
                debug!("LLMAgent: Tool is some {:?}", target_tool.is_some());
                if let Some(tool) = target_tool {
                    let result = tool.call(&tool_call_result.arguments).await;
                    debug!("LLMAgent: Tool call result: {:?}", result);
                    let tool_message =
                        Message::new_tool_message(result?, &tool_call_result.id.to_string());
                    messages.add_message(tool_message);

                    // TODO: ここで実際にinvokeする
                    debug!("LLMAgent: re invoke:");
                    debug!("LLMAgent new message: {:?}", messages);
                    let result = self.llm.invoke(&messages).await?;
                    debug!("LLMAgent: After tool call result: {:?}", result);

                    conversations.push(Conversation {
                        request: messages.clone(),
                        response: result,
                    });
                } else {
                    debug!("LLMAgent: Tool not found");
                }
            }
        }

        Ok(conversations)
    }
}

pub struct LLMAgentBuilder<T>
where
    T: LLM,
{
    llm: T,
    system_prompt: Option<Message>,
    tools: HashMap<String, Box<dyn Tool>>,
}

impl<T> LLMAgentBuilder<T>
where
    T: LLM,
{
    pub fn new(llm: T) -> Self {
        LLMAgentBuilder {
            llm: llm,
            system_prompt: None,
            tools: HashMap::new(),
        }
    }
    pub fn build(self) -> Result<LLMAgent<T>, anyhow::Error> {
        Ok(LLMAgent {
            llm: self.llm,
            system_prompt: None,
            tools: self.tools,
        })
    }

    pub fn with_system_prompt(mut self, system_prompt: &str) -> Self {
        let message = Message::new_system_message(system_prompt);
        self.system_prompt = Some(message);
        self
    }

    pub fn with_tool<A: Tool + 'static>(mut self, tool: A) -> Self {
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), Box::new(tool));
        self
    }
}

// Toolの呼び出しを含むメッセージの例
// ```json
// {
//   "choices": [
//     {
//       "finish_reason": "tool_calls",
//       "index": 0,
//       "message": {
//         "content": null,
//         "role": "assistant",
//         "tool_calls": [
//           {
//             "id": "call_abc123",
//             "function": {
//               "arguments": "{\"location\": \"tokyo\"}",
//               "name": "get_weather"
//             },
//             "type": "function"
//           }
//         ]
//       }
//     }
//   ],
//   "created": 1699999999,
//   "id": "chatcmpl-xxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
//   "model": "gpt-3.5-turbo-0613",
//   "object": "chat.completion",
//   "usage": {
//     "completion_tokens": 123,
//     "prompt_tokens": 456,
//     "total_tokens": 579
//   }
// }
// ```

// Agentにはシステムプロンプトを初期設定したい。
// Agentにはツールを初期設定したい。
// 1 Agent n tools, 1 system prompt

pub mod gemini;
