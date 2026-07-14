use crate::error::Error;

/// Réponse de l'endpoint refresh_token de Discord
#[derive(Debug, Clone)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

/// Réponse de l'endpoint authorization_code de Discord
#[derive(Debug, Clone)]
pub struct CodeExchangeResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// Rafraîchit un token Discord OAuth2 via grant_type=refresh_token
pub async fn refresh_user_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<TokenRefreshResponse, Error> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let resp = client
        .post("https://discord.com/api/v10/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Network(format!("Discord returned HTTP {}: {}", status, body)));
    }

    #[derive(serde::Deserialize)]
    struct RawResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: u64,
    }

    let raw: RawResponse = resp.json().await?;
    Ok(TokenRefreshResponse {
        access_token: raw.access_token,
        refresh_token: raw.refresh_token,
        expires_in: raw.expires_in,
    })
}

/// Échange un code OAuth2 contre des tokens via grant_type=authorization_code
pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<CodeExchangeResponse, Error> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
    ];

    let resp = client
        .post("https://discord.com/api/v10/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Network(format!("Discord returned HTTP {}: {}", status, body)));
    }

    #[derive(serde::Deserialize)]
    struct RawResponse {
        access_token: String,
        refresh_token: String,
        expires_in: u64,
    }

    let raw: RawResponse = resp.json().await?;
    Ok(CodeExchangeResponse {
        access_token: raw.access_token,
        refresh_token: raw.refresh_token,
        expires_in: raw.expires_in,
    })
}