use super::types::MCPServerConfig;

impl MCPServerConfig {
    #[allow(dead_code)]
    pub fn new(
        id: String,
        name: String,
        server_url: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            server_url,
            oauth_client_id: None,
            oauth_client_secret: None,
            enabled: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[allow(dead_code)]
    pub fn with_oauth(mut self, client_id: Option<String>, client_secret: Option<String>) -> Self {
        self.oauth_client_id = client_id;
        self.oauth_client_secret = client_secret;
        self
    }

    #[allow(dead_code)]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[allow(dead_code)]
    pub fn update(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}