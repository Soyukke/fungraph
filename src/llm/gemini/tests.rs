#[cfg(test)]
mod tests {
    use crate::llm::{
        LLM, Messages, MessagesBuilder,
        gemini::{Gemini, GeminiConfigBuilder},
    };

    use anyhow::Result;
    use httpmock::prelude::*;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn test_response() -> &'static str {
        r#"{"choices":[{"finish_reason":"stop","index":0,"message":{"content":"こんにちは世界","role":"assistant"}}],"created":1743601854,"model":"gemini-2.0-flash","object":"chat.completion","usage":{"completion_tokens":1527,"prompt_tokens":6,"total_tokens":1533}}"#
    }

    fn mock_gemini_api(status: u16, body: &str) -> MockServer {
        // Create a mock on the server.
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/chat/completions");
            then.status(status)
                .header("content-type", "text/html; charset=UTF-8")
                .body(body);
        });
        server
    }

    // RUST_LOG=debug cargo test llm::gemini::tests::tests::test_invoke -- --nocapture --exact
    #[tokio::test]
    async fn test_invoke() -> Result<()> {
        // 1. ロガーを初期化します (RUST_LOG=debug 環境変数を設定すると、詳細なログが出力されます)
        init_logger();

        // Gemini API をモックします (実際の API は呼び出されません)
        let server = mock_gemini_api(200, test_response());

        // 2. Gemini の設定を構築します
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key") // APIキーを設定します。
            .with_api_base(&server.url("")) // モックサーバーの URLを使用します。テスト時以外は設定不要です。
            .build()?;

        // 3. Gemini クライアントを作成します
        let gemini = Gemini::new(config);

        // 4. メッセージを作成します
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Translate the following sentence to Japanese: Hello, world!")
            .build();

        // 5. Gemini API を呼び出します
        let result = gemini.invoke(&messages).await?;

        // 6. 結果を検証します
        assert_eq!(result.generation().contains("こんにちは世界"), true);

        Ok(())
    }

    // RUST_LOG=debug cargo test llm::gemini::tests::tests::test_invoke_error -- --nocapture --exact
    #[tokio::test]
    async fn test_invoke_error() -> Result<()> {
        init_logger();
        let error_response = r#"
    {
        "error": {
            "code": 500,
            "message": "Internal Server Error",
            "status": "INTERNAL"
        }
    }
    "#;
        let server = mock_gemini_api(500, error_response);
        let config = GeminiConfigBuilder::new()
            .with_api_key("test_api_key")
            .with_api_base(&server.url(""))
            .build()?;
        let gemini = Gemini::new(config);
        let messages: Messages = MessagesBuilder::new()
            .add_human_message("Once upon a time")
            .build();
        let result = gemini.invoke(&messages).await;
        assert!(result.is_err());
        // エラーメッセージの内容を検証する場合は、以下のようにします
        // assert_eq!(result.unwrap_err().to_string(), "...");
        Ok(())
    }
}
