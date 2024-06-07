use reqwest::Client;
use tracing::{error, info};

pub struct CratesIo {
    client: Client,
    cargo_cookie: String,
}

impl CratesIo {
    pub fn new(cargo_cookie: String) -> Self {
        CratesIo {
            client: Client::new(),
            cargo_cookie,
        }
    }

    pub async fn search(&self, crate_name: &str) -> Option<String> {
        info!("Searching crates.io for {}", crate_name);
        let request_url = format!("https://crates.io/api/v1/crates/{}", crate_name);
        let cookie = format!("cargo_session={}", self.cargo_cookie);

        let response = self
            .client
            .get(&request_url)
            .header(reqwest::header::COOKIE, cookie)
            .header(
                reqwest::header::USER_AGENT,
                "my_crawler (help@mycrawler.com)",
            )
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to read body".to_string());

                info!("Response Status: {}", status);

                if status.is_success() {
                    match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(crate_data) => {
                            let repo_url = crate_data["crate"]["repository"]
                                .as_str()
                                .map(|s| s.to_string());
                            info!("Repository URL: {:?}", repo_url);
                            repo_url
                        }
                        Err(e) => {
                            error!("Failed to parse JSON response: {}", e);
                            None
                        }
                    }
                } else {
                    error!(
                        "Failed to fetch crate info from crates.io with status: {}",
                        status
                    );
                    None
                }
            }
            Err(e) => {
                error!("Error fetching crate info: {}", e);
                None
            }
        }
    }
}
