use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;

// =============================================================================
// Configuration
// =============================================================================

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,

    pub port: u16,
}

impl ServerConfig {
    #[must_use]
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    #[must_use]
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

// =============================================================================
// Server
// =============================================================================

pub struct Server {
    config: ServerConfig,
}

impl Server {
    #[must_use]
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(ServerConfig::default())
    }

    pub async fn run(self, router: Router) -> anyhow::Result<()> {
        let address = self.config.socket_addr();

        tracing::info!("Starting server on {}", address);

        let listener = TcpListener::bind(&address).await?;

        tracing::info!("Server listening on {}", address);

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!("Server shutdown complete");

        Ok(())
    }

    #[must_use]
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

// =============================================================================
// Shutdown Signal
// =============================================================================

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }
        () = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod server_config {
        use super::*;

        #[rstest]
        fn new_creates_config() {
            let config = ServerConfig::new("127.0.0.1", 8080);

            assert_eq!(config.host, "127.0.0.1");
            assert_eq!(config.port, 8080);
        }

        #[rstest]
        fn default_config() {
            let config = ServerConfig::default();

            assert_eq!(config.host, "0.0.0.0");
            assert_eq!(config.port, 3000);
        }

        #[rstest]
        fn socket_addr_formats_correctly() {
            let config = ServerConfig::new("localhost", 3000);

            assert_eq!(config.socket_addr(), "localhost:3000");
        }

        #[rstest]
        fn clone() {
            let config = ServerConfig::new("0.0.0.0", 3000);
            let cloned = config.clone();

            assert_eq!(config.host, cloned.host);
            assert_eq!(config.port, cloned.port);
        }

        #[rstest]
        fn debug_format() {
            let config = ServerConfig::new("0.0.0.0", 3000);
            let debug = format!("{:?}", config);

            assert!(debug.contains("ServerConfig"));
            assert!(debug.contains("0.0.0.0"));
            assert!(debug.contains("3000"));
        }
    }

    mod server {
        use super::*;

        #[rstest]
        fn new_creates_server() {
            let config = ServerConfig::new("0.0.0.0", 3000);
            let server = Server::new(config.clone());

            assert_eq!(server.config().host, config.host);
            assert_eq!(server.config().port, config.port);
        }

        #[rstest]
        fn with_defaults_creates_server() {
            let server = Server::with_defaults();

            assert_eq!(server.config().host, "0.0.0.0");
            assert_eq!(server.config().port, 3000);
        }

        #[rstest]
        fn config_returns_config_reference() {
            let server = Server::with_defaults();
            let config = server.config();

            assert_eq!(config.host, "0.0.0.0");
        }
    }
}
