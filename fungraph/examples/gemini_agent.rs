use anyhow::Result;
use async_trait::async_trait;
use env_logger::init;
use fungraph::agent::LLMAgent;
use fungraph::tools::ToolParameters;
use fungraph::types::openai::Parameters;
use fungraph::{
    llm::gemini::{Gemini, GeminiConfigBuilder},
    tools::Tool,
};
use log::{debug, info};
use serde_json::Value;

struct WeatherTool;

#[derive(ToolParameters)]
struct WeatherToolParameters {
    /// 天気を取得したい場所を指定します。例. "東京"
    location: String,
}

#[async_trait]
impl Tool for WeatherTool {
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

    let agent = LLMAgent::builder(gemini)
        .with_system_prompt("あなたは親切なアシスタントです。")
        .with_tool(tool)
        .build()?;

    let results = agent.chat("今日の東京の天気は？").await?;

    for conversation in results {
        info!("Conversation: {:?}", conversation);
    }
    Ok(())
}
