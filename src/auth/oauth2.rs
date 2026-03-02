use anyhow::Result;
use google_gmail1::oauth2::{
    ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
};
use std::path::Path;

/// Create an OAuth2 authenticator for user authentication
/// This works with personal Gmail accounts
pub async fn create_oauth2_authenticator(
    client_secret_path: &str,
) -> Result<InstalledFlowAuthenticator> {
    // Read the OAuth2 client configuration
    let secret = read_client_secret(client_secret_path).await?;

    // Create the authenticator with a persistent token storage
    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokencache.json")
    .build()
    .await?;

    Ok(auth)
}

async fn read_client_secret(path: &str) -> Result<ApplicationSecret> {
    let secret_file = Path::new(path);
    if !secret_file.exists() {
        return Err(anyhow::anyhow!(
            "OAuth2 client secret file not found: {}. \
             Download it from Google Cloud Console > APIs & Services > Credentials > \
             Create Credentials > OAuth client ID > Desktop application",
            path
        ));
    }

    let secret_content = tokio::fs::read_to_string(path).await?;
    let secret: ApplicationSecret = serde_json::from_str(&secret_content)?;
    Ok(secret)
}