use anyhow::Result;

#[derive(Clone, Debug, PartialEq)]
pub enum GeminiModel {
    Gemini15,
    Gemini20,
}

impl ToString for GeminiModel {
    fn to_string(&self) -> String {
        match self {
            GeminiModel::Gemini15 => "gemini-1.5-flash".to_string(),
            GeminiModel::Gemini20 => "gemini-2.0-flash-001".to_string(),
        }
    }
}

impl Into<String> for GeminiModel {
    fn into(self) -> String {
        self.to_string()
    }
}

#[derive(Clone)]
pub struct GeminiConfig {
    api_base: String,
    api_key: String,
    model: GeminiModel,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_base: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
            api_key: "".to_string(),
            model: GeminiModel::Gemini15,
        }
    }
}

impl GeminiConfig {
    pub fn api_base(&self) -> &str {
        &self.api_base
    }
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
    pub fn model(&self) -> &GeminiModel {
        &self.model
    }
}

pub struct GeminiConfigBuilder {
    config: GeminiConfig,
}

impl GeminiConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: GeminiConfig::default(),
        }
    }
    pub fn with_api_base(mut self, api_base: &str) -> Self {
        self.config.api_base = api_base.into();
        self
    }
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.config.api_key = api_key.into();
        self
    }
    pub fn with_model(mut self, model: GeminiModel) -> Self {
        self.config.model = model;
        self
    }
    pub fn build(self) -> Result<GeminiConfig> {
        if self.config.api_key.is_empty() {
            anyhow::bail!("API key must be set");
        }

        Ok(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use env_logger::init;
    use httpmock::prelude::*;

    // cargo test --lib gemini::tests::test_gemini_config_builder_api_key_empty
    #[test]
    fn test_gemini_config_builder_api_key_empty() {
        let result = GeminiConfigBuilder::new().build();
        match result {
            Ok(_) => assert!(false),
            Err(err) => assert_eq!(err.to_string(), "API key must be set"),
        }
    }

    // cargo test --lib gemini::config::tests::test_gemini_config_builder_all_fields
    #[test]
    fn test_gemini_config_builder_all_fields() {
        let config = GeminiConfigBuilder::new()
            .with_api_base("https://example.com")
            .with_api_key("test_api_key")
            .with_model(GeminiModel::Gemini20)
            .build()
            .unwrap();
        assert_eq!(config.api_base, "https://example.com");
        assert_eq!(config.api_key, "test_api_key");
        assert_eq!(config.model, GeminiModel::Gemini20);
    }

    // cargo test --lib gemini::tests::test_httpmock -- --nocapture
    #[tokio::test]
    async fn test_httpmock() -> Result<()> {
        dotenvy::dotenv()?;
        init();
        // Start a lightweight mock server.
        let server = MockServer::start();

        // Create a mock on the server.
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/translate")
                .query_param("word", "hello");
            then.status(200)
                .header("content-type", "text/html; charset=UTF-8")
                .body("hola");
        });

        // Send an HTTP request to the mock server. This simulates your code.
        let client = reqwest::Client::new();

        let response = client
            .get(server.url("/translate?word=hello"))
            .send()
            .await?;

        // Ensure the specified mock was called exactly one time (or fail with a
        // detailed error description).
        mock.assert();

        // Ensure the mock server did respond as specified.
        assert_eq!(response.status(), 200);
        assert_eq!(response.text().await?, "hola");
        Ok(())
    }
}
