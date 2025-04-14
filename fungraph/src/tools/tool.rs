use anyhow::Result;
use async_trait::async_trait;
use fungraph_llm::openai::{FunctionDescription, Parameters, Tool, ToolType};
use serde_json::Value;
use std::string::String;

#[async_trait]
pub trait FunTool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn parameters(&self) -> Parameters;
    async fn call(&self, input: Value) -> Result<String>;

    fn to_openai_tool(&self) -> Tool {
        Tool {
            r#type: ToolType::Function,
            function: FunctionDescription {
                name: self.name().into(),
                description: self.description().into(),
                parameters: self.parameters(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::ToolParameters;

    use super::*;
    use fungraph_llm::openai::Property;
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
                items: None,
            };
            let unit_prop = Property {
                r#type: "string".to_string(),
                description: Some(
                    "The temperature unit to use. Infer this from the user's location.".to_string(),
                ),
                enum_values: Some(vec!["celsius".to_string(), "fahrenheit".to_string()]),
                items: None,
            };

            let mut props = HashMap::new();
            props.insert("location".to_string(), location_prop);
            props.insert("unit".to_string(), unit_prop);

            Parameters {
                r#type: "object".to_string(),
                properties: props,
                required: Some(vec!["location".to_string()]),
            }
        }
    }

    #[async_trait]
    impl FunTool for MyTool {
        fn name(&self) -> String {
            "get_weather".into()
        }
        fn description(&self) -> String {
            "Get the current weather in a given location".into()
        }
        fn parameters(&self) -> Parameters {
            MyToolParameters::parameters()
        }
        async fn call(&self, input: Value) -> Result<String> {
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
        let openai_tool = my_tool.to_openai_tool();
        let result = serde_json::to_value(&openai_tool).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_tool_runner() {
        let my_tool = MyTool {};
    }
}
