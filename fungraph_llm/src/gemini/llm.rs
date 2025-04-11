use std::{
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use log::{debug, warn};

use anyhow::Result;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    TokenUsage,
    openai::{
        ChatChoiceStream, ChatCompletionResponseStream, CreateChatCompletionStreamResponse,
        FinishReason,
    },
    {
        CallOptions, GenerateResult, LLM, LLMError, LLMResult, Message, MessageType, Messages,
        ToolCallResult,
        gemini::{GeminiResponse, OpenAIContent},
    },
};

use super::{GeminiConfig, GeminiRequest};

#[derive(Clone)]
pub struct Gemini {
    config: GeminiConfig,
    options: CallOptions,
}

impl Gemini {
    pub fn new(config: GeminiConfig) -> Self {
        Self {
            config,
            options: CallOptions::default(),
        }
    }

    pub fn with_options(mut self, options: CallOptions) -> Self {
        self.options = options;
        self
    }
}
// open ai互換のgeminiを使う
// https://developers.googleblog.com/en/gemini-is-now-accessible-from-the-openai-library/

#[async_trait]
impl LLM for Gemini {
    async fn generate(&self, prompt: &Messages) -> Result<LLMResult, LLMError> {
        let gemini_request = self.build_gemini_request_no_stream(prompt)?;
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.config.api_base());
        debug!("Gemini Request Url: {:?}", url);

        let response = client
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.config.api_key()))
            .body(serde_json::to_string(&gemini_request)?)
            .send()
            .await?;

        debug!("Gemini Response: {:?}", response);
        let status = response.status();
        let body_json = response.text().await?;
        debug!("Gemini Response Body: {:?}", body_json);

        if status.is_success() {
            let gemini_response: GeminiResponse = serde_json::from_str(&body_json)?;
            let mut generate_result = GenerateResult::default();
            let mut result = LLMResult::Generate(generate_result.clone());
            if let Some(choice) = gemini_response.choices.first() {
                let finish_reason = choice.finish_reason.unwrap();
                match finish_reason {
                    FinishReason::ToolCalls => {
                        let choice = choice.clone();
                        let name = choice
                            .message
                            .tool_calls
                            .clone()
                            .unwrap()
                            .first()
                            .unwrap()
                            .function
                            .clone()
                            .name
                            .to_string();
                        let arguments = serde_json::from_str(
                            &choice
                                .clone()
                                .message
                                .tool_calls
                                .unwrap()
                                .first()
                                .unwrap()
                                .function
                                .clone()
                                .arguments,
                        )
                        .unwrap();
                        let id = choice
                            .clone()
                            .message
                            .tool_calls
                            .unwrap()
                            .first()
                            .unwrap()
                            .id
                            .to_string();
                        let tool_calls =
                            serde_json::to_value(&choice.clone().message.tool_calls).unwrap();
                        result = LLMResult::ToolCall(ToolCallResult {
                            id,
                            name,
                            arguments,
                            ai_message: Message {
                                content: Some("tool called".into()),
                                message_type: MessageType::AIMessage,
                                id: None,
                                tool_calls: Some(tool_calls),
                                images: None,
                                name: None,
                            },
                        });
                    }
                    _ => {
                        choice.message.content.as_ref().map(|content| {
                            generate_result.set_generation(content);
                        });
                        result = LLMResult::Generate(generate_result);
                    }
                }
            }
            Ok(result)
        } else {
            Err(LLMError::OtherError(format!(
                "Gemini API error: {} - {}",
                status, body_json
            )))
        }
    }

    async fn invoke(&self, messages: &Messages) -> Result<LLMResult, LLMError> {
        self.generate(messages).await
    }

    async fn invoke_stream_one_result(&self, messages: &Messages) -> Result<LLMResult, LLMError> {
        debug!("message: {:?}", messages.messages);

        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.config.api_base());

        let request = self.build_gemini_stream_request(messages)?;

        let event_source = client
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.config.api_key()))
            .body(serde_json::to_string(&request)?)
            .eventsource()
            .unwrap();
        let mut original_stream: ChatCompletionResponseStream = stream(event_source).await;

        let mut tokens = None;
        let mut generation = String::new();
        while let Some(result) = original_stream.next().await {
            match result {
                Ok(response) => {
                    debug!("response: {:?}", response);
                    if let Some(usage) = response.usage {
                        tokens = Some(TokenUsage {
                            prompt_tokens: usage.prompt_tokens,
                            completion_tokens: usage.completion_tokens,
                            total_tokens: usage.total_tokens,
                        });
                    }
                    for chat_choice in response.choices.iter() {
                        let chat_choice: ChatChoiceStream = chat_choice.clone();

                        let finish_reason = chat_choice.finish_reason.unwrap();
                        match finish_reason {
                            FinishReason::ToolCalls => {
                                //if let Some(tool_calls) = chat_choice.delta.tool_calls {
                                //    let data = tool_calls.iter().for_each(|tool_call| {
                                //        let id = &tool_call.id;
                                //        let tool_call_type = &tool_call.r#type;
                                //        let function = &tool_call.function;
                                //        let index = &tool_call.index;
                                //    });
                                //}
                            }
                            _ => {
                                if let Some(content) = chat_choice.delta.content {
                                    generation.push_str(&content);
                                }
                            }
                        };
                    }
                }
                Err(err) => {
                    eprintln!("Error from streaming response: {:?}", err);
                }
            }
        }
        Ok(LLMResult::Generate(GenerateResult::new(generation, tokens)))
    }

    async fn invoke_stream(&self, messages: &Messages) -> Result<ChatStream, LLMError> {
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.config.api_base());

        let request = self.build_gemini_stream_request(messages)?;

        let event_source = client
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.config.api_key()))
            .body(serde_json::to_string(&request)?)
            .eventsource()
            .unwrap();
        Ok(ChatStream::new(event_source))
    }

    fn add_options(&mut self, options: &CallOptions) {
        self.options.merge(options);
    }
}

