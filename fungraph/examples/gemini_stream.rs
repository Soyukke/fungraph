use env_logger::init;
use fungraph::llm::{
    LLM, LLMResult, Messages,
    gemini::{Gemini, GeminiConfigBuilder},
};
use log::{debug, info};
use tokio_stream::StreamExt;

// cargo run --example gemini_stream
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    init();
    let api_key = dotenvy::var("GEMINI_API_KEY")?;
    let gemini = Gemini::new(GeminiConfigBuilder::new().with_api_key(&api_key).build()?);
    let messages = Messages::builder()
        .add_human_message("LLMの仕組みについて解説してください。")
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
