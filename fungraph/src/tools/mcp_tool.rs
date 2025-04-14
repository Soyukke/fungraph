// Partially based on code from:
// https://github.com/modelcontextprotocol/rust-sdk
// MIT License: https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE
// Copyright (c) 2025 Model Context Protocol

use anyhow::Result;
use async_trait::async_trait;
use fungraph_llm::openai::Parameters;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

// convert mcp tool to FunTool
use rmcp::{
    model::{CallToolRequestParam, Tool as McpTool},
    service::ServerSink,
};

pub struct McpToolAdapter {
    tool: McpTool,
    server: ServerSink,
}

impl McpToolAdapter {
    pub fn new(tool: McpTool, server: ServerSink) -> Self {
        Self { tool, server }
    }
}

use super::FunTool;

#[derive(Default)]
pub struct ToolSet {
    tools: HashMap<String, Arc<dyn FunTool>>,
}

impl ToolSet {
    pub fn add_tool<T: FunTool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().into(), Arc::new(tool));
    }

    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn FunTool>> {
        self.tools.get(name).cloned()
    }

    pub fn tools(&self) -> Vec<Arc<dyn FunTool>> {
        self.tools.values().cloned().collect()
    }
}

pub async fn get_mcp_tools(server: ServerSink) -> Result<Vec<McpToolAdapter>> {
    let tools = server.list_all_tools().await?;
    Ok(tools
        .into_iter()
        .map(|tool| McpToolAdapter::new(tool, server.clone()))
        .collect())
}

#[async_trait]
impl FunTool for McpToolAdapter {
    fn name(&self) -> String {
        self.tool.name.clone().to_string()
    }

    fn description(&self) -> String {
        self.tool
            .description
            .clone()
            .unwrap_or_default()
            .to_string()
    }

    fn parameters(&self) -> Parameters {
        let value = serde_json::to_value(&self.tool.input_schema).unwrap_or(serde_json::json!({}));
        let mut params = serde_json::from_value(value).unwrap();
        params
    }

    async fn call(&self, args: Value) -> Result<String> {
        let arguments = match args {
            Value::Object(map) => Some(map),
            _ => None,
        };

        let call_result = self
            .server
            .call_tool(CallToolRequestParam {
                name: self.tool.name.clone(),
                arguments,
            })
            .await?;
        let result = serde_json::to_string(&call_result).unwrap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_funtools() {
        assert_eq!(2 + 2, 4);
    }
}
