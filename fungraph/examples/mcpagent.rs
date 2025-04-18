use std::io;

use async_trait::async_trait;
use env_logger::init;
use fungraph::{
    agent::{LLMAgent, MCPAgent},
    node::{FunGraph, FunNode, FunState},
};
use fungraph_llm::{
    LLM, LLMResult, Message, Messages,
    gemini::{Gemini, GeminiConfigBuilder},
};
use log::debug;

#[derive(Debug)]
struct InputNode {}

#[async_trait]
impl FunNode<MCPAgentState> for InputNode {
    fn get_name(&self) -> &'static str {
        "UserInput"
    }

    async fn run(&self, state: &mut MCPAgentState) {
        println!("Please type your message:");
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input;
        state.user_input = Some(input.trim().to_string());
    }
}

struct ResolverNode<T: LLM> {
    agent: LLMAgent<T>,
}

#[async_trait]
impl<T> FunNode<MCPAgentState> for ResolverNode<T>
where
    T: LLM,
{
    fn get_name(&self) -> &'static str {
        "Resolver"
    }

    async fn run(&self, state: &mut MCPAgentState) {
        if let Some(user_input) = &state.user_input {
            let mut messages = state.histories.clone();
            messages.add_message(Message::new_human_message(user_input));

            state.histories = messages.clone();

            let result = self.agent.invoke(&messages).await.unwrap();
            println!("LLM: {}", result.final_answer);
        } else {
            println!("No user input provided.");
            return;
        }
    }
}

struct MCPAgentState {
    pub user_input: Option<String>,
    pub histories: Messages,
}
impl FunState for MCPAgentState {}

fn build_graph<T: LLM + 'static>(agent: LLMAgent<T>) -> FunGraph<MCPAgentState> {
    let input_node = InputNode {};
    let resolver_node = ResolverNode { agent: agent };
    let mut graph = FunGraph::new();
    let input_node_index = graph.add_node(input_node);
    let resolver_node_index = graph.add_node(resolver_node);
    graph.add_start_edge(input_node_index);
    graph.add_edge(input_node_index, resolver_node_index);
    graph.add_edge(resolver_node_index, input_node_index);
    graph
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    init();
    debug!("Starting chatbot example");
    let api_key = dotenvy::var("GEMINI_API_KEY")?;
    let llm = Gemini::new(GeminiConfigBuilder::new().with_api_key(&api_key).build()?);
    let agent = MCPAgent::builder(llm)
        .with_mcp_config_path("fungraph/examples/use_mcp/src/config.toml")
        .build()
        .await?;
    let graph = build_graph(agent.into());
    let initial_state = MCPAgentState {
        user_input: None,
        histories: Messages::builder().build(),
    };
    graph.run(initial_state).await;

    Ok(())
}
