use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use tracing::error;

pub const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1";
pub const GROQ_BASE_MODEL: &str = "llama3-70b-8192";
pub const GROQ_CRATES_TEMPLATE: &str =
    "Based on the user instructions, identify the necessary Rust crates. \
    Respond only with a comma-separated list of binaries, such as 'hello_world_tool, http_server, basic_axum_math'. \n\
    Example: For 'simple http server with post endpoints that do basic math', respond with 'hello_world_tool, http_server, basic_axum_math'. \n\
    You must include hello_world_tool in the list as the first binary. \n\
    Do not include descriptions or additional information. User instructions: {user_instructions}";

pub struct Groq {
    api_key: String,
    base_url: String,
    client: Client,
    model: String,
}

impl Groq {
    pub fn new(api_key: &str) -> Groq {
        let base_url = String::from(GROQ_API_BASE_URL);
        let client = Client::new();
        Groq {
            api_key: api_key.to_string(),
            base_url,
            client,
            model: GROQ_BASE_MODEL.to_string(),
        }
    }

    pub async fn request_chat_completion(
        &self,
        message: &str,
    ) -> Result<ChatCompletionResponse, Error> {
        let chat_completion_endpoint = format!("{}/chat/completions", self.base_url);
        let request_body = ChatCompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            model: self.model.to_string(),
        };

        let response = self
            .client
            .post(&chat_completion_endpoint)
            .bearer_auth(&self.api_key)
            .json(&request_body)
            .send()
            .await?;

        response.json::<ChatCompletionResponse>().await
    }

    pub async fn get_crates_list(
        &self,
        user_instructions: &str,
    ) -> Result<Vec<String>, anyhow::Error> {
        let message = GROQ_CRATES_TEMPLATE.replace("{user_instructions}", user_instructions);
        let response = self.request_chat_completion(&message).await?;

        if let Some(choice) = response.choices.first() {
            let crates_list = choice.message.content.trim();
            if crates_list.contains(',') {
                Ok(crates_list
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect())
            } else {
                error!(
                    "Response did not follow the expected comma-separated format: {}",
                    crates_list
                );
                Err(anyhow::Error::msg("Invalid format"))
            }
        } else {
            error!("No choices returned in the response");
            Err(anyhow::Error::msg("No choices in response"))
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u64,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