pub(crate) async fn stream<O>(
    mut event_source: EventSource,
) -> Pin<Box<dyn Stream<Item = Result<O, LLMError>> + Send>>
where
    O: DeserializeOwned + std::marker::Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(ev) = event_source.next().await {
            match ev {
                Err(e) => {
                    if let Err(_e) = tx.send(Err(LLMError::OtherError(format!(
                        "Event source error: {}",
                        e.to_string()
                    )))) {
                        // rx dropped
                        break;
                    }
                }
                Ok(event) => match event {
                    Event::Message(message) => {
                        if message.data == "[DONE]" {
                            break;
                        }

                        let response = match serde_json::from_str::<O>(&message.data) {
                            Err(e) => Err(LLMError::OtherError(format!(
                                "serde_json error: {}",
                                e.to_string()
                            ))),
                            Ok(output) => Ok(output),
                        };

                        if let Err(_e) = tx.send(response) {
                            // rx dropped
                            break;
                        }
                    }
                    Event::Open => continue,
                },
            }
        }

        event_source.close();
    });

    Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
}

pub struct ChatStream {
    event_source: EventSource,
}

impl ChatStream {
    pub fn new(event_source: EventSource) -> Self {
        Self { event_source }
    }
}

impl Stream for ChatStream {
    type Item = Result<LLMResult, LLMError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        debug!("Polling for next event");
        match Pin::new(&mut self.event_source).poll_next(cx) {
            Poll::Ready(Some(ev)) => {
                debug!("Received event: {:?}", ev);
                match ev {
                    Err(e) => {
                        match e {
                            reqwest_eventsource::Error::StreamEnded => {
                                warn!("reqwest_eventsource::Error::StreamEnded: {:?}", e);
                                Poll::Ready(None) // ストリームを終了
                            }
                            _ => Poll::Ready(Some(Err(LLMError::from(e)))), // エラーを伝播
                        }
                    }
                    Ok(event) => match event {
                        Event::Message(message) => {
                            if message.data == "[DONE]" {
                                Poll::Ready(None)
                            } else {
                                let response = serde_json::from_str::<
                                    CreateChatCompletionStreamResponse,
                                >(&message.data);

                                debug!("response: {:?}", response);
                                let result = match response {
                                    Err(e) => Err(LLMError::from(e)),
                                    Ok(response) => {
                                        let mut tokens = None;
                                        if let Some(usage) = response.usage {
                                            tokens = Some(TokenUsage {
                                                prompt_tokens: usage.prompt_tokens,
                                                completion_tokens: usage.completion_tokens,
                                                total_tokens: usage.total_tokens,
                                            });
                                        }
                                        if let Some(choice) = response.choices.first() {
                                            if let Some(finish_reason) = &choice.finish_reason {
                                                match finish_reason {
                                                    FinishReason::ToolCalls => {
                                                        let choice = choice.clone();
                                                        let name = (&choice
                                                            .delta
                                                            .tool_calls
                                                            .clone()
                                                            .unwrap()
                                                            .first()
                                                            .unwrap()
                                                            .function
                                                            .clone()
                                                            .unwrap()
                                                            .name
                                                            .unwrap())
                                                            .to_string();
                                                        let arguments = serde_json::from_str(
                                                            &choice
                                                                .clone()
                                                                .delta
                                                                .tool_calls
                                                                .unwrap()
                                                                .first()
                                                                .unwrap()
                                                                .function
                                                                .clone()
                                                                .unwrap()
                                                                .arguments
                                                                .unwrap(),
                                                        )
                                                        .unwrap();
                                                        let tool_calls = serde_json::to_value(
                                                            &choice.clone().delta.tool_calls,
                                                        )
                                                        .unwrap();

                                                        Ok(LLMResult::ToolCall(ToolCallResult {
                                                            id: "".to_string(),
                                                            name,
                                                            arguments,
                                                            ai_message: Message {
                                                                content: Some("tool called".into()),
                                                                message_type:
                                                                    MessageType::AIMessage,
                                                                id: None,
                                                                tool_calls: Some(tool_calls),
                                                                images: None,
                                                                name: None,
                                                            },
                                                        }))
                                                    }
                                                    _ => {
                                                        // func a
                                                        if let Some(content) = &choice.delta.content
                                                        {
                                                            Ok(LLMResult::Generate(
                                                                GenerateResult::new(
                                                                    content.clone(),
                                                                    tokens,
                                                                ),
                                                            ))
                                                        } else {
                                                            Err(LLMError::OtherError(
                                                                "No content in response"
                                                                    .to_string(),
                                                            ))
                                                        }
                                                    }
                                                }
                                            } else {
                                                // func a
                                                if let Some(content) = &choice.delta.content {
                                                    Ok(LLMResult::Generate(GenerateResult::new(
                                                        content.clone(),
                                                        tokens,
                                                    )))
                                                } else {
                                                    Err(LLMError::OtherError(
                                                        "No content in response".to_string(),
                                                    ))
                                                }
                                            }
                                        } else {
                                            Err(LLMError::OtherError(
                                                "No choices in response".to_string(),
                                            ))
                                        }
                                    }
                                };
                                Poll::Ready(Some(result))
                            }
                        }
                        Event::Open => {
                            debug!("Received Event::Open, waiting for Event::Message");
                            cx.waker().wake_by_ref();
                            Poll::Pending
                        }
                    },
                }
            }
            Poll::Ready(None) => {
                debug!("EventSource completed");
                Poll::Ready(None)
            }
            Poll::Pending => {
                debug!("EventSource pending");
                Poll::Pending
            }
        }
    }
}

