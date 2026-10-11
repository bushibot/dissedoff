use dissedoff_protocol::rest::*;
use dissedoff_protocol::ApiError;
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("server error ({code}): {message}")]
    Api { code: String, message: String },
    #[error("not authenticated — call register() or login() first")]
    NotAuthenticated,
}

/// A thin REST client for one Dissedoff server. Holds the current session
/// token (if any) so every client (CLI, and eventually the Tauri apps) can
/// share this one implementation instead of re-deriving the request shapes.
pub struct RestClient {
    base_url: String,
    http: reqwest::Client,
    token: RwLock<Option<String>>,
}

impl RestClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
            token: RwLock::new(None),
        }
    }

    pub async fn token(&self) -> Option<String> {
        self.token.read().await.clone()
    }

    pub async fn register(
        &self,
        username: Option<String>,
        password: String,
    ) -> Result<AuthResponse, ClientError> {
        let resp: AuthResponse = self
            .post_unauthenticated("/api/v1/auth/register", &RegisterRequest { username, password })
            .await?;
        *self.token.write().await = Some(resp.token.clone());
        Ok(resp)
    }

    pub async fn login(&self, username: String, password: String) -> Result<AuthResponse, ClientError> {
        let resp: AuthResponse = self
            .post_unauthenticated("/api/v1/auth/login", &LoginRequest { username, password })
            .await?;
        *self.token.write().await = Some(resp.token.clone());
        Ok(resp)
    }

    pub async fn server_info(&self) -> Result<ServerInfoResponse, ClientError> {
        self.get("/api/v1/server").await
    }

    pub async fn list_channels(&self) -> Result<Vec<ChannelResponse>, ClientError> {
        self.get("/api/v1/channels").await
    }

    pub async fn create_channel(&self, name: &str, kind: &str) -> Result<ChannelResponse, ClientError> {
        self.post_authenticated(
            "/api/v1/channels",
            &CreateChannelRequest {
                name: name.to_string(),
                kind: kind.to_string(),
            },
        )
        .await
    }

    pub async fn list_messages(&self, channel_id: &str) -> Result<Vec<MessageResponse>, ClientError> {
        self.get(&format!("/api/v1/channels/{channel_id}/messages")).await
    }

    pub async fn create_message(&self, channel_id: &str, content: &str) -> Result<MessageResponse, ClientError> {
        self.post_authenticated(
            &format!("/api/v1/channels/{channel_id}/messages"),
            &CreateMessageRequest {
                content: content.to_string(),
            },
        )
        .await
    }

    pub async fn voice_status(&self) -> Result<VoiceStatusResponse, ClientError> {
        self.get("/api/v1/voice/status").await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let token = self.token.read().await.clone().ok_or(ClientError::NotAuthenticated)?;
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(token)
            .send()
            .await?;
        Self::parse(resp).await
    }

    async fn post_authenticated<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ClientError> {
        let token = self.token.read().await.clone().ok_or(ClientError::NotAuthenticated)?;
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(token)
            .json(body)
            .send()
            .await?;
        Self::parse(resp).await
    }

    async fn post_unauthenticated<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ClientError> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await?;
        Self::parse(resp).await
    }

    async fn parse<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T, ClientError> {
        if resp.status().is_success() {
            Ok(resp.json::<T>().await?)
        } else {
            match resp.json::<ApiError>().await {
                Ok(err) => Err(ClientError::Api {
                    code: err.error_code,
                    message: err.message,
                }),
                Err(_) => Err(ClientError::Api {
                    code: "unknown_error".to_string(),
                    message: "request failed".to_string(),
                }),
            }
        }
    }
}
