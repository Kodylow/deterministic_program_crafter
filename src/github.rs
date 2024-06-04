pub struct Github {
    client: reqwest::Client,
    api_key: String,
}

impl Github {
    pub fn new(api_key: &str) -> Github {
        Github {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
        }
    }
}
