//! Docker environment management for integration tests.

#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub app_base_url: String,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            app_base_url: std::env::var("BANK_API_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
        }
    }
}
