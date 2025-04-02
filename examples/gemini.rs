use env_logger::init;
use fungraph::llm::{gemini::Gemini, llm::LLM};
use log::{debug, info};

// logを表示する
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    init();
    let api_key = dotenvy::var("GEMINI_API_KEY")?;
    //let gemini = Gemini::new(api_key);
    //let prompt = "Once upon a time";
    //let response = gemini.invoke(prompt).await?;
    Ok(())
}
