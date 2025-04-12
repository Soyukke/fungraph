use fungraph::tools::ToolParameters;
use fungraph_llm::openai::Parameters;

#[derive(ToolParameters)]
struct MyTool {
    /// This is a test description.
    name: String,
    age: i32,
}

#[derive(ToolParameters)]
struct MyOptionTool {
    /// This is a test description.
    name: Option<String>,
    age: Option<i32>,
}

//#[derive(ToolParameters)]
//struct MyUnsupportTool {
//    /// This is a test description.
//    name: Vec<String>,
//    age: i32,
//}

#[test]
fn test_generated_parameters() {
    let my_tool = MyTool {
        name: "test".to_string(),
        age: 30,
    };

    // need use `use fungraph::tools::ToolParameters;`
    let parameters: Parameters = MyTool::parameters();

    // r#type フィールドの検証
    assert_eq!(parameters.r#type, "object".to_string());

    // required フィールドの検証
    assert_eq!(
        parameters.required,
        Some(vec!["name".to_string(), "age".to_string()])
    );

    // properties フィールドの検証
    assert_eq!(parameters.properties.len(), 2); // プロパティの数を確認

    // name プロパティの検証
    let name_property = parameters.properties.get("name").unwrap();
    assert_eq!(name_property.r#type, "string".to_string());
    assert_eq!(
        name_property.description,
        Some("This is a test description.".to_string())
    );
    assert_eq!(name_property.enum_values, None);

    // age プロパティの検証
    let age_property = parameters.properties.get("age").unwrap();
    assert_eq!(age_property.r#type, "number".to_string());
    assert_eq!(age_property.description, None);
    assert_eq!(age_property.enum_values, None);
}

#[test]
fn test_option_parameter() {
    let my_tool = MyOptionTool {
        name: Some("test".to_string()),
        age: Some(30),
    };
    let parameters: Parameters = MyOptionTool::parameters();
    // r#type フィールドの検証
    assert_eq!(parameters.r#type, "object".to_string());
    // required フィールドの検証
    assert_eq!(
        parameters.required,
        Some(vec!["name".to_string(), "age".to_string()])
    );
    // properties フィールドの検証
    assert_eq!(parameters.properties.len(), 2); // プロパティの数を確認
    // name プロパティの検証
    let name_property = parameters.properties.get("name").unwrap();
    assert_eq!(name_property.r#type, "string".to_string());
    assert_eq!(
        name_property.description,
        Some("This is a test description.".to_string())
    );
    assert_eq!(name_property.enum_values, None);
    // age プロパティの検証
    let age_property = parameters.properties.get("age").unwrap();
    assert_eq!(age_property.r#type, "number".to_string());
    assert_eq!(age_property.description, None);
}
