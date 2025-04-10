use super::LLMAgentable;
use crate::llm::gemini::Gemini;

pub struct GeminiAgent {
    llm: Gemini,
}

impl GeminiAgent {
    pub fn new(llm: Gemini) -> Self {
        GeminiAgent { llm }
    }
}

impl LLMAgentable<Gemini> for GeminiAgent {
    fn get_name(&self) -> String {
        "GeminiAgent".to_string()
    }

    fn get_llm(&self) -> &Gemini {
        &self.llm
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use httpmock::{
        Method::POST, MockServer, server::matchers::readers::expectations::body_includes,
    };
    use log::{debug, info};
    use serde_json::{Value, json};

    use crate::{
        agent::LLMAgent,
        llm::{
            LLMResult, Messages,
            gemini::{GeminiConfigBuilder, OpenAIMessages},
        },
        tools::{Tool, ToolParameters},
        types::openai::{Parameters, Property},
    };

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
    async fn test_agent_init() -> Result<()> {
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key") // APIキーを設定します。
            .with_api_base("http://localhost:8080") // モックサーバーの URLを使用します。テスト時以外は設定不要です。
            .build()?;

        // 3. Gemini クライアントを作成します
        let gemini = Gemini::new(config);
        let agent = GeminiAgent::new(gemini);
        Ok(())
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
        let agent = GeminiAgent::new(gemini);
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
            };
            let unit_prop = Property {
                r#type: "string".to_string(),
                description: Some(
                    "The temperature unit to use. Infer this from the user's location.".to_string(),
                ),
                enum_values: Some(vec!["celsius".to_string(), "fahrenheit".to_string()]),
            };

            let mut props = HashMap::new();
            props.insert("location".to_string(), location_prop);
            props.insert("unit".to_string(), unit_prop);

            Parameters {
                r#type: "object".to_string(),
                properties: props,
                required: vec!["location".to_string()],
            }
        }
    }

    #[async_trait]
    impl Tool for MyTool {
        fn name(&self) -> &'static str {
            "get_weather"
        }
        fn description(&self) -> &'static str {
            "Get the current weather in a given location"
        }
        fn parameters(&self) -> Parameters {
            MyToolParameters::parameters()
        }
        async fn call(&self, input: &Value) -> Result<String> {
            info!("Calling weather tool with input: {}", input);
            Ok("現在の東京の天気は晴れ、気温は25度です。".into())
        }
    }

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    // RUST_LOG=debug cargo test test_agent_chat_with_tools -- --nocapture
    #[tokio::test]
    async fn test_agent_chat_with_tools() -> Result<()> {
        init_logger();

        let req_1 = r#"{\"messages\":[{\"role\":\"user\",\"content\":\"現在の東京の天気を調べてください。\"}],\"model\":\"gemini-1.5-flash\"}"#;
        let request_messages = r#"
{
  "model": "gpt-3.5-turbo-0613",
  "messages": [
    {
      "role": "user",
      "content": "現在の東京の天気を調べてください。"
    },
    {
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
    {
      "role": "tool",
      "tool_call_id": "call_abc123",
      "content": "現在の東京の天気は晴れ、気温は25度です。"
    }
  ]
}
        "#;

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

        let server = MockServer::start();
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

        let tool_json = MyTool.to_openai_tool();

        let messages_1 = Messages::builder()
            .add_human_message("現在の東京の天気を調べてください。")
            .add_tools(vec![tool_json])
            .build();

        // 最初のメッセージ、ユーザーのリクエスト
        let req_messages_1 = json!(
          [
            {
              "role": "user",
              "content": "現在の東京の天気を調べてください。"
            }
          ]
        );

        // AIレスポンス, ツール結果を付与したメッセージ
        let req_messages_2 = json!(
            [
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
        );

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
        mock1.assert();
        mock2.assert();

        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].request.messages[0].content,
            messages_1.messages[0].content
        );

        assert_eq!(results[0].request.to_json_value(), req_messages_1);
        assert_eq!(results[1].request.to_json_value(), req_messages_2);

        // TODO: ツールを含めたリクエストであるかテストする

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
}
