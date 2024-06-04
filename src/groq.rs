use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};

pub const GROQ_API_BASE_URL: &str = "https://api.groq.ai/openai/v1";
pub const GROQ_BASE_MODEL: &str = "llama3-8b-8192";

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
