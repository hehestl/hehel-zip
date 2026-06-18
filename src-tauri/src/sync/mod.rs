use crate::auth::{get_session_token, read_session};
use crate::error::{AppError, AppResult};
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, IF_NONE_MATCH, HeaderValue};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    pub enabled: bool,
    pub api_base_url: String,
    pub project_id: String,
    pub heron_auth_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncQueueItem {
    pub id: i64,
    pub archive_id: String,
    pub entry_path: String,
    pub status_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteArchiveLink {
    pub entry_path: String,
    pub workflow_status_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteWorkflowStatus {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntryDto {
    pub path: String,
    pub size_bytes: u64,
    pub is_dir: bool,
}

#[derive(Debug, Serialize)]
struct BulkArchiveLinksPayload {
    archive_id: String,
    links: Vec<RemoteArchiveLinkPayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteArchiveLinkPayload {
    pub entry_path: String,
    pub workflow_status_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncArchiveResponse {
    archive_id: String,
    manifest_hash: String,
    manifest_version: i64,
    server_updated_at: String,
}

pub struct HestiaSyncClient {
    http: Client,
}

impl HestiaSyncClient {
    pub fn new() -> Self {
        Self {
            http: Client::new(),
        }
    }

    fn bearer(config: &SyncConfig) -> AppResult<String> {
        if !config.access_token.is_empty() {
            return Ok(config.access_token.clone());
        }
        get_session_token()?.ok_or_else(|| {
            AppError::Sync("Не выполнен вход. Войдите через Heron.".into())
        })
    }

    fn auth_header(config: &SyncConfig) -> AppResult<HeaderValue> {
        let token = Self::bearer(config)?;
        HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|e| AppError::Sync(format!("auth header: {e}")))
    }

    pub async fn fetch_workflow_statuses(
        &self,
        config: &SyncConfig,
    ) -> AppResult<Vec<RemoteWorkflowStatus>> {
        let url = format!(
            "{}/api/hehel/projects/{}/workflow-statuses",
            config.api_base_url.trim_end_matches('/'),
            config.project_id
        );
        let response = self
            .http
            .get(&url)
            .header(AUTHORIZATION, Self::auth_header(config)?)
            .send()
            .await
            .map_err(|e| AppError::Sync(format!("catalog: {e}")))?;

        if response.status().as_u16() == 401 {
            return Err(AppError::Sync("401 — войдите снова".into()));
        }
        if !response.status().is_success() {
            return Err(AppError::Sync(format!(
                "catalog HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Sync(format!("catalog parse: {e}")))
    }

    pub async fn init_board(&self, config: &SyncConfig) -> AppResult<()> {
        let url = format!(
            "{}/api/hehel/projects/{}/archives/init-board",
            config.api_base_url.trim_end_matches('/'),
            config.project_id
        );
        let response = self
            .http
            .post(&url)
            .header(AUTHORIZATION, Self::auth_header(config)?)
            .send()
            .await
            .map_err(|e| AppError::Sync(format!("init-board: {e}")))?;
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Sync(format!("init-board: {body}")));
        }
        Ok(())
    }

    pub async fn push_bulk(
        &self,
        config: &SyncConfig,
        archive_id: String,
        links: Vec<RemoteArchiveLinkPayload>,
    ) -> AppResult<()> {
        let url = format!(
            "{}/api/hehel/projects/{}/archive-links/bulk",
            config.api_base_url.trim_end_matches('/'),
            config.project_id
        );
        let payload = BulkArchiveLinksPayload { archive_id, links };
        let response = self
            .http
            .post(&url)
            .header(AUTHORIZATION, Self::auth_header(config)?)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Sync(format!("push: {e}")))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Sync(format!("push HTTP: {body}")));
        }
        Ok(())
    }

    pub async fn sync_archive(
        &self,
        config: &SyncConfig,
        archive_id: String,
        label: String,
        entries: Vec<ManifestEntryDto>,
        links: Vec<RemoteArchiveLinkPayload>,
    ) -> AppResult<String> {
        let client_request_id = Uuid::new_v4().to_string();
        let url = format!(
            "{}/api/hehel/projects/{}/archives/sync",
            config.api_base_url.trim_end_matches('/'),
            config.project_id
        );
        let body = serde_json::json!({
            "archiveId": archive_id,
            "label": label,
            "entries": entries,
            "links": links,
        });
        let response = self
            .http
            .post(&url)
            .header(AUTHORIZATION, Self::auth_header(config)?)
            .header("clientRequestId", client_request_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Sync(format!("sync: {e}")))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Sync(format!("sync HTTP: {text}")));
        }
        let resp: SyncArchiveResponse = response
            .json()
            .await
            .map_err(|e| AppError::Sync(format!("sync parse: {e}")))?;
        Ok(resp.manifest_hash)
    }

    pub async fn pull_manifest_if_changed(
        &self,
        config: &SyncConfig,
        archive_id: &str,
        cached_hash: Option<&str>,
    ) -> AppResult<Option<Vec<ManifestEntryDto>>> {
        let url = format!(
            "{}/api/hehel/projects/{}/archives/{}/manifest",
            config.api_base_url.trim_end_matches('/'),
            config.project_id,
            archive_id
        );
        let mut req = self
            .http
            .get(&url)
            .header(AUTHORIZATION, Self::auth_header(config)?);
        if let Some(hash) = cached_hash {
            req = req.header(IF_NONE_MATCH, hash);
        }
        let response = req
            .send()
            .await
            .map_err(|e| AppError::Sync(format!("pull manifest: {e}")))?;
        if response.status().as_u16() == 304 {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(AppError::Sync(format!(
                "manifest HTTP {}",
                response.status()
            )));
        }
        #[derive(Deserialize)]
        struct ManifestBody {
            entries: Vec<ManifestEntryDto>,
        }
        let body: ManifestBody = response
            .json()
            .await
            .map_err(|e| AppError::Sync(format!("manifest parse: {e}")))?;
        Ok(Some(body.entries))
    }

    pub async fn pull_links(
        &self,
        config: &SyncConfig,
        archive_id: &str,
    ) -> AppResult<Vec<RemoteArchiveLink>> {
        let url = format!(
            "{}/api/hehel/projects/{}/archive-links?archiveId={}",
            config.api_base_url.trim_end_matches('/'),
            config.project_id,
            archive_id
        );
        let response = self
            .http
            .get(&url)
            .header(AUTHORIZATION, Self::auth_header(config)?)
            .send()
            .await
            .map_err(|e| AppError::Sync(format!("pull: {e}")))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Sync(format!("pull HTTP: {body}")));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Sync(format!("pull parse: {e}")))
    }

    pub fn fill_config_from_session(mut config: SyncConfig) -> AppResult<SyncConfig> {
        if let Some(session) = read_session()? {
            if config.api_base_url.is_empty() {
                config.api_base_url = session.hcom_api_url;
            }
            if config.heron_auth_url.is_empty() {
                config.heron_auth_url = session.heron_auth_url;
            }
            if config.access_token.is_empty() {
                config.access_token = session.session_token;
            }
        }
        Ok(config)
    }
}

impl Default for HestiaSyncClient {
    fn default() -> Self {
        Self::new()
    }
}