pub trait OpenAIMessages {
    fn to_openai_messages(&self) -> Vec<OpenAIContent>;
    fn to_json_value(&self) -> Value;
}

impl OpenAIMessages for Messages {
    fn to_openai_messages(&self) -> Vec<OpenAIContent> {
        let mut contents: Vec<OpenAIContent> = Vec::new();
        for message in self.messages.iter() {
            let role = match message.message_type {
                MessageType::AIMessage => "assistant",
                MessageType::HumanMessage => "user",
                MessageType::SystemMessage => "system",
                MessageType::ToolMessage => "tool",
            }
            .to_string();
            let tool_calls = message.tool_calls.clone();
            let gemini_message = OpenAIContent {
                content: message.content.clone(),
                role,
                tool_calls,
                tool_call_id: message.id.clone(),
            };
            contents.push(gemini_message);
        }
        contents
    }

    fn to_json_value(&self) -> Value {
        let contents: Vec<OpenAIContent> = self.to_openai_messages();
        serde_json::to_value(contents).unwrap()
    }
}

impl Gemini {
    fn build_gemini_request(
        &self,
        messages: &Messages,
        is_stream: bool,
    ) -> Result<GeminiRequest, LLMError> {
        let contents = messages.to_openai_messages();

        let tools = if messages.tools.is_empty() {
            None
        } else {
            Some(messages.tools.clone())
        };

        let tool_choice = if messages.tools.is_empty() {
            None
        } else {
            Some("auto".to_string())
        };

        let stream = if is_stream { Some(true) } else { None };

        let gemini_request = GeminiRequest {
            messages: contents,
            model: self.config.model().clone().into(),
            stream,
            tools,
            tool_choice,
        };
        debug!(
            "Gemini Request json: {:?}",
            serde_json::to_string(&gemini_request)?
        );
        Ok(gemini_request)
    }

