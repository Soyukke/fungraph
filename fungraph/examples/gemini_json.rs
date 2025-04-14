use env_logger::init;
use fungraph_llm::{
    LLM, LLMResult, Messages,
    gemini::{Gemini, GeminiConfigBuilder},
};
use log::{debug, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    init();
    let api_key = dotenvy::var("GEMINI_API_KEY")?;
    let gemini = Gemini::new(
        GeminiConfigBuilder::new()
            .with_api_key(&api_key)
            .with_json_response()
            .build()?,
    );
    let messages = Messages::builder()
        .add_system_message("メッセージをjson形式でレスポンスしてください。keys: message")
        .add_human_message("LLMの仕組みについて解説してください。")
        .build();
    let response = gemini.invoke(&messages).await?;

    match response {
        LLMResult::Generate(result) => {
            debug!("Received generation: {}", result.generation());
        }
        LLMResult::ToolCall(tool_call) => {
            debug!("Received tool call: {:?}", tool_call);
        }
        _ => {
            info!("Error");
        }
    }
    Ok(())
}
