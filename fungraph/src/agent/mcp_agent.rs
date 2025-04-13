use std::{collections::HashMap, io, path::Path, process::Stdio};

use anyhow::Result;
use async_trait::async_trait;
use fungraph_llm::{LLM, gemini::Gemini};
use rmcp::{RoleClient, ServiceExt, service::RunningService};
use serde::{Deserialize, Serialize};

use crate::{
    agent::LLMAgent,
    node::{FunGraph, FunNode, FunState},
    tools::{FunTool, mcp_tool::get_mcp_tools},
};

pub struct MCPAgent<T: LLM> {
    agent: LLMAgent<T>,
}

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
            let result = self.agent.chat(user_input).await.unwrap();
            println!("LLM response: {:?}", result);
        } else {
            println!("No user input provided.");
            return;
        }
    }
}

struct MCPAgentState {
    pub user_input: Option<String>,
}
impl FunState for MCPAgentState {}

impl<T> MCPAgent<T>
where
    T: LLM,
{
    pub fn system_prompt(&self) -> String {
        let prompt = self.agent.system_prompt.clone().unwrap_or("".into());
        prompt
    }

    pub fn tools(&self) -> Vec<Box<&dyn FunTool>> {
        vec![]
    }

    pub fn builder(llm: T) -> MCPAgentBuilder<T> {
        MCPAgentBuilder {
            llm,
            system_prompt: None,
            mcp_config_path: None,
        }
    }
}

impl<T> Into<LLMAgent<T>> for MCPAgent<T>
where
    T: LLM,
{
    fn into(self) -> LLMAgent<T> {
        self.agent
    }
}

pub struct MCPAgentBuilder<T: LLM> {
    llm: T,
    system_prompt: Option<String>,
    mcp_config_path: Option<String>,
}

impl<T> MCPAgentBuilder<T>
where
    T: LLM,
{
    pub fn with_system_prompt(mut self, system_prompt: &str) -> Self {
        self.system_prompt = Some(system_prompt.to_string());
        self
    }
    pub fn with_mcp_config_path(mut self, mcp_config_path: &str) -> Self {
        self.mcp_config_path = Some(mcp_config_path.to_string());
        self
    }
    pub async fn build(self) -> Result<MCPAgent<T>> {
        let config_path = self
            .mcp_config_path
            .unwrap_or_else(|| "config.toml".to_string());

        // load config
        let config = Config::load(config_path).await?;
        println!("config: {:?}", config);

        // load mcp
        if config.mcp.is_some() {
            let mcp_clients = config.create_mcp_clients().await?;

            for (name, client) in mcp_clients {
                println!("loading mcp tools: {}", name);
                let server = client.peer().clone();
                let tools = get_mcp_tools(server).await?;

                for tool in tools {
                    println!("adding tool name: {}", tool.name());
                    println!("description: {:?}", tool.description());
                    println!("parameters: {:?}", tool.parameters());
                    println!("\n");
                }
            }
        }

        let mut builder = LLMAgent::builder(self.llm);
        let system_prompt = if let Some(system_prompt) = self.system_prompt {
            system_prompt
        } else {
            r#"
        あなたは様々なツールを利用してユーザーの要望を叶えるエージェントです。
        - ユーザーの要望に対して、必要なツールを選択し、適切な引数を指定して実行してください。
        - ユーザーの要望が不明瞭な場合は、質問をして明確にしてください。
        - ユーザーの要望が実行不可能な場合は、実行せずにその旨を伝えてください。また、代替案を提案してください。
        "#.into()
        };
        builder = builder.with_system_prompt(&system_prompt);
        let agent = builder.build()?;
        Ok(MCPAgent { agent })
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub mcp: Option<McpConfig>,
    pub model_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpConfig {
    pub server: Vec<McpServerConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(flatten)]
    pub transport: McpServerTransportConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum McpServerTransportConfig {
    Sse {
        url: String,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        envs: HashMap<String, String>,
    },
}

impl McpServerTransportConfig {
    pub async fn start(&self) -> Result<RunningService<RoleClient, ()>> {
        println!("Starting mcp server transport: {:?}", self);
        let client = match self {
            McpServerTransportConfig::Sse { url } => {
                println!("Starting SSE transport with URL: {}", url);
                let transport = rmcp::transport::sse::SseTransport::start(url).await?;
                ().serve(transport).await?
            }
            McpServerTransportConfig::Stdio {
                command,
                args,
                envs,
            } => {
                let transport = rmcp::transport::child_process::TokioChildProcess::new(
                    tokio::process::Command::new(command)
                        .args(args)
                        .envs(envs)
                        .stderr(Stdio::inherit())
                        .stdout(Stdio::inherit()),
                )?;
                ().serve(transport).await?
            }
        };
        println!("Mcp server started");
        Ok(client)
    }
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub async fn create_mcp_clients(
        &self,
    ) -> Result<HashMap<String, RunningService<RoleClient, ()>>> {
        let mut clients = HashMap::new();

        if let Some(mcp_config) = &self.mcp {
            for server in &mcp_config.server {
                println!("Loading mcp server: {}", server.name);
                let client = server.transport.start().await?;
                println!("Mcp server started: {}", server.name);
                clients.insert(server.name.clone(), client);
            }
        }

        Ok(clients)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use fungraph_llm::gemini::{Gemini, GeminiConfigBuilder, llm};

    #[tokio::test]
    async fn test_mcp_agent_init() -> Result<()> {
        let api_key = "test";
        let llm = Gemini::new(GeminiConfigBuilder::new().with_api_key(&api_key).build()?);
        let agent = MCPAgent::builder(llm)
            .with_system_prompt("test prompt")
            .with_mcp_config_path("examples/use_mcp/src/config2.toml")
            .build()
            .await?;
        assert_eq!(agent.system_prompt(), "test prompt");
        assert_eq!(agent.tools().len(), 0);
        Ok(())
    }

    //#[tokio::test]
    //async fn test_mcp_agent_run() -> Result<()> {
    //    let api_key = "test";
    //    let llm = Gemini::new(GeminiConfigBuilder::new().with_api_key(&api_key).build()?);
    //    // FunGraph wrapper
    //    let agent = MCPAgent::builder(llm)
    //        .with_system_prompt("test prompt")
    //        .with_mcp_config_path("examples/use_mcp/src/config2.toml")
    //        .build()
    //        .await?;
    //    let agent = agent.run().await;
    //    agent_iter.next().await;
    //    assert_eq!(agent.current_node_name(), "User Input");
    //    Ok(())
    //}
}
