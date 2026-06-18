use crate::auth::session::{write_session, AuthSession};
use crate::error::{AppError, AppResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use reqwest::Client;
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

const OAUTH_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthLoginResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Deserialize)]
struct ExchangeResponse {
    #[serde(rename = "sessionToken")]
    session_token: String,
}

fn assert_listener_accepting(listener: &TcpListener) -> AppResult<()> {
    listener
        .set_nonblocking(true)
        .map_err(|e| AppError::Auth(e.to_string()))?;
    std::thread::sleep(Duration::from_millis(50));
    match listener.accept() {
        Ok((mut stream, _)) => {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(AppError::Auth(format!("listener probe: {e}"))),
    }
}

const CALLBACK_HTML: &str = r#"<!DOCTYPE html><html lang="ru"><head><meta charset="utf-8"/><title>Hehel Zip</title></head><body><p>Завершение входа…</p><script>
(function(){
  var q = new URLSearchParams(location.search);
  var expected = q.get('state');
  var hash = new URLSearchParams(location.hash.replace(/^#/, ''));
  var token = hash.get('access_token');
  if (!expected || !token) { document.body.textContent = 'Ошибка: нет state или access_token'; return; }
  fetch('/finish?state=' + encodeURIComponent(expected), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ accessToken: token })
  }).then(function(r){ document.body.textContent = r.ok ? 'OK — можно закрыть вкладку.' : 'Ошибка входа'; });
})();
</script></body></html>"#;

pub async fn start_heron_login(
    heron_auth_url: String,
    hcom_api_url: String,
) -> AppResult<AuthLoginResult> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| AppError::Auth(format!("bind: {e}")))?;
    assert_listener_accepting(&listener)?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::Auth(e.to_string()))?
        .port();

    let mut state = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut state);
    let state_param = URL_SAFE_NO_PAD.encode(state);
    let return_to = format!(
        "http://127.0.0.1:{port}/callback?state={}",
        urlencoding::encode(&state_param)
    );

    let login_url = format!(
        "{}/login?return_to={}",
        heron_auth_url.trim_end_matches('/'),
        urlencoding::encode(&return_to)
    );

    open::that(&login_url).map_err(|e| AppError::Auth(format!("open browser: {e}")))?;

    let done: Arc<Mutex<Option<AuthSession>>> = Arc::new(Mutex::new(None));
    let started = Instant::now();
    listener
        .set_nonblocking(true)
        .map_err(|e| AppError::Auth(e.to_string()))?;

    while started.elapsed() < OAUTH_TTL {
        if done.lock().map_err(|e| AppError::Auth(e.to_string()))?.is_some() {
            break;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                if let Ok(session) = handle_http(
                    &mut stream,
                    &state,
                    &heron_auth_url,
                    &hcom_api_url,
                    &state_param,
                )
                .await
                {
                    write_session(&session)?;
                    *done.lock().map_err(|e| AppError::Auth(e.to_string()))? =
                        Some(session);
                    return Ok(AuthLoginResult {
                        ok: true,
                        message: "Вход выполнен".into(),
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => return Err(AppError::Auth(format!("accept: {e}"))),
        }
    }

    Err(AppError::Auth("OAuth timeout (5 min)".into()))
}

async fn handle_http(
    stream: &mut TcpStream,
    expected_state: &[u8; 32],
    _heron_auth_url: &str,
    hcom_api_url: &str,
    state_param: &str,
) -> AppResult<AuthSession> {
    let mut buf = [0u8; 16384];
    let n = stream
        .read(&mut buf)
        .map_err(|e| AppError::Auth(e.to_string()))?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let line = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = line.split_whitespace().collect();
    let method = parts.first().copied().unwrap_or("");
    let path = parts.get(1).copied().unwrap_or("");

    if method == "GET" && path.starts_with("/callback") {
        let body = CALLBACK_HTML;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .map_err(|e| AppError::Auth(e.to_string()))?;
        return Err(AppError::Auth("awaiting finish".into()));
    }

    if method == "POST" && path.starts_with("/finish") {
        let query_state = path
            .split('?')
            .nth(1)
            .and_then(|q| {
                q.split('&')
                    .find_map(|p| p.strip_prefix("state=").map(|v| urlencoding::decode(v).ok()))
            })
            .and_then(|o| o)
            .map(|s| s.into_owned())
            .ok_or_else(|| AppError::Auth("missing state".into()))?;

        let state_bytes = URL_SAFE_NO_PAD
            .decode(query_state.as_bytes())
            .map_err(|_| AppError::Auth("bad state".into()))?;
        if state_bytes.len() != 32
            || state_bytes.as_slice().ct_eq(expected_state).unwrap_u8() == 0
        {
            return Err(AppError::Auth("state mismatch".into()));
        }

        let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
        let json_body = &req[body_start..];
        let parsed: serde_json::Value = serde_json::from_str(json_body)
            .map_err(|e| AppError::Auth(format!("body: {e}")))?;
        let access_token = parsed
            .get("accessToken")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Auth("missing accessToken".into()))?;

        let client = Client::new();
        let exchange_url = format!(
            "{}/api/auth/heron/exchange",
            hcom_api_url.trim_end_matches('/')
        );
        let exchange_res = client
            .post(&exchange_url)
            .header("X-Client-App", "hehel-zip")
            .json(&serde_json::json!({ "accessToken": access_token }))
            .send()
            .await
            .map_err(|e| AppError::Auth(format!("exchange: {e}")))?;

        if !exchange_res.status().is_success() {
            let body = exchange_res.text().await.unwrap_or_default();
            stream
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n")
                .ok();
            return Err(AppError::Auth(format!("exchange: {body}")));
        }
        let exchanged: ExchangeResponse = exchange_res
            .json()
            .await
            .map_err(|e| AppError::Auth(format!("exchange parse: {e}")))?;

        let _ = state_param;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .map_err(|e| AppError::Auth(e.to_string()))?;

        return Ok(AuthSession {
            session_token: exchanged.session_token,
            hcom_api_url: hcom_api_url.to_string(),
            heron_auth_url: _heron_auth_url.to_string(),
            expires_at: None,
        });
    }

    stream
        .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
        .ok();
    Err(AppError::Auth("not found".into()))
}
