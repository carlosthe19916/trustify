use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose};
use directories::ProjectDirs;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf, process::ExitCode};
use tokio::time::{Duration, sleep};

#[derive(clap::Parser, Debug, Clone, Eq, PartialEq)]
#[command()]
#[group(id = "login")]
pub struct Login {
    #[arg(id = "oidc_client_id", long = "oidc-client-id", default_value = "cli")]
    pub oidc_client_id: String,

    // If not set, we will try to discover it from trustify_server_url
    #[arg(id = "oidc_server_url", long = "oidc-server-url")]
    pub oidc_server_url: Option<String>,

    // The Trustify root URL
    #[arg(
        id = "trustify_server_url",
        long = "trustify-server-url",
        default_value = "http://localhost:8080"
    )]
    pub trustify_server_url: String,
}

#[derive(clap::Parser, Debug, Clone, Eq, PartialEq)]
#[command()]
#[group(id = "logout")]
pub struct Logout;

#[derive(Debug, Clone, Eq, PartialEq)]
struct Auth {
    config_dir: PathBuf,
    config_file: PathBuf,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
struct TrustifyConfig {
    oidc_server_url: String,
}

#[derive(Deserialize, Debug)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

impl Auth {
    fn init_default() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("org", "guac", "trustify")
            .ok_or(anyhow!("Could not determine config directory for this OS"))?;
        let config_dir: PathBuf = proj_dirs.config_dir().to_path_buf();

        // Ensure directory exists
        fs::create_dir_all(&config_dir)?;

        // Define JSON file path
        let config_file = config_dir.join("config.json");

        Ok(Self {
            config_dir,
            config_file,
        })
    }

    fn save_token_response(self, token_response: &TokenResponse) -> Result<()> {
        // Serialize to JSON string
        let json = serde_json::to_string_pretty(token_response)?;

        // Write JSON to fileProjectDirs
        let mut file = fs::File::create(&self.config_file)?;
        file.write_all(json.as_bytes())?;

        println!("Credentials saved to {:?}", &self.config_file);
        Ok(())
    }
}

impl Login {
    pub async fn run(self) -> Result<ExitCode> {
        let client = Client::new();

        // Prepare OIDC URLs
        let oidc_server_url = if let Some(oidc_server_url) = self.oidc_server_url {
            oidc_server_url
        } else {
            let index_page = client
                .get(self.trustify_server_url)
                .send()
                .await?
                .text()
                .await?;

            let regex = Regex::new(r#"window\._env\s*=\s*"([^"]+)""#)
                .map_err(|error| anyhow!(error.to_string()))?;
            let env_base64 = regex
                .captures(&index_page)
                .ok_or(anyhow!("Could not match regex in index.html"))?
                .get(1)
                .ok_or(anyhow!("Could not extract _env value"))?;

            let decoded_bytes = general_purpose::STANDARD.decode(env_base64.as_str())?;
            let decoded_str = String::from_utf8(decoded_bytes)?;
            let trustify_config: TrustifyConfig = serde_json::from_str(&decoded_str)?;
            trustify_config.oidc_server_url
        };
        println!("oidc_server_url={oidc_server_url}");

        // Get device code
        let resp = client
            .post(format!(
                "{}/protocol/openid-connect/auth/device",
                oidc_server_url
            ))
            .form(&[
                ("client_id", self.oidc_client_id.as_str()),
                ("scope", "openid"),
            ])
            .send()
            .await?
            .json::<DeviceCodeResponse>()
            .await?;

        println!(
            "Please visit {} and enter code: {}",
            resp.verification_uri, resp.user_code
        );
        println!("Code expires in {} seconds", resp.expires_in);

        // Poll token endpoint
        loop {
            sleep(Duration::from_secs(resp.interval)).await;

            let token_resp = client
                .post(format!("{}/protocol/openid-connect/token", oidc_server_url))
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", resp.device_code.as_str()),
                    ("client_id", self.oidc_client_id.as_str()),
                ])
                .send()
                .await?;

            if token_resp.status().is_success() {
                let token_response = token_resp.json::<TokenResponse>().await?;

                match Auth::init_default() {
                    Ok(auth) => {
                        auth.save_token_response(&token_response)?;
                        break Ok(ExitCode::SUCCESS);
                    }
                    Err(_) => break Ok(ExitCode::FAILURE),
                }
            }
        }
    }
}

impl Logout {
    pub async fn run(self) -> Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
