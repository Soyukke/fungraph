use anyhow::Result;
use async_trait::async_trait;
use env_logger::init;
use fungraph::{
    llm::{
        LLM, LLMResult, Messages,
        gemini::{Gemini, GeminiConfigBuilder},
    },
    node::{EndFunNode, FunGraph, FunNode, FunState, StartFunNode},
};
use log::{debug, info};
use std::io;

#[derive(Debug)]
struct ChatbotState {
    pub message: Option<String>,
    pub histories: Vec<String>,
}

impl FunState for ChatbotState {}

#[derive(Debug)]
struct InputNode {}

/// ユーザーからの入力を受け取るステートノード
#[async_trait]
impl FunNode<ChatbotState> for InputNode {
    fn get_name(&self) -> String {
        "InputNode".to_string()
    }

    async fn run(&self, state: &mut ChatbotState) {
        // 標準入力からユーザーの入力を受け取る
        println!("Please type your message:");
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input.trim();

        state.message = Some(input.to_string());
        state.histories.push(input.to_string());
    }
}

struct OutputNode {
    llm: Gemini,
}

impl OutputNode {
    fn new(api_key: &str) -> Result<Self> {
        let gemini = Gemini::new(GeminiConfigBuilder::new().with_api_key(&api_key).build()?);
        Ok(OutputNode { llm: gemini })
    }
}

/// llmが回答を標準出力にメッセージを表示するステートノード
#[async_trait]
impl FunNode<ChatbotState> for OutputNode {
    fn get_name(&self) -> String {
        "OutputNode".to_string()
    }

    async fn run(&self, state: &mut ChatbotState) {
        let message = state.message.clone().unwrap();
        let messages = Messages::builder().add_human_message(&message).build();
        let result = self.llm.invoke(&messages).await;

        match result {
            Ok(LLMResult::Generate(result)) => {
                debug!("Received generation: {}", result.generation());
                state.histories.push(result.generation().to_string());
                println!("LLM: {}", result.generation());
            }
            Ok(LLMResult::ToolCall(tool_call)) => {
                debug!("Received tool call: {:?}", tool_call);
            }
            Err(e) => {
                log::error!("Error: {}", e);
            }
        }
    }
}

struct ChatBotAgent {
    graph: FunGraph<ChatbotState>,
}

impl ChatBotAgent {
    pub fn new() -> Result<Self> {
        let input_node = InputNode {};
        let api_key = dotenvy::var("GEMINI_API_KEY")?;
        let output_node = OutputNode::new(&api_key)?;

        let mut graph: FunGraph<ChatbotState> = FunGraph::new();
        let start_node_index = graph.add_node(StartFunNode {});
        let end_node_index = graph.add_node(EndFunNode {});
        let input_node_index = graph.add_node(input_node);
        let llm_node_index = graph.add_node(output_node);
        //graph.add_edge(
        //    start_node_index,
        //    input_node_index,
        //    "Start -> User".to_string(),
        //);
        //graph.add_edge(input_node_index, llm_node_index, "User -> LLM".to_string());
        //graph.add_edge(llm_node_index, input_node_index, "LLM -> User".to_string());
        //graph.add_edge(input_node_index, end_node_index, "User -> End".to_string());

        Ok(ChatBotAgent { graph })
    }

    pub async fn run(&self) {
        let initial_state = ChatbotState {
            message: None,
            histories: vec![],
        };
        self.graph.run(initial_state).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    init();
    debug!("Starting chatbot example");

    let agent = ChatBotAgent::new()?;
    agent.run().await;

    Ok(())
}
