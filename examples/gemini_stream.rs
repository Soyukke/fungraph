use env_logger::init;
use fungraph::llm::{Message, gemini::Gemini, llm::LLM};
use log::{debug, info};

// cargo run --example gemini_stream
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    init();
    let api_key = dotenvy::var("GEMINI_API_KEY")?;
    //let gemini = Gemini::new(api_key);
    //let messages = [Message::new_human_message(
    //    "LLMの仕組みについて解説してください。",
    //)];
    //let response = gemini.invoke_stream(&messages).await?;
    Ok(())
}
