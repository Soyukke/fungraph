use std::env;

use env_logger::init;
use fungraph::llm::{gemini::Gemini, llm::LLM};
use log::{debug, info};
use petgraph::{Graph, visit::Bfs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //init();

    //dotenvy::dotenv()?;
    //let api_key = dotenvy::var("GEMINI_API_KEY")?;

    //let gemini = Gemini::new(api_key);
    //let prompt = "Once upon a time";
    //gemini.invoke(prompt).await?;

    //let mut graph = Graph::<&str, &str>::new();

    //let a = graph.add_node("A");
    //let b = graph.add_node("B");
    //let c = graph.add_node("C");

    //graph.add_edge(a, b, "Edge AB");
    //graph.add_edge(b, c, "Edge BC");

    //let mut bfs = Bfs::new(&graph, a);

    //while let Some(node) = bfs.next(&graph) {
    //    info!("Visited: {:?}", graph[node]);
    //}
    Ok(())
}