    fn build_gemini_stream_request(&self, messages: &Messages) -> Result<GeminiRequest, LLMError> {
        self.build_gemini_request(messages, true)
    }

    fn build_gemini_request_no_stream(
        &self,
        messages: &Messages,
    ) -> Result<GeminiRequest, LLMError> {
        self.build_gemini_request(messages, false)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        LLM, LLMResult, Messages, MessagesBuilder,
        gemini::{Gemini, GeminiConfigBuilder, GeminiModel},
        types::openai::Tool,
    };

    use anyhow::Result;
    use futures::StreamExt;
    use httpmock::prelude::*;
    use log::debug;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn test_response() -> &'static str {
        r#"{"choices":[{"finish_reason":"stop","index":0,"message":{"content":"こんにちは世界","role":"assistant"}}],"created":1743601854,"model":"gemini-2.0-flash","object":"chat.completion","usage":{"completion_tokens":1527,"prompt_tokens":6,"total_tokens":1533}}"#
    }

    fn mock_gemini_api(status: u16, body: &str) -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(status)
                .header("content-type", "text/json; charset=UTF-8")
                .body(body);
        });
        server
    }

    fn mock_gemini_stream_api(status: u16, body: &str) -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(status)
                .header("Content-Type", "text/event-stream") // stream 用のヘッダー
                .body(body);
        });
        server
    }

    fn build_gemini(model: GeminiModel) -> Gemini {
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key")
            .with_api_base("http://localhost:8080")
            .with_model(model)
            .build()
            .unwrap();
        Gemini::new(config)
    }

    #[test]
    fn test_build_gemini_request() {
        let gemini = build_gemini(GeminiModel::Gemini20);
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Translate the following sentence to Japanese: Hello, world!")
            .build();
        let request = gemini.build_gemini_request_no_stream(&messages).unwrap();
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.model, "gemini-2.0-flash-001");
    }

    #[test]
    fn test_build_gemini_request_with_tools() {
        let gemini = build_gemini(GeminiModel::Gemini20);
        let tools = vec![Tool {
            r#type: crate::types::openai::ToolType::Function,
            function: crate::types::openai::FunctionDescription {
                name: "my_function".to_string(),
                description: "This is a test function".to_string(),
                parameters: crate::types::openai::Parameters {
                    r#type: "object".to_string(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            },
        }];
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Translate the following sentence to Japanese: Hello, world!")
            .add_tools(tools)
            .build();
        let request = gemini.build_gemini_request_no_stream(&messages).unwrap();
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.tools.unwrap().len(), 1);
        assert_eq!(request.tool_choice.unwrap(), "auto");
        assert_eq!(request.model, "gemini-2.0-flash-001");
    }

    // RUST_LOG=debug cargo test llm::gemini::tests::tests::test_invoke -- --nocapture --exact
    #[tokio::test]
    async fn test_invoke() -> Result<()> {
        // 1. ロガーを初期化します (RUST_LOG=debug 環境変数を設定すると、詳細なログが出力されます)
        init_logger();

        // Gemini API をモックします (実際の API は呼び出されません)
        let server = mock_gemini_api(200, test_response());

        // 2. Gemini の設定を構築します
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key") // APIキーを設定します。
            .with_api_base(&server.url("")) // モックサーバーの URLを使用します。テスト時以外は設定不要です。
            .build()?;

        // 3. Gemini クライアントを作成します
        let gemini = Gemini::new(config);

        // 4. メッセージを作成します
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Translate the following sentence to Japanese: Hello, world!")
            .build();

        // 5. Gemini API を呼び出します
        let result = gemini.invoke(&messages).await?;

        // 6. 結果を検証します
        match result {
            LLMResult::Generate(result) => {
                assert_eq!(result.generation(), "こんにちは世界");
            }
            _ => panic!("Expected Generate result"),
        }

        Ok(())
    }

    // RUST_LOG=debug cargo test llm::gemini::tests::tests::test_invoke_error -- --nocapture --exact
    #[tokio::test]
    async fn test_invoke_error() -> Result<()> {
        init_logger();
        let error_response = r#"
    {
        "error": {
            "code": 500,
            "message": "Internal Server Error",
            "status": "INTERNAL"
        }
    }
    "#;
        let server = mock_gemini_api(500, error_response);
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key")
            .with_api_base(&server.url(""))
            .build()?;
        let gemini = Gemini::new(config);
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Once upon a time")
            .build();
        let result = gemini.invoke(&messages).await;
        assert!(result.is_err());
        // エラーメッセージの内容を検証する場合は、以下のようにします
        // assert_eq!(result.unwrap_err().to_string(), "...");
        Ok(())
    }

    // RUST_LOG=debug cargo test llm::gemini::llm::tests::test_invoke_stream -- --exact
    #[tokio::test]
    async fn test_invoke_stream() -> Result<()> {
        init_logger();

        let body = r#"
data: {"choices":[{"delta":{"content":"hello"},"finish_reason":null,"index":0}],"created":1677667095,"model":"gpt-3.5-turbo-0301","object":"chat.completion.chunk"}

data: {"choices":[{"delta":{"content":" world"},"finish_reason":null,"index":0}],"created":1677667095,"model":"gpt-3.5-turbo-0301","object":"chat.completion.chunk"}

data: [DONE]
"#;

        let server = mock_gemini_stream_api(200, body);
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key") // APIキーを設定します。
            .with_api_base(&server.url("")) // モックサーバーの URLを使用します。テスト時以外は設定不要です。
            .build()?;

        let gemini = Gemini::new(config);
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Translate the following sentence to Japanese: Hello, world!")
            .build();
        let mut stream = gemini.invoke_stream(&messages).await?;

        let mut expected_values = vec!["hello", " world"];
        while let Some(result) = stream.next().await {
            let delta = result?;
            match delta {
                LLMResult::Generate(delta) => {
                    assert_eq!(delta.generation(), expected_values.remove(0));
                }
                _ => panic!("Expected Stream result"),
            }
        }
        assert!(expected_values.is_empty());

        Ok(())
    }

    // RUST_LOG=debug cargo test llm::gemini::llm::tests::test_invoke_stream_tool_calls
    #[tokio::test]
    async fn test_invoke_stream_tool_calls() -> Result<()> {
        init_logger();

        let body = r#"
data: {"choices":[{"delta":{"role":"assistant","tool_calls":[{"function":{"arguments":"{\"location\":\"Tokyo\"}","name":"get_current_weather"},"id":"","type":"function"}]},"finish_reason":"tool_calls","index":0}],"created":1743981505,"model":"gemini-2.0-flash","object":"chat.completion.chunk"}

data: [DONE]
"#;

        let server = mock_gemini_stream_api(200, body);
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key") // APIキーを設定します。
            .with_api_base(&server.url("")) // モックサーバーの URLを使用します。テスト時以外は設定不要です。
            .build()?;

        let gemini = Gemini::new(config);
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Translate the following sentence to Japanese: Hello, world!")
            .build();
        let mut stream = gemini.invoke_stream(&messages).await?;

        if let Some(result) = stream.next().await {
            let delta = result?;
            debug!("delta: {:?}", delta);
            match delta {
                LLMResult::ToolCall(delta) => {
                    assert_eq!(delta.name, "get_current_weather");
                }
                _ => assert!(false, "Expected Stream result"),
            }
        } else {
            assert!(false, "Expected Stream result");
        }

        Ok(())
    }
}
