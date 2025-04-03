use crate::types::openai::Parameters;

use super::ToolParameters;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::string::String;

#[async_trait]
pub trait Tool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Parameters;
    async fn call(&self, input: &Value) -> Result<String>;

    fn to_openai_tool(&self) -> crate::types::openai::Tool {
        crate::types::openai::Tool {
            r#type: crate::types::openai::ToolType::Function,
            function: crate::types::openai::FunctionDescription {
                name: self.name().into(),
                description: self.description().into(),
                parameters: self.parameters(),
            },
        }
    }
}

fn to_openai_tool<T: Tool>(tool: &T) -> crate::types::openai::Tool {
    crate::types::openai::Tool {
        r#type: crate::types::openai::ToolType::Function,
        function: crate::types::openai::FunctionDescription {
            name: tool.name().into(),
            description: tool.description().into(),
            parameters: tool.parameters(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::openai::{Parameters, Property};
    use serde_json::json;
    use std::collections::HashMap;

    struct MyTool;
    struct MyToolParameters {
        name: String,
    }

    impl ToolParameters for MyToolParameters {
        fn parameters() -> Parameters {
            let location_prop = Property {
                r#type: "string".to_string(),
                description: Some("The city and state, e.g. San Francisco, CA".to_string()),
                enum_values: None,
            };
            let unit_prop = Property {
                r#type: "string".to_string(),
                description: Some(
                    "The temperature unit to use. Infer this from the user's location.".to_string(),
                ),
                enum_values: Some(vec!["celsius".to_string(), "fahrenheit".to_string()]),
            };

            let mut props = HashMap::new();
            props.insert("location".to_string(), location_prop);
            props.insert("unit".to_string(), unit_prop);

            Parameters {
                r#type: "object".to_string(),
                properties: props,
                required: vec!["location".to_string()],
            }
        }
    }

    #[async_trait]
    impl Tool for MyTool {
        fn name(&self) -> &'static str {
            "get_weather"
        }
        fn description(&self) -> &'static str {
            "Get the current weather in a given location"
        }
        fn parameters(&self) -> Parameters {
            MyToolParameters::parameters()
        }
        async fn call(&self, input: &Value) -> Result<String> {
            Ok("test".into())
        }
    }

    fn test_expected_json_value() -> serde_json::Value {
        json!(
          {
            "type": "function",
            "function": {
              "name": "get_weather",
              "description": "Get the current weather in a given location",
              "parameters": {
                "type": "object",
                "properties": {
                  "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                  },
                  "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The temperature unit to use. Infer this from the user's location."
                  }
                },
                "required": ["location"]
              }
            }
          }
        )
    }

    #[test]
    fn test_tool_json() {
        let expected = test_expected_json_value();
        let my_tool = MyTool {};
        let openai_tool = to_openai_tool::<MyTool>(&my_tool);
        let result = serde_json::to_value(&openai_tool).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_tool_runner() {
        let my_tool = MyTool {};
    }
}
