use std::path::Path;

use serde::Deserialize;
use urlencoding::encode;

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub name: String,
}

impl DatabaseConfig {
    pub fn url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            encode(&self.user),
            encode(&self.password),
            self.host,
            self.port,
            self.name
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub server: ServerConfig,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let candidates = [
            Path::new("env_config.yml").to_path_buf(),
            Path::new("../env_config.yml").to_path_buf(),
        ];
        let path = candidates
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow::anyhow!("env_config.yml not found"))?;
        let raw = std::fs::read_to_string(path)?;
        let cfg: AppConfig = serde_yaml::from_str(&raw)?;
        Ok(cfg)
    }
}
