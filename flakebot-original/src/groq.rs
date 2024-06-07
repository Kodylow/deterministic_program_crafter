use std::path::PathBuf;

use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{error, info};

use crate::templates::{
    GROQ_ADD_DEPENDENCY_TEMPLATE, GROQ_COMMIT_MESSAGE_TEMPLATE, GROQ_CRATES_TEMPLATE,
    GROQ_CRATE_DESCRIPTION_TEMPLATE, GROQ_INTERACTION_INSTRUCTIONS_TEMPLATE,
    GROQ_PR_MESSAGE_TEMPLATE, GROQ_PR_TITLE_TEMPLATE, GROQ_REWRITE_MAIN_RS_TEMPLATE,
    GROQ_VALIDATE_BINARY_TEMPLATE,
};

pub const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1";
pub const GROQ_BASE_MODEL: &str = "llama3-70b-8192";

pub struct Groq {
    api_key: String,
    base_url: String,
    client: Client,
    model: String,
}

impl Groq {
    pub fn new(api_key: &str) -> Groq {
        let base_url = String::from(GROQ_API_BASE_URL);
        Groq {
            api_key: api_key.to_string(),
            base_url,
            client: Client::new(),
            model: GROQ_BASE_MODEL.to_string(),
        }
    }

    pub async fn request_chat_completion(
        &self,
        message: &str,
    ) -> Result<ChatCompletionResponse, Error> {
        // Back off for 2 seconds before retrying
        loop {
            match self.inner_request_chat_completion(message).await {
                Ok(chat_response) => return Ok(chat_response),
                Err(e) => {
                    error!("Hit groq API limits: {}, backing off for 10 seconds", e);
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    pub async fn inner_request_chat_completion(
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

    pub async fn create_crate_description(
        &self,
        cargo_toml_contents: &str,
        readme_contents: &str,
        main_rs_contents: &str,
    ) -> Result<String, anyhow::Error> {
        let message = GROQ_CRATE_DESCRIPTION_TEMPLATE
            .replace("{cargo_toml_contents}", cargo_toml_contents)
            .replace("{readme_contents}", readme_contents)
            .replace("{main_rs_contents}", main_rs_contents);

        let response = self.request_chat_completion(&message).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            error!("No choices returned in the response");
            Err(anyhow::Error::msg("No choices in response"))
        }
    }

    pub async fn validate_binary(
        &self,
        instructions: &str,
        main_rs_contents: &str,
        // errors: &str,
    ) -> Result<String, anyhow::Error> {
        let message = GROQ_VALIDATE_BINARY_TEMPLATE
            .replace("{user_instructions}", instructions)
            .replace("{main_rs_contents}", main_rs_contents);
        // .replace("{errors}", errors);
        let response = self.request_chat_completion(&message).await?;
        if let Some(choice) = response.choices.first() {
            if choice.message.content.trim() == "true" {
                Ok("true".to_string())
            } else {
                Ok(choice.message.content.clone())
            }
        } else {
            Err(anyhow::Error::msg("No response from validation request"))
        }
    }

    pub async fn rewrite_main_rs(
        &self,
        instructions: &str,
        main_rs_contents: &str,
    ) -> Result<String, anyhow::Error> {
        let message = GROQ_REWRITE_MAIN_RS_TEMPLATE
            .replace("{user_instructions}", instructions)
            .replace("{main_rs_contents}", main_rs_contents);
        let response = self.request_chat_completion(&message).await?;
        let response = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from rewrite request"))?;
        Ok(response)
    }

    pub async fn add_cargo_deps(
        &self,
        main_rs_contents: &str,
        repo_dir: &PathBuf,
    ) -> Result<(), anyhow::Error> {
        let message = GROQ_ADD_DEPENDENCY_TEMPLATE.replace("{main_rs_contents}", main_rs_contents);
        let response = self.request_chat_completion(&message).await?;
        // run the command
        let command = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from add_cargo_deps request"))?;
        info!("Adding cargo command: {}", command);
        let add_output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(repo_dir)
            .output()
            .await?;

        if !add_output.status.success() {
            let add_errors = String::from_utf8_lossy(&add_output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to add missing crate: {}",
                add_errors
            ));
        }
        Ok(())
    }

    pub async fn get_interaction_instructions(
        &self,
        main_rs_path: &PathBuf,
    ) -> Result<String, anyhow::Error> {
        let main_rs_contents = std::fs::read_to_string(main_rs_path)?;
        let message =
            GROQ_INTERACTION_INSTRUCTIONS_TEMPLATE.replace("{main_rs_contents}", &main_rs_contents);
        let response = self.request_chat_completion(&message).await?;
        if let Some(choice) = response.choices.first() {
            if choice.message.content.trim() == "true" {
                Ok("true".to_string())
            } else {
                Ok(choice.message.content.clone())
            }
        } else {
            Err(anyhow::Error::msg("No response from validation request"))
        }
    }

    pub async fn generate_commit_message(&self, git_diff: &str) -> Result<String, anyhow::Error> {
        let message = GROQ_COMMIT_MESSAGE_TEMPLATE.replace("{git_diff}", git_diff);
        let response = self.request_chat_completion(&message).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            error!("No choices returned in the response for commit message generation");
            Err(anyhow::anyhow!("No choices in response for commit message"))
        }
    }

    pub async fn generate_pr_message_and_title(
        &self,
        github_token: &str,
        git_diff: &str,
    ) -> Result<(String, String), anyhow::Error> {
        let message = GROQ_PR_MESSAGE_TEMPLATE
            .replace("{github_token}", github_token)
            .replace("{git_diff}", git_diff);
        let message_response = self.request_chat_completion(&message).await?;
        let message = message_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from rewrite request"))?;
        let title = GROQ_PR_TITLE_TEMPLATE.replace("{pr_message}", &message);
        let title_response = self.request_chat_completion(&title).await?;
        let title = title_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from rewrite request"))?;
        Ok((title, message))
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
