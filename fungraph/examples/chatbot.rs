use async_trait::async_trait;
use env_logger::init;
use fungraph::{
    llm::Message,
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

#[derive(Debug)]
struct OutputNode {}

/// 標準出力にメッセージを表示するステートノード
#[async_trait]
impl FunNode<ChatbotState> for OutputNode {
    fn get_name(&self) -> String {
        "OutputNode".to_string()
    }

    async fn run(&self, state: ChatbotState) -> ChatbotState {
        println!("出力: {}", state.message.clone().unwrap());
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
    pub fn new() -> Self {
        let input_node = InputNode {};
        let output_node = OutputNode {};

        let mut graph: FunGraph<ChatbotState> = FunGraph::new();
        let a = graph.add_node(input_node);
        let b = graph.add_node(output_node);
        graph.add_edge(a, b, "Edge AB".to_string());
        // 制約
        // conditionalじゃない場合はadd_edgeで同一fromで複数toを追加できないようにしたい。
        // conditionalの場合は複数toが設定できる

        //let mut graph = Graph::<Box<dyn FunNode<ChatbotState>>, String>::new();
        //let a = graph.add_node(Box::new(input_node));
        //let b = graph.add_node(Box::new(output_node));
        //graph.add_edge(a, b, "Edge AB".to_string());

        ChatBotAgent { graph }
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

    let agent = ChatBotAgent::new();
    agent.run().await;

    Ok(())
}
