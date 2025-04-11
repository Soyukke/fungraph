use anyhow::Result;
use async_trait::async_trait;
use env_logger::init;
use fungraph::tools::{FunTool, ToolParameters};
use fungraph_llm::{
    LLM, LLMResult, Messages,
    gemini::{Gemini, GeminiConfigBuilder},
    openai::Parameters,
};
use log::{debug, info};
use serde_json::Value;
use tokio_stream::StreamExt;

struct WeatherTool;

#[derive(ToolParameters)]
struct WeatherToolParameters {
    /// 天気を取得したい場所を指定します。例. "東京"
    location: String,
}

#[async_trait]
impl FunTool for WeatherTool {
    fn name(&self) -> &'static str {
        "weather_tool"
    }

    fn description(&self) -> &'static str {
        "指定した場所の天気を取得します。レスポンス例: 晴れ"
    }

    fn parameters(&self) -> Parameters {
        WeatherToolParameters::parameters()
    }

    async fn call(&self, input: &Value) -> Result<String> {
        debug!("Calling weather tool with input: {}", input);
        Ok("Sunny".into())
    }
}

// cargo run --example gemini_tool
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    init();
    let api_key = dotenvy::var("GEMINI_API_KEY")?;
    WeatherToolParameters::parameters();
    let gemini = Gemini::new(GeminiConfigBuilder::new().with_api_key(&api_key).build()?);
    let tool = WeatherTool {};
    let messages = Messages::builder()
        .add_human_message("今日の東京の天気は？")
        .add_tools(vec![tool.to_openai_tool()])
        .build();
    let mut response = gemini.invoke_stream(&messages).await?;

    while let Some(result) = response.next().await {
        match result {
            Ok(LLMResult::Generate(result)) => {
                debug!("Received generation: {}", result.generation());
            }
            Ok(LLMResult::ToolCall(tool_call)) => {
                debug!("Received tool call: {:?}", tool_call);
            }
            Err(e) => {
                info!("Error: {:?}", e);
            }
        }
    }
    Ok(())
}
