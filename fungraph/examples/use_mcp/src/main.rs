use anyhow::Result;
use fungraph::tools::FunTool;
use fungraph::tools::mcp_tool::get_mcp_tools;
use use_mcp::config::Config;

const DEFAULT_CONFIG_PATH: &str = "src/config.toml";

#[tokio::main]
async fn main() -> Result<()> {
    // load config
    let config = Config::load(DEFAULT_CONFIG_PATH).await?;
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

    Ok(())
}
