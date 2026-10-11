use std::path::PathBuf;

pub struct Config {
    pub data_dir: PathBuf,
    pub bind_addr: String,
    pub default_server_name: String,
}

impl Config {
    pub fn from_env() -> Self {
        let data_dir = std::env::var("DISSEDOFF_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data"));
        let bind_addr =
            std::env::var("DISSEDOFF_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let default_server_name = std::env::var("DISSEDOFF_SERVER_NAME")
            .unwrap_or_else(|_| "My Dissedoff Server".to_string());
        Self {
            data_dir,
            bind_addr,
            default_server_name,
        }
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("dissedoff.sqlite")
    }

    pub fn salt_path(&self) -> PathBuf {
        self.data_dir.join("server_salt.bin")
    }
}
