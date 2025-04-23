mod mcp_agent;
use futures::Stream;
pub use mcp_agent::*;
use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
};

use fungraph_llm::{LLM, LLMError, LLMResult, Message, Messages, MessagesBuilder};
use log::debug;

use crate::tools::FunTool;

pub type Conversations = Vec<Conversation>;

#[derive(Debug)]
pub struct AgentResponse {
    pub final_answer: String,
    pub intermediate_steps: Vec<Conversation>,
}

pub struct AgentStream<'a, T: LLM> {
    agent: &'a LLMAgent<T>,
    next_action: Option<AgentAction>,
}

impl<'a, T: LLM> Stream for AgentStream<'a, T> {
    type Item = AgentAction;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.next_action.is_none() {
            let next_action = AgentAction::Request("現在の東京の天気を調べてください。".into());
            self.next_action = Some(next_action.clone());
            return Poll::Ready(Some(next_action));
        }
        Poll::Ready(Some(AgentAction::Response("晴れ".into())))
    }
}

/// ツール呼び出し要求
/// 普通の回答
/// LLM問い合わせ
#[derive(Debug, Clone)]
pub enum AgentAction {
    ToolCall,
    Response(String),
    Request(String),
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

    async fn start(&self, messages: &Messages) -> Result<AgentStream<'_, T>, LLMError> {
        Ok(AgentStream {
            agent: self,
            next_action: None,
        })
    }

    pub async fn invoke_chat(&self, user_message: &str) -> Result<LLMResult, LLMError> {
        let mut messages = Messages::builder().add_human_message(user_message).build();
        let messages = self.build_messages2(&mut messages);
        let result = self.llm.invoke(&messages).await?;
        Ok(result)
    }

    pub async fn invoke(&self, messages: &Messages) -> Result<AgentResponse, LLMError> {
        let mut messages = messages.clone();
        let mut messages = self.build_messages2(&mut messages);
        let result = self.llm.invoke(&messages).await?;
        let mut conversations = vec![Conversation {
            request: messages.clone(),
            response: result.clone(),
        }];

        let mut final_answer = "".to_string();
        match result {
            LLMResult::Generate(_generate_result) => {
                final_answer = _generate_result.generation().to_string()
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
        GenerateResult, TokenUsage,
        gemini::{Gemini, GeminiConfigBuilder, OpenAIMessages},
        openai::{Parameters, Property, Tool},
    };
    use futures::{stream, task::waker};
    use httpmock::{
        Method::POST, Mock, MockServer, server::matchers::readers::expectations::body_includes,
    };
    use log::{debug, info};
    use serde_json::{Value, json};
    use serial_test::serial;
    use tokio_stream::StreamExt;

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

        //assert_eq!(results.len(), 1);
        //assert_eq!(
        //    results[0].request.messages[0].content,
        //    Some(system_prompt.to_string())
        //);

        //assert_eq!(
        //    results[0].request.messages[1].content,
        //    messages_1.messages[0].content
        //);

        Ok(())
    }

    fn mock_server_setup(request_message: &str, response_message: &str) -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(200)
                .header("content-type", "text/json; charset=UTF-8")
                .body(format!(
                    r#"{{
        "choices": [
            {{
                "finish_reason": "stop",
                "index": 0,
                "message": {{
                    "content": "{}",
                    "role": "assistant"
                }}
            }}
        ],
        "created": 1743601854,
        "model": "gemini-2.0-flash",
        "object": "chat.completion",
        "usage": {{
            "completion_tokens": 1527,
            "prompt_tokens": 6,
            "total_tokens": 1533
        }}
    }}"#,
                    response_message
                ));
        });
        server
    }

    // cargo test agent::tests::test_agent_invoke_tool_call -- --exact --nocapture
    fn mock_toolcall_server_setup(request_message: &str, tool_args_str: &str) -> MockServer {
        let escaped_tool_args = tool_args_str.replace("\"", "\\\"");
        let server = MockServer::start();
        let response_body = format!(
            r#"
{{
  "choices": [
    {{
      "finish_reason": "tool_calls",
      "index": 0,
      "message": {{
        "content": null,
        "role": "assistant",
        "tool_calls": [
          {{
            "id": "call_abc123",
            "function": {{
              "arguments": "{}",
              "name": "get_weather"
            }},
            "type": "function"
          }}
        ]
      }}
    }}
  ],
  "created": 1743601854,
  "model": "gemini-2.0-flash",
  "object": "chat.completion",
  "usage": {{
    "completion_tokens": 1527,
    "prompt_tokens": 6,
    "total_tokens": 1533
  }}
}}
                    "#,
            escaped_tool_args
        );
        debug!("mock_toolcall_server_setup: {}", response_body);
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(200)
                .header("content-type", "text/json; charset=UTF-8")
                .body(response_body);
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

    // RUST_LOG=debug cargo test agent::tests::test_agent_invoke_2 -- --exact --nocapture
    #[tokio::test]
    #[serial]
    async fn test_agent_invoke_2() -> Result<()> {
        let request_message = "現在の東京の天気を調べてください。";
        let response_message = "晴れ";
        let server = mock_server_setup(request_message, response_message);
        let agent = setup_agent(server)?;
        let messages = test_message(request_message);
        let result = agent.invoke(&messages).await?;
        assert_eq!(response_message, result.final_answer);
        Ok(())
    }

    // RUST_LOG=debug cargo test agent::tests::test_agent_invoke_chat -- --exact --nocapture
    #[tokio::test]
    #[serial]
    async fn test_agent_invoke_chat() -> Result<()> {
        let request_message = "現在の東京の天気を調べてください。";
        let response_message = "晴れ";
        let server = mock_server_setup(request_message, response_message);
        let agent = setup_agent(server)?;
        let result = agent.invoke_chat(request_message).await?;
        if let LLMResult::Generate(result) = result {
            assert_eq!(response_message, result.generation());
        } else {
            assert!(false, "No results returned");
        }
        Ok(())
    }

    // RUST_LOG=debug cargo test agent::tests::test_agent_invoke_tool_call -- --exact --nocapture
    #[tokio::test]
    #[serial]
    async fn test_agent_invoke_tool_call() -> Result<()> {
        init_logger();
        let request_message = "現在の東京の天気を調べてください。";
        let tool_args = r#"{"location": "tokyo"}"#;

        let server = mock_toolcall_server_setup(request_message, tool_args);
        let agent = setup_agent(server)?;
        let result = agent.invoke_chat(request_message).await?;
        if let LLMResult::ToolCall(result) = result {
            assert_eq!("get_weather", result.name);
            assert_eq!(
                serde_json::from_str::<Value>(tool_args).unwrap(),
                result.arguments
            );
        } else {
            assert!(false, "No results returned");
        }
        Ok(())
    }

    async fn test_agent_start_simple(request_message: &str, response_message: &str) -> Result<()> {
        let server = mock_server_setup(request_message, response_message);
        let agent = setup_agent(server)?;
        let messages = test_message(request_message);
        let mut stream = agent.start(&messages).await?;
        let action = stream.next().await;
        assert!(
            matches!(action, Some(AgentAction::Request(message)) if message ==request_message.to_string())
        );
        assert!(
            matches!(stream.next_action.clone(), Some(AgentAction::Request(message)) if message == request_message.to_string())
        );
        let action = stream.next().await;
        assert!(matches!(
            action,
            Some(AgentAction::Response(message)) if message == response_message.to_string()
        ));
        Ok(())
    }

    // RUST_LOG=debug cargo test agent::tests::test_agent_start -- --exact --nocapture
    #[tokio::test]
    #[serial]
    async fn test_agent_start() -> Result<()> {
        test_agent_start_simple("現在の東京の天気を調べてください。", "晴れ").await?;
        test_agent_start_simple("hello request", "hello response").await?;
        Ok(())
    }
}
