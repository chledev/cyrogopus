use crate::server::{
    activity::ApiActivity, backup::adapters::BackupAdapter, installation::InstallationScript,
    permissions::Permissions, schedule::ApiScheduleCompletionStatus,
};
use axum::http::HeaderMap;
use std::fmt::Debug;
use tokio::future::Future;
use uuid::Uuid;

pub struct Client {
    pub(super) config: crate::config::RemoteQuery,

    pub(super) client: reqwest::Client,
    pub(super) url: String,
}

impl Client {
    pub fn new(config: &crate::config::InnerConfig, ignore_certificate_errors: bool) -> Result<Self, anyhow::Error> {
        let mut headers = HeaderMap::with_capacity(3);
        
        let user_agent = format!(
            "cyrogopus/v{} (id:{})",
            crate::VERSION,
            config.token_id
        )
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse User-Agent header: {}", e))?;
        headers.insert("User-Agent", user_agent);
        
        let accept = "application/vnd.pterodactyl.v1+json"
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse Accept header: {}", e))?;
        headers.insert("Accept", accept);
        
        let auth = format!("Bearer {}.{}", config.token_id, config.token)
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse Authorization header: {}", e))?;
        headers.insert("Authorization", auth);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .danger_accept_invalid_certs(ignore_certificate_errors)
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        Ok(Self {
            config: config.remote_query,
            client,
            url: format!("{}/api/remote", config.remote.trim_end_matches('/')),
        })
    }

    async fn retry<T, E: Debug, Fut: Future<Output = Result<T, E>>>(
        &self,
        func: impl Fn() -> Fut,
        should_retry: impl Fn(&E) -> bool,
    ) -> Result<T, E> {
        let mut tries = 0;
        let mut last_err = None;

        while tries < self.config.retry_limit {
            match func().await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    if !should_retry(&err) {
                        return Err(err);
                    }

                    tracing::debug!("retry attempt {} failed: {:#?}", tries + 1, err);

                    last_err = Some(err);
                    tries += 1;

                    if tries < self.config.retry_limit {
                        let backoff_secs = tries.pow(2);
                        let jitter = rand::random::<f32>() * 0.3;
                        let sleep_duration = std::time::Duration::from_secs_f32(
                            backoff_secs as f32 * (1.0 + jitter),
                        );

                        tokio::time::sleep(sleep_duration).await;
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| panic!("retry called with retry_limit of 0")))
    }

    fn skip_client_errors(err: &anyhow::Error) -> bool {
        if let Some(reqwest_err) = err.downcast_ref::<reqwest::Error>()
            && reqwest_err.status().is_some_and(|s| s.is_client_error())
        {
            return false;
        }

        true
    }

    #[tracing::instrument(skip(self, password))]
    pub async fn get_sftp_auth(
        &self,
        r#type: super::AuthenticationType,
        username: &str,
        password: &str,
    ) -> Result<(Uuid, Uuid, Permissions, Vec<String>), anyhow::Error> {
        tracing::debug!("getting sftp auth");

        self.retry(
            || super::get_sftp_auth(self, r#type, username, password),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_activity(&self, activity: Vec<ApiActivity>) -> Result<(), anyhow::Error> {
        tracing::debug!("sending {} activity to remote", activity.len());

        super::send_activity(self, activity).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_schedule_status(
        &self,
        schedules: Vec<ApiScheduleCompletionStatus>,
    ) -> Result<(), anyhow::Error> {
        tracing::debug!("sending {} schedule status to remote", schedules.len());

        super::send_schedule_status(self, schedules).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn reset_state(&self) -> Result<(), anyhow::Error> {
        tracing::info!("resetting remote state");

        self.retry(|| super::reset_state(self), Self::skip_client_errors)
            .await
    }

    pub async fn servers(&self) -> Result<Vec<super::servers::RawServer>, anyhow::Error> {
        tracing::info!("fetching all servers from remote");

        let mut servers = Vec::new();

        let mut page = 1;
        loop {
            tracing::info!("fetching page {} of servers", page);
            let (new_servers, pagination) = self
                .retry(
                    || super::servers::get_servers_paged(self, page),
                    Self::skip_client_errors,
                )
                .await?;
            servers.extend(new_servers);

            if pagination.current_page >= pagination.last_page {
                break;
            }

            page += 1;
        }

        tracing::info!("fetched {} servers from remote", servers.len());

        Ok(servers)
    }

    pub async fn server(
        &self,
        uuid: Uuid,
    ) -> Result<super::servers::RawServer, anyhow::Error> {
        self.retry(
            || super::servers::get_server(self, uuid),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn server_install_script(
        &self,
        uuid: Uuid,
    ) -> Result<InstallationScript, anyhow::Error> {
        tracing::info!("fetching server install script");

        self.retry(
            || super::servers::get_server_install_script(self, uuid),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_server_install(
        &self,
        uuid: Uuid,
        successful: bool,
        reinstalled: bool,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("setting server install status");

        self.retry(
            || super::servers::set_server_install(self, uuid, successful, reinstalled),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_server_transfer(
        &self,
        uuid: Uuid,
        successful: bool,
        backups: Vec<Uuid>,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("setting server transfer status");

        self.retry(
            || super::servers::set_server_transfer(self, uuid, successful, &backups),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_server_startup_variable(
        &self,
        uuid: Uuid,
        env_variable: &str,
        value: &str,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("setting server startup variable");

        super::servers::set_server_startup_variable(self, uuid, env_variable, value).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_server_startup_command(
        &self,
        uuid: Uuid,
        command: &str,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("setting server startup command");

        super::servers::set_server_startup_command(self, uuid, command).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_server_startup_docker_image(
        &self,
        uuid: Uuid,
        image: &str,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("setting server startup docker image");

        super::servers::set_server_startup_docker_image(self, uuid, image).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_backup_status(
        &self,
        uuid: Uuid,
        data: &super::backups::RawServerBackup,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("setting backup status");

        self.retry(
            || super::backups::set_backup_status(self, uuid, data),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_backup_restore_status(
        &self,
        uuid: Uuid,
        successful: bool,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("setting backup restore status");

        self.retry(
            || super::backups::set_backup_restore_status(self, uuid, successful),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn backup_upload_urls(
        &self,
        uuid: Uuid,
        size: u64,
    ) -> Result<(u64, Vec<String>), anyhow::Error> {
        tracing::info!("getting backup upload urls");

        self.retry(
            || super::backups::backup_upload_urls(self, uuid, size),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn backup_restic_configuration(
        &self,
        uuid: Uuid,
    ) -> Result<super::backups::ResticBackupConfiguration, anyhow::Error> {
        tracing::info!("getting restic backup configuration");

        self.retry(
            || super::backups::backup_restic_configuration(self, uuid),
            Self::skip_client_errors,
        )
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_backup(
        &self,
        server: Uuid,
        name: Option<&str>,
        ignored_files: &[String],
    ) -> Result<(BackupAdapter, Uuid), anyhow::Error> {
        tracing::info!("creating backup");

        super::backups::create_backup(self, server, name, ignored_files).await
    }
}
