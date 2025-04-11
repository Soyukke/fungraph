use fungraph_llm::{LLM, MessagesBuilder};

// llmに入力し、出力する処理を実装する
pub struct SimpleLLM<T: LLM> {
    llm: T,
}

impl<T> SimpleLLM<T>
where
    T: LLM,
{
    pub async fn run(&self, message: &str) -> String {
        let messages = MessagesBuilder::new().add_human_message(message).build();
        self.llm.invoke(&messages).await.unwrap();
        "".to_string()
    }
}
