use anyhow::Result;
use async_trait::async_trait;
use env_logger::init;
use fungraph::{
    llm::{
        LLM, LLMResult, Message, Messages, error,
        gemini::{Gemini, GeminiConfigBuilder},
    },
    node::{FunGraph, FunGraphBuilder, FunNode, FunState},
    *,
};
use log::{debug, info};
use petgraph::{Graph, data::DataMap, graph::NodeIndex, visit::IntoNeighbors};
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

    async fn run(&self, state: ChatbotState) -> ChatbotState {
        // 標準入力からユーザーの入力を受け取る
        println!("何か入力してください:");
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input.trim();
        println!("入力された内容: {}", input);

        ChatbotState {
            message: Some(input.to_string()),
            histories: state.histories,
        }
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

    async fn run(&self, state: ChatbotState) -> ChatbotState {
        let message = state.message.clone().unwrap();
        let messages = Messages::builder().add_human_message(&message).build();
        let result = self.llm.invoke(&messages).await;

        match result {
            Ok(LLMResult::Generate(result)) => {
                info!("Received generation: {}", result.generation());
            }
            Ok(LLMResult::ToolCall(tool_call)) => {
                info!("Received tool call: {:?}", tool_call);
            }
            Err(e) => {
                log::error!("Error: {}", e);
            }
        }
        ChatbotState {
            message: state.message,
            histories: state.histories,
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
        let a = graph.add_node(input_node);
        let b = graph.add_node(output_node);
        graph.add_edge(a, b, "Edge AB".to_string());

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
