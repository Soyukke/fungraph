# fungraph

fungraph aims to be a state transition framework incorporating LLMs (currently unimplemented).

## Current Features

*   **Gemini Support:** Currently supports the Gemini LLM.
*   **LLM-Powered Agent with Tools:** Provides an agent that integrates LLMs and tools using Gemini.

## Overview

fungraph is a Rust library designed to facilitate the creation of intelligent agents by combining the power of Large Language Models (LLMs) with custom tools. It currently focuses on integrating with the Gemini LLM and provides a flexible framework for defining and utilizing tools within an agent.

## Defining Tools with Macros

Tools can be easily defined using macros:

```rust
use anyhow::Result;
use async_trait::async_trait;
use fungraph::tools::ToolParameters;
use fungraph::types::openai::Parameters;
use fungraph::tools::Tool;
use log::debug;
use serde_json::Value;

struct WeatherTool;

#[derive(ToolParameters)]
struct WeatherToolParameters {
    /// The location for which to retrieve weather information. Example: "Tokyo"
    location: String,
}

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &'static str {
        "weather_tool"
    }

    fn description(&self) -> &'static str {
        "Retrieves the weather for a specified location. Example response: Sunny"
    }

    fn parameters(&self) -> Parameters {
        WeatherToolParameters::parameters()
    }

    async fn call(&self, input: &Value) -> Result<String> {
        debug!("Calling weather tool with input: {}", input);
        Ok("Sunny".into())
    }
}
```

This code snippet demonstrates how to define a `WeatherTool` using the `ToolParameters` derive macro. The `WeatherTool` retrieves weather information for a given location.

## Example Code

You can find an example of how to use tool calling with fungraph in the following file:

[`fungraph/examples/gemini_tool.rs`](fungraph/examples/gemini_tool.rs)

This example showcases how to create an agent with a tool and use it to answer questions.
