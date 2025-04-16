use fungraph_llm::openai::Parameters;

pub trait ToolParameters {
    fn parameters() -> Parameters;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fungraph_llm::openai::Property;
    use serde_json::json;

    use super::*;

    struct TestToolParam {}

    impl ToolParameters for TestToolParam {
        fn parameters() -> Parameters {
            let value1 = Property {
                r#type: "string".to_string(),
                description: Some("This is a string parameter".to_string()),
                enum_values: Some(vec!["value1".to_string(), "value2".to_string()]),
                ..Default::default()
            };
            let value2 = Property {
                r#type: "number".to_string(),
                description: Some("This is an integer parameter".to_string()),
                enum_values: Some(vec!["1".to_string(), "2".to_string()]),
                ..Default::default()
            };

            Parameters {
                r#type: "object".to_string(),
                properties: HashMap::from([
                    ("param1".to_string(), value1),
                    ("param2".to_string(), value2),
                ]),
                required: Some(vec!["param1".to_string()]),
            }
        }
    }

    #[test]
    fn test_parameters_json() {
        let tool_param = TestToolParam {};
        let json = serde_json::to_value(&TestToolParam::parameters()).unwrap();
        let json_value = json!(
            {
                "type":"object",
                "properties": {
                    "param1":{"type":"string","description":"This is a string parameter","enum":["value1","value2"]},
                    "param2":{"type":"number","description":"This is an integer parameter","enum":["1","2"]}
                },
                "required":["param1"]
            }
        );
        assert_eq!(json, json_value);
    }
}
