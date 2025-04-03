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

use crate::{
    llm::{
        CallOptions, GenerateResult, LLM, LLMError, Message, MessageType, Messages,
        gemini::{GeminiResponse, OpenAIContent},
    },
    types::{
        ChatChoiceStream, ChatCompletionResponseStream, CreateChatCompletionStreamResponse,
        TokenUsage,
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
    async fn generate(&self, prompt: &[Message]) -> Result<GenerateResult, LLMError> {
        let gemini_request = self.build_gemini_request(prompt)?;
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
            if let Some(choice) = gemini_response.choices.first() {
                choice.message.content.as_ref().map(|content| {
                    generate_result.set_generation(content);
                });
            }
            Ok(generate_result)
        } else {
            Err(LLMError::OtherError(format!(
                "Gemini API error: {} - {}",
                status, body_json
            )))
        }
    }

    async fn invoke(&self, messages: &Messages) -> Result<GenerateResult, LLMError> {
        self.generate(messages.as_ref()).await
    }

    async fn invoke_stream_one_result(
        &self,
        messages: &[Message],
    ) -> Result<GenerateResult, LLMError> {
        debug!("message: {:?}", messages);

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

                        if let Some(content) = chat_choice.delta.content {
                            generation.push_str(&content);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error from streaming response: {:?}", err);
                }
            }
        }

        Ok(GenerateResult::new(generation, tokens))
    }

    async fn invoke_stream(&self, messages: &Messages) -> Result<ChatStream, LLMError> {
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.config.api_base());

        let request = self.build_gemini_stream_request(messages.as_ref())?;

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
    type Item = Result<GenerateResult, LLMError>;

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
                                        let mut generation = String::new();
                                        for choice in response.choices.iter() {
                                            if let Some(content) = &choice.delta.content {
                                                generation.push_str(content);
                                            }
                                        }
                                        Ok(GenerateResult::new(generation, tokens))
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

impl Gemini {
    fn build_gemini_request(&self, messages: &[Message]) -> Result<GeminiRequest, LLMError> {
        let mut contents: Vec<OpenAIContent> = Vec::new();
        for message in messages {
            let role = match message.message_type {
                MessageType::AIMessage => "model",
                MessageType::HumanMessage => "user",
                MessageType::SystemMessage => "system",
                MessageType::ToolMessage => "tool",
            }
            .to_string();

            let gemini_message = OpenAIContent {
                content: message.content.clone(),
                role,
            };
            contents.push(gemini_message);
        }
        let gemini_request = GeminiRequest {
            messages: contents,
            model: self.config.model().clone().into(),
            stream: None,
        };
        debug!(
            "Gemini Request json: {:?}",
            serde_json::to_string(&gemini_request)?
        );
        Ok(gemini_request)
    }

    fn build_gemini_stream_request(&self, messages: &[Message]) -> Result<GeminiRequest, LLMError> {
        let mut contents: Vec<OpenAIContent> = Vec::new();
        for message in messages {
            let role = match message.message_type {
                MessageType::AIMessage => "model",
                MessageType::HumanMessage => "user",
                MessageType::SystemMessage => "system",
                MessageType::ToolMessage => "tool",
            }
            .to_string();

            let gemini_message = OpenAIContent {
                content: message.content.clone(),
                role,
            };
            contents.push(gemini_message);
        }
        let gemini_request = GeminiRequest {
            messages: contents,
            model: self.config.model().clone().into(),
            stream: Some(true),
        };
        debug!(
            "Gemini Request json: {:?}",
            serde_json::to_string(&gemini_request)?
        );
        Ok(gemini_request)
    }
}
