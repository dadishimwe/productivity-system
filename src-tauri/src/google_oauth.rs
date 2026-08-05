use productivity_core::AppState;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tiny_http::{Header, Response, Server};

const KEYRING_SERVICE: &str = "productivity-app-google";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar";

pub fn store_refresh_token(account_id: &str, token: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account_id).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

pub fn delete_refresh_token(account_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account_id).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn oauth_config() -> Result<(String, String), String> {
    let client_id = std::env::var("GOOGLE_OAUTH_CLIENT_ID")
        .map_err(|_| "Set GOOGLE_OAUTH_CLIENT_ID in the environment".to_string())?;
    let client_secret = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")
        .map_err(|_| "Set GOOGLE_OAUTH_CLIENT_SECRET in the environment".to_string())?;
    Ok((client_id, client_secret))
}

fn pkce_pair() -> (String, String) {
    let verifier: String = (0..64)
        .map(|_| {
            let idx = rand::random::<usize>() % 62;
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"[idx] as char
        })
        .collect();
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        hasher.finalize(),
    );
    (verifier, challenge)
}

fn wait_for_auth_code(listener: std::net::TcpListener) -> Result<String, String> {
    let server = Server::from_listener(listener, None).map_err(|e| e.to_string())?;
    for request in server.incoming_requests() {
        let path = request.url();
        let query = path.split('?').nth(1).unwrap_or("");
        for pair in query.split('&') {
            let mut kv = pair.splitn(2, '=');
            let key = kv.next().unwrap_or("");
            let val = kv.next().unwrap_or("");
            if key == "code" && !val.is_empty() {
                let code = urlencoding::decode(val)
                    .map_err(|e| e.to_string())?
                    .into_owned();
                let response = Response::from_string(
                    "<html><body><p>Signed in. You can close this tab.</p></body></html>",
                )
                .with_header(Header::from_bytes("Content-Type", "text/html").unwrap());
                let _ = request.respond(response);
                return Ok(code);
            }
            if key == "error" {
                let _ = request.respond(Response::from_string("Authorization denied."));
                return Err("Google authorization was denied".into());
            }
        }
        let _ = request.respond(Response::from_string("Missing code.").with_status_code(400));
    }
    Err("OAuth callback server stopped".into())
}

pub async fn connect_google(
    app: &AppHandle,
    state: &AppState,
) -> Result<productivity_core::google_accounts::GoogleAccount, String> {
    let (client_id, client_secret) = oauth_config()?;
    let (verifier, challenge) = pkce_pair();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");
    let auth_url = format!(
        "{AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(CALENDAR_SCOPE),
        urlencoding::encode(&challenge),
    );

    let server_handle = std::thread::spawn(move || wait_for_auth_code(listener));

    app.shell()
        .open(auth_url, None)
        .map_err(|e| e.to_string())?;

    let code = server_handle
        .join()
        .map_err(|_| "OAuth thread panicked".to_string())??;

    let client = Client::new();
    let token_resp = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let access_token = token_resp
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Token response missing access_token".to_string())?;
    let refresh_token = token_resp
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "No refresh token returned — revoke app access in Google Account and try again".to_string()
        })?;

    let userinfo = client
        .get(USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let email = userinfo
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Userinfo missing email".to_string())?;

    let account = productivity_core::google_accounts::upsert_account(state, email)
        .await
        .map_err(|e| e.to_string())?;

    store_refresh_token(&account.id, refresh_token)?;

    Ok(account)
}

pub async fn disconnect_google(state: &AppState, account_id: &str) -> Result<(), String> {
    delete_refresh_token(account_id)?;
    productivity_core::google_accounts::disconnect_account(state, account_id)
        .await
        .map_err(|e| e.to_string())
}
