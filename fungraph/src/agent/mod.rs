mod mcp_agent;
use futures::Stream;
pub use mcp_agent::*;
use std::{collections::HashMap, pin::Pin, task::{Context, Poll}, thread::sleep, time::Duration};

use fungraph_llm::{LLM, LLMError, LLMResult, Message, Messages, MessagesBuilder};
use log::{debug, info};

use crate::tools::FunTool;

pub type Conversations = Vec<Conversation>;

#[derive(Debug)]
pub struct AgentResponse {
    pub final_answer: String,
    pub intermediate_steps: Vec<Conversation>,
}

pub struct AgentStream {}

impl Stream for MyDataStream {
    type Item = NextAction;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(NextAction::ToolCall))
    }
}


/// ツール呼び出し要求
/// 普通の回答
/// LLM問い合わせ
pub enum NextAction {
    ToolCall,
    Response,
    Request,
}

#[derive(Debug)]
pub struct Conversation {
    pub request: Messages,
    pub response: LLMResult,
}

pub struct LLMAgent<T>
where
    T: LLM,
{
    llm: T,
    system_prompt: Option<String>,
    tools: HashMap<String, Box<dyn FunTool>>,
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

    fn build_messages2(&self, messages: &mut Messages) -> Messages {
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

        for message in messages.messages.iter() {
            builder = builder.add_message(message);
        }

        builder.build()
    }

    async fn start(
        &self,
        messages: &Messages,
    ) -> Result<AgentStream, LLMError> {
        let mut messages = self.build_messages2(messages);
    }

    pub async fn invoke(&self, messages: &Messages) -> Result<AgentResponse, LLMError> {
        let mut messages = messages.clone();
        let mut messages = self.build_messages2(&mut messages);
        let result = self.llm.invoke(&messages).await?;
        let mut conversations = vec![Conversation {
            request: messages.clone(),
            response: result.clone(),
        }];

        let final_answer = match result {
            LLMResult::Generate(_generate_result) => {
                _generate_result.generation()
            }
            LLMResult::ToolCall(tool_call_result) => {
                messages.add_message(tool_call_result.ai_message.clone());
                let target_tool = self.tools.get(&tool_call_result.name);
                if let Some(tool) = target_tool {
                    let result = tool.call(tool_call_result.arguments).await;
                    let tool_message =
                        Message::new_tool_message(result?, &tool_call_result.id.to_string());
                    messages.add_message(tool_message);

                    let result = self.llm.invoke(&messages).await?;

                    conversations.push(Conversation {
                        request: messages.clone(),
                        response: result,
                    });
                } else {
                    debug!("LLMAgent: Tool not found");
                }
            }
        }

        Ok(AgentResponse {
            final_answer,
            intermediate_steps: conversations,
        })
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
            LLMResult::Generate(_generate_result) => {
                // Stop
            }
            LLMResult::ToolCall(tool_call_result) => {
                messages.add_message(tool_call_result.ai_message.clone());
                let target_tool = self.tools.get(&tool_call_result.name);
                if let Some(tool) = target_tool {
                    let result = tool.call(tool_call_result.arguments).await;
                    let tool_message =
                        Message::new_tool_message(result?, &tool_call_result.id.to_string());
                    messages.add_message(tool_message);

                    let result = self.llm.invoke(&messages).await?;

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

struct MyDataStream {
    index: usize,
    data:Vec<usize> 
}
struct MyData {
    id: u32,
    name: String,
}

impl Stream for MyDataStream {
    type Item = usize;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.index >= self.data.len() {
            return Poll::Ready(None);
        }

        let item = self.data[self.index].clone();
        self.index += 1;


        Poll::Ready(Some(item))
    }
}

pub struct LLMAgentBuilder<T>
where
    T: LLM,
{
    llm: T,
    system_prompt: Option<Message>,
    tools: HashMap<String, Box<dyn FunTool>>,
}

impl<T> LLMAgentBuilder<T>
where
    T: LLM,
{
    pub fn new(llm: T) -> Self {
        LLMAgentBuilder {
            llm,
            system_prompt: None,
            tools: HashMap::new(),
        }
    }
    pub fn build(self) -> Result<LLMAgent<T>, anyhow::Error> {
        Ok(LLMAgent {
            llm: self.llm,
            system_prompt: self.system_prompt.unwrap().content,
            tools: self.tools,
        })
    }

    pub fn with_system_prompt(mut self, system_prompt: &str) -> Self {
        let message = Message::new_system_message(system_prompt);
        self.system_prompt = Some(message);
        self
    }

    pub fn with_tool<A: FunTool + 'static>(mut self, tool: A) -> Self {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use fungraph_llm::{
        gemini::{Gemini, GeminiConfigBuilder, OpenAIMessages},
        openai::{Parameters, Property, Tool},
    };
    use futures::task::waker;
    use httpmock::{
        Method::POST, Mock, MockServer, server::matchers::readers::expectations::body_includes,
    };
    use log::{debug, info};
    use serde_json::{Value, json};

    use crate::tools::ToolParameters;

    use super::*;
    use anyhow::Result;

    fn mock_gemini_api(status: u16, body: &str) -> MockServer {
        mock_gemini_api_multiples(status, &[body])
    }

    fn mock_gemini_api_multiples(status: u16, body_list: &[&str]) -> MockServer {
        let server = MockServer::start();
        for body in body_list {
            server.mock(|when, then| {
                when.method(POST).path("/chat/completions");
                then.status(status)
                    .header("content-type", "text/json; charset=UTF-8")
                    .body(body);
            });
        }
        server
    }

    #[tokio::test]
    async fn test_agent_chat() -> Result<()> {
        let server = mock_gemini_api(
            200,
            r#"{"choices":[{"finish_reason":"stop","index":0,"message":{"content":"こんにちは","role":"assistant"}}],"created":1743601854,"model":"gemini-2.0-flash","object":"chat.completion","usage":{"completion_tokens":1527,"prompt_tokens":6,"total_tokens":1533}}"#,
        );
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key")
            .with_api_base(&server.url(""))
            .build()?;

        // 3. Gemini クライアントを作成します
        let gemini = Gemini::new(config);
        let agent = LLMAgent::builder(gemini)
            .with_system_prompt("あなたは親切なアシスタントです。")
            .build()?;
        // llmへのリクエストとレスポンスのペア
        let results = agent.chat("こんにちは").await?;
        match &results.first().unwrap().response {
            LLMResult::Generate(result) => {
                assert_eq!(result.generation(), "こんにちは");
            }
            _ => panic!("No results returned"),
        }
        Ok(())
    }

    struct MyTool;
    struct MyToolParameters {
        name: String,
    }

    impl ToolParameters for MyToolParameters {
        fn parameters() -> Parameters {
            let location_prop = Property {
                r#type: "string".to_string(),
                description: Some("The city and state, e.g. San Francisco, CA".to_string()),
                enum_values: None,
                items: None,
            };
            let unit_prop = Property {
                r#type: "string".to_string(),
                description: Some(
                    "The temperature unit to use. Infer this from the user's location.".to_string(),
                ),
                enum_values: Some(vec!["celsius".to_string(), "fahrenheit".to_string()]),
                items: None,
            };

            let mut props = HashMap::new();
            props.insert("location".to_string(), location_prop);
            props.insert("unit".to_string(), unit_prop);

            Parameters {
                r#type: "object".to_string(),
                properties: props,
                required: Some(vec!["location".to_string()]),
            }
        }
    }

    #[async_trait]
    impl FunTool for MyTool {
        fn name(&self) -> String {
            "get_weather".into()
        }
        fn description(&self) -> String {
            "Get the current weather in a given location".into()
        }
        fn parameters(&self) -> Parameters {
            MyToolParameters::parameters()
        }
        async fn call(&self, input: Value) -> Result<String> {
            info!("Calling weather tool with input: {}", input);
            Ok("現在の東京の天気は晴れ、気温は25度です。".into())
        }
    }

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn test_agent_chat_with_tools_mocks<'a>(server: &'a MockServer) -> (Mock<'a>, Mock<'a>) {
        let response1 = r#"
{
  "id": "chatcmpl-example",
  "object": "chat.completion",
  "created": 1627034289,
  "model": "gpt-3.5-turbo-0613",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_abc123",
            "type": "function",
            "function": {
              "name": "get_weather",
              "arguments": "{\n \"location\": \"tokyo\",\n \"unit\": \"celsius\"\n}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ],
  "usage": {
    "prompt_tokens": 80,
    "completion_tokens": 30,
    "total_tokens": 110
  }
}
            "#;
        let response2 = r#"{"choices":[{"finish_reason":"stop","index":0,"message":{"content":"現在の東京は晴れ、気温は25度です。","role":"assistant"}}],"created":1743601854,"model":"gemini-2.0-flash","object":"chat.completion","usage":{"completion_tokens":1527,"prompt_tokens":6,"total_tokens":1533}}"#;

        let mock1 = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .body_excludes("assistant")
                .body_includes("tools");
            then.status(200)
                .header("content-type", "text/json; charset=UTF-8")
                .body(response1);
        });
        let mock2 = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .body_includes("assistant")
                .body_includes("tools");
            then.status(200)
                .header("content-type", "text/json; charset=UTF-8")
                .body(response2);
        });

        (mock1, mock2)
    }

    fn test_request_messages_1() -> Value {
        json!(
          [
            {
              "role": "system",
              "content": "あなたは親切なアシスタントです。"
            },
            {
              "role": "user",
              "content": "現在の東京の天気を調べてください。"
            }
          ]
        )
    }

    fn test_request_messages_2() -> Value {
        json!(
            [
                {
                  "role": "system",
                  "content": "あなたは親切なアシスタントです。"
                },
                {
                    "role": "user",
                    "content": "現在の東京の天気を調べてください。"
                },
                {
                    "role": "assistant",
                    "content": "tool called",
                    "tool_calls": [
                {
                    "id": "call_abc123",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\n \"location\": \"tokyo\",\n \"unit\": \"celsius\"\n}"
                    }
                }
                    ]
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_abc123",
                    "content": "現在の東京の天気は晴れ、気温は25度です。"
                }
            ]
        )
    }

    // RUST_LOG=debug cargo test test_agent_chat_with_tools -- --nocapture
    #[tokio::test]
    async fn test_agent_chat_with_tools() -> Result<()> {
        init_logger();

        let server = MockServer::start();
        let (mock1, mock2) = test_agent_chat_with_tools_mocks(&server);

        let tool_json = MyTool.to_openai_tool();

        let messages_1 = Messages::builder()
            .add_human_message("現在の東京の天気を調べてください。")
            .add_tools(vec![tool_json])
            .build();

        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key")
            .with_api_base(&server.url(""))
            .build()?;

        let my_tool = MyTool {};
        let gemini = Gemini::new(config);
        let agent = LLMAgent::builder(gemini)
            .with_system_prompt("あなたは親切なアシスタントです。")
            .with_tool(my_tool)
            .build()?;
        // llmへのリクエストとレスポンスのペア
        let results = agent.chat("現在の東京の天気を調べてください。").await?;
        mock1.assert(); // check called mock 1
        mock2.assert(); // check called mock 2

        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].request.messages[1].content, // 0 is system_prompt
            messages_1.messages[0].content
        );

        assert_eq!(
            results[0].request.to_json_value(),
            test_request_messages_1()
        );
        assert_eq!(
            results[1].request.to_json_value(),
            test_request_messages_2()
        );

        // 最初のレスポンスはツールコール
        match &results.get(0).unwrap().response {
            LLMResult::ToolCall(result) => {
                assert_eq!(result.name, "get_weather");
            }
            _ => assert!(false, "No results returned"),
        }

        // 2番目のレスポンスは、最初のユーザーへのレスポンス
        match &results.get(1).unwrap().response {
            LLMResult::Generate(tool_call) => {
                assert_eq!(tool_call.generation(), "現在の東京は晴れ、気温は25度です。");
            }
            LLMResult::ToolCall(tool_call) => {
                debug!("No results returned, {:?}", tool_call);
                assert!(false, "No generate")
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_invoke() -> Result<()> {
        init_logger();

        let server = mock_gemini_api(
            200,
            r#"{"choices":[{"finish_reason":"stop","index":0,"message":{"content":"こんにちは","role":"assistant"}}],"created":1743601854,"model":"gemini-2.0-flash","object":"chat.completion","usage":{"completion_tokens":1527,"prompt_tokens":6,"total_tokens":1533}}"#,
        );

        let messages_1 = Messages::builder()
            .add_human_message("現在の東京の天気を調べてください。")
            .build();

        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key")
            .with_api_base(&server.url(""))
            .build()?;

        let system_prompt = "あなたは親切なアシスタントです。";
        let my_tool = MyTool {};
        let gemini = Gemini::new(config);
        let agent = LLMAgent::builder(gemini)
            .with_system_prompt(system_prompt)
            .build()?;
        // llmへのリクエストとレスポンスのペア
        let messages = Messages::builder()
            .add_human_message("現在の東京の天気を調べてください。")
            .build();
        let results = agent.invoke(&messages).await?;

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].request.messages[0].content,
            Some(system_prompt.to_string())
        );

        assert_eq!(
            results[0].request.messages[1].content,
            messages_1.messages[0].content
        );

        Ok(())
    }

    fn mock_server_setup() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .body_excludes("assistant")
                .body_includes("tools");
            then.status(200)
                .header("content-type", "text/json; charset=UTF-8")
                .body(r#"{"choices":[{"finish_reason":"stop","index":0,"message":{"content":"晴れ","role":"assistant"}}],"created":1743601854,"model":"gemini-2.0-flash","object":"chat.completion","usage":{"completion_tokens":1527,"prompt_tokens":6,"total_tokens":1533}}"#);
        });
        server
    }

    fn setup_agent(server: MockServer) -> Result<LLMAgent<Gemini>> {
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key")
            .with_api_base(&server.url(""))
            .build()?;
        let gemini = Gemini::new(config);
        let agent = LLMAgent::builder(gemini)
            .with_system_prompt("あなたは親切なアシスタントです。")
            .build()?;
        Ok(agent)
    }

    fn test_message(message: &str) -> Messages {
        Messages::builder().add_human_message(message).build()
    }

    #[tokio::test]
    async fn test_agent_invoke_2() -> Result<()> {
        let server = mock_server_setup();
        let agent = setup_agent(server)?;
        let messages = test_message("現在の東京の天気を調べてください。");

        let result = agent.invoke(&messages).await?;
        assert_eq!("晴れ", result.final_answer);
        Ok(())
    }

    #[tokio::test]
    async fn test_agent_start() -> Result<()> {
        let server = mock_server_setup();
        let agent = setup_agent(server)?;
        let messages = test_message("現在の東京の天気を調べてください。");

        let stream = agent.start(&messages).await?;
        let action = stream.next().await;
        assert!(matches!(action, Some(NextAction::Request(_))));
        let action = stream.next().await;
        assert_eq!(action, Some(NextAction::ToolCall));
        let action = stream.next().await;
        assert_eq!(action, Some(NextAction::Request));
        let action = stream.next().await;
        assert_eq!(action, Some(NextAction::Response));

        Ok(())
    }
}
