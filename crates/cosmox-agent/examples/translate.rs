use rig::{
    agent::Agent,
    client::CompletionClient,
    completion::Prompt,
    providers::llamafile::{self},
};

pub struct TranslatorAgent {
    agent: Agent<llamafile::CompletionModel>,
}
impl TranslatorAgent {
    pub fn new(base_url: &'static str, model: &'static str) -> Self {
        let client = llamafile::Client::from_url(base_url);
        let agent = client
    .agent(model)
    .preamble(
    r#"
    You are a fast translation engine. Do NOT think, do NOT reason, and do NOT output any chain of thought. Translate the input directly and output NOTHING else. Respond instantly.
    You are a professional translator.
    Only return the translated text without any explanations, quotes, or extra words.",
    "#)
    .build();
        Self { agent }
    }

    pub async fn translate(
        &self,
        plaintext: &'static str,
        language: &'static str,
    ) -> Result<String, rig::completion::PromptError> {
        self.agent
            .prompt(
                format!(
                    r#"
    translate to {language}
    {}
    "#,
                    plaintext
                )
                .as_str(),
            )
            .await
    }
}

#[tokio::main]
async fn main() {
    let agent = TranslatorAgent::new("http://192.168.1.132:8080", "Qwen3.5-9B-Q4_K_M.gguf");
    let plaintext = "探索未知的领域虽然充满挑战，但这也正是技术研究的乐趣所在。";
    let languages = vec![
        "English", "Japanese", "German", "French", "Russian", "Spanish",
    ];

    println!("--- Starting Multi-language Translation Test ---");
    println!("Source: {}\n", plaintext);

    for language in languages {
        // Call agent.translate(plaintext, language)
        match agent.translate(plaintext, language).await {
            Ok(translation) => {
                println!("[{}] -> {}", language, translation);
            }
            Err(e) => {
                eprintln!("[{}] Translation failed: {}", language, e);
            }
        }
    }
}
