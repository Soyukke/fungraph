use anyhow::Result;
use async_trait::async_trait;
use env_logger::init;
use fungraph::tools::ToolParameters;
use fungraph::{agent::LLMAgent, tools::FunTool};
use fungraph_llm::openai::Parameters;
use fungraph_llm::{
    gemini::{Gemini, GeminiConfigBuilder},
    openai::Tool,
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
impl FunTool for WeatherTool {
    fn name(&self) -> String {
        "weather_tool".into()
    }

    fn description(&self) -> String {
        "指定した場所の天気を取得します。レスポンス例: 晴れ".into()
    }

    fn parameters(&self) -> Parameters {
        WeatherToolParameters::parameters()
    }

    async fn call(&self, input: Value) -> Result<String> {
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
