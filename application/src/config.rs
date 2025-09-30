use anyhow::Context;
use axum::{extract::ConnectInfo, http::HeaderMap};
use serde::{Deserialize, Serialize};
use serde_default::DefaultFromSerde;
use std::{
    cell::UnsafeCell,
    collections::{BTreeMap, HashMap},
    fs::File,
    io::BufRead,
    ops::{Deref, DerefMut},
    os::unix::fs::PermissionsExt,
    str::FromStr,
    sync::Arc,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;

fn app_name() -> String {
    "Pterodactyl".to_string()
}
fn api_host() -> std::net::IpAddr {
    std::net::IpAddr::from([0, 0, 0, 0])
}
fn api_port() -> u16 {
    8080
}
fn api_server_remote_download_limit() -> usize {
    3
}
fn api_remote_download_blocked_cidrs() -> Vec<cidr::IpCidr> {
    Vec::from([
        cidr::IpCidr::from_str("127.0.0.0/8").unwrap(),
        cidr::IpCidr::from_str("10.0.0.0/8").unwrap(),
        cidr::IpCidr::from_str("172.16.0.0/12").unwrap(),
        cidr::IpCidr::from_str("192.168.0.0/16").unwrap(),
        cidr::IpCidr::from_str("169.254.0.0/16").unwrap(),
        cidr::IpCidr::from_str("::1/128").unwrap(),
        cidr::IpCidr::from_str("fe80::/10").unwrap(),
        cidr::IpCidr::from_str("fc00::/7").unwrap(),
    ])
}
fn api_directory_entry_limit() -> usize {
    10000
}
fn api_file_search_threads() -> usize {
    4
}
fn api_file_decompression_threads() -> usize {
    2
}
fn api_file_compression_threads() -> usize {
    2
}
fn api_upload_limit() -> usize {
    100
}

fn system_root_directory() -> String {
    "/var/lib/pterodactyl".to_string()
}
fn system_log_directory() -> String {
    "/var/log/pterodactyl".to_string()
}
fn system_data() -> String {
    "/var/lib/pterodactyl/volumes".to_string()
}
fn system_archive_directory() -> String {
    "/var/lib/pterodactyl/archives".to_string()
}
fn system_backup_directory() -> String {
    "/var/lib/pterodactyl/backups".to_string()
}
fn system_tmp_directory() -> String {
    "/tmp/pterodactyl".to_string()
}
fn system_username() -> String {
    "pterodactyl".to_string()
}
fn system_timezone() -> String {
    if let Ok(tz) = std::env::var("TZ") {
        return tz;
    } else if let Ok(tz) = File::open("/etc/timezone") {
        let mut buf = String::new();

        if std::io::BufReader::new(tz).read_line(&mut buf).is_ok() {
            return buf.trim().to_string();
        }
    }

    chrono::Local::now().offset().to_string()
}
fn system_passwd_directory() -> String {
    "/run/wings/etc".to_string()
}
fn system_disk_check_interval() -> u64 {
    150
}
fn system_disk_check_threads() -> usize {
    2
}
fn system_activity_send_interval() -> u64 {
    60
}
fn system_activity_send_count() -> usize {
    100
}
fn system_check_permissions_on_boot() -> bool {
    true
}
fn system_check_permissions_on_boot_threads() -> usize {
    4
}
fn system_websocket_log_count() -> usize {
    150
}

fn system_sftp_bind_address() -> std::net::IpAddr {
    std::net::IpAddr::from([0, 0, 0, 0])
}
fn system_sftp_bind_port() -> u16 {
    2022
}
fn system_sftp_key_algorithm() -> String {
    "ssh-ed25519".to_string()
}
fn system_sftp_directory_entry_limit() -> u64 {
    20000
}
fn system_sftp_directory_entry_send_amount() -> usize {
    500
}

fn system_sftp_shell_enabled() -> bool {
    true
}

fn system_sftp_shell_cli_name() -> String {
    ".cyrogopus".to_string()
}

fn system_crash_detection_enabled() -> bool {
    true
}
fn system_crash_detection_detect_clean_exit_as_crash() -> bool {
    true
}
fn system_crash_detection_timeout() -> u64 {
    60
}

fn system_backup_mounting_enabled() -> bool {
    true
}
fn system_backup_mounting_path() -> String {
    ".backups".to_string()
}

fn system_backup_wings_create_threads() -> usize {
    4
}
fn system_backup_wings_restore_threads() -> usize {
    4
}

fn system_backup_s3_create_threads() -> usize {
    4
}
fn system_backup_s3_part_upload_timeout() -> u64 {
    2 * 60 * 60
}
fn system_backup_s3_retry_limit() -> u64 {
    10
}

fn system_backup_ddup_bak_create_threads() -> usize {
    4
}

fn system_backup_restic_repository() -> String {
    "/var/lib/pterodactyl/backups/restic".to_string()
}
fn system_backup_restic_password_file() -> String {
    "/var/lib/pterodactyl/backups/restic_password".to_string()
}
fn system_backup_restic_retry_lock_seconds() -> u64 {
    60
}

fn system_backup_btrfs_restore_threads() -> usize {
    4
}
fn system_backup_btrfs_create_read_only() -> bool {
    true
}

fn system_backup_zfs_restore_threads() -> usize {
    4
}

fn docker_socket() -> String {
    "/var/run/docker.sock".to_string()
}
fn docker_delete_container_on_stop() -> bool {
    true
}

fn docker_network_interface() -> String {
    "172.18.0.1".to_string()
}
fn docker_network_dns() -> Vec<String> {
    vec!["1.1.1.1".to_string(), "1.0.0.1".to_string()]
}
fn docker_network_name() -> String {
    "pterodactyl_nw".to_string()
}
fn docker_network_driver() -> String {
    "bridge".to_string()
}
fn docker_network_mode() -> String {
    "pterodactyl_nw".to_string()
}
fn docker_network_enable_icc() -> bool {
    true
}
fn docker_network_network_mtu() -> u64 {
    1500
}

fn docker_network_interfaces_v4_subnet() -> String {
    "172.18.0.0/16".to_string()
}
fn docker_network_interfaces_v4_gateway() -> String {
    "172.18.0.1".to_string()
}
fn docker_network_interfaces_v6_subnet() -> String {
    "fdba:17c8:6c94::/64".to_string()
}
fn docker_network_interfaces_v6_gateway() -> String {
    "fdba:17c8:6c94::1011".to_string()
}

fn docker_tmpfs_size() -> u64 {
    100
}
fn docker_container_pid_limit() -> u64 {
    512
}

fn docker_installer_limits_timeout() -> u64 {
    30 * 60
}
fn docker_installer_limits_memory() -> u64 {
    1024
}
fn docker_installer_limits_cpu() -> u64 {
    100
}

fn docker_overhead_default_multiplier() -> f64 {
    1.05
}

fn docker_log_config_type() -> String {
    "local".to_string()
}
fn docker_log_config_config() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("max-size".to_string(), "5m".to_string()),
        ("max-file".to_string(), "1".to_string()),
        ("compress".to_string(), "false".to_string()),
        ("mode".to_string(), "non-blocking".to_string()),
    ])
}

fn throttles_enabled() -> bool {
    true
}
fn throttles_lines() -> u64 {
    2000
}
fn throttles_line_reset_interval() -> u64 {
    100
}

fn remote_query_timeout() -> u64 {
    30
}
fn remote_query_boot_servers_per_page() -> u64 {
    50
}
fn remote_query_retry_limit() -> u64 {
    10
}

nestify::nest! {
    #[derive(Deserialize, Serialize, DefaultFromSerde)]
    pub struct InnerConfig {
        #[serde(default)]
        pub debug: bool,
        #[serde(default = "app_name")]
        pub app_name: String,
        #[serde(default)]
        pub uuid: uuid::Uuid,

        #[serde(default)]
        pub token_id: String,
        #[serde(default)]
        pub token: String,

        #[serde(default)]
        pub api: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct Api {
            #[serde(default = "api_host")]
            pub host: std::net::IpAddr,
            #[serde(default = "api_port")]
            pub port: u16,

            #[serde(default)]
            pub ssl: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct ApiSsl {
                #[serde(default)]
                pub enabled: bool,
                #[serde(default)]
                pub cert: String,
                #[serde(default)]
                pub key: String,
            },

            #[serde(default)]
            pub disable_openapi_docs: bool,
            #[serde(default)]
            pub disable_remote_download: bool,
            #[serde(default = "api_server_remote_download_limit")]
            pub server_remote_download_limit: usize,
            #[serde(default = "api_remote_download_blocked_cidrs")]
            pub remote_download_blocked_cidrs: Vec<cidr::IpCidr>,
            #[serde(default)]
            pub disable_directory_size: bool,
            #[serde(default = "api_directory_entry_limit")]
            pub directory_entry_limit: usize,
            #[serde(default)]
            pub send_offline_server_logs: bool,
            #[serde(default = "api_file_search_threads")]
            pub file_search_threads: usize,
            #[serde(default = "api_file_decompression_threads")]
            pub file_decompression_threads: usize,
            #[serde(default = "api_file_compression_threads")]
            pub file_compression_threads: usize,
            #[serde(default = "api_upload_limit")]
            /// MB
            pub upload_limit: usize,
            #[serde(default)]
            pub trusted_proxies: Vec<std::net::IpAddr>,
        },
        #[serde(default)]
        pub system: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct System {
            #[serde(default = "system_root_directory")]
            pub root_directory: String,
            #[serde(default = "system_log_directory")]
            pub log_directory: String,
            #[serde(default = "system_data", rename = "data")]
            pub data_directory: String,
            #[serde(default = "system_archive_directory")]
            pub archive_directory: String,
            #[serde(default = "system_backup_directory")]
            pub backup_directory: String,
            #[serde(default = "system_tmp_directory")]
            pub tmp_directory: String,

            #[serde(default = "system_username")]
            pub username: String,
            #[serde(default = "system_timezone")]
            pub timezone: String,

            #[serde(default)]
            pub user: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemUser {
                #[serde(default)]
                pub rootless: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemUserRootless {
                    #[serde(default)]
                    pub enabled: bool,
                    #[serde(default)]
                    pub container_uid: u32,
                    #[serde(default)]
                    pub container_gid: u32,
                },

                #[serde(default)]
                pub uid: u32,
                #[serde(default)]
                pub gid: u32,
            },

            #[serde(default)]
            pub passwd: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemPasswd {
                #[serde(default)]
                pub enabled: bool,
                #[serde(default = "system_passwd_directory")]
                pub directory: String,
            },

            #[serde(default = "system_disk_check_interval")]
            pub disk_check_interval: u64,
            #[serde(default = "system_disk_check_threads")]
            pub disk_check_threads: usize,
            #[serde(default)]
            pub disk_limiter_mode: #[derive(Deserialize, Serialize, Default)] #[serde(rename_all = "snake_case")] pub enum SystemDiskLimiterMode {
                #[default]
                None,
                BtrfsSubvolume,
                ZfsDataset,
                XfsQuota,
            },
            #[serde(default = "system_activity_send_interval")]
            pub activity_send_interval: u64,
            #[serde(default = "system_activity_send_count")]
            pub activity_send_count: usize,
            #[serde(default = "system_check_permissions_on_boot")]
            pub check_permissions_on_boot: bool,
            #[serde(default = "system_check_permissions_on_boot_threads")]
            pub check_permissions_on_boot_threads: usize,
            #[serde(default = "system_websocket_log_count")]
            pub websocket_log_count: usize,

            #[serde(default)]
            pub sftp: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemSftp {
                #[serde(default = "system_sftp_bind_address")]
                pub bind_address: std::net::IpAddr,
                #[serde(default = "system_sftp_bind_port")]
                pub bind_port: u16,

                #[serde(default)]
                pub read_only: bool,
                #[serde(default = "system_sftp_key_algorithm")]
                pub key_algorithm: String,
                #[serde(default)]
                pub disable_password_auth: bool,
                #[serde(default = "system_sftp_directory_entry_limit")]
                pub directory_entry_limit: u64,
                #[serde(default = "system_sftp_directory_entry_send_amount")]
                pub directory_entry_send_amount: usize,

                #[serde(default)]
                pub shell: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemSftpShell {
                    #[serde(default = "system_sftp_shell_enabled")]
                    pub enabled: bool,

                    #[serde(default)]
                    pub cli: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemSftpShellCli {
                        #[serde(default = "system_sftp_shell_cli_name")]
                        pub name: String,
                    },
                },
            },

            #[serde(default)]
            pub crash_detection: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemCrashDetection {
                #[serde(default = "system_crash_detection_enabled")]
                pub enabled: bool,
                #[serde(default = "system_crash_detection_detect_clean_exit_as_crash")]
                pub detect_clean_exit_as_crash: bool,
                #[serde(default = "system_crash_detection_timeout")]
                pub timeout: u64,
            },

            #[serde(default)]
            pub backups: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackups {
                #[serde(default)]
                /// MiB/s
                pub write_limit: u64,
                #[serde(default)]
                /// MiB/s
                pub read_limit: u64,
                #[serde(default)]
                pub compression_level: crate::io::compression::CompressionLevel,

                #[serde(default)]
                pub mounting: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackupsMounting {
                    #[serde(default = "system_backup_mounting_enabled")]
                    pub enabled: bool,
                    #[serde(default = "system_backup_mounting_path")]
                    pub path: String,
                },

                #[serde(default)]
                pub wings: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackupsWings {
                    #[serde(default = "system_backup_wings_create_threads")]
                    pub create_threads: usize,
                    #[serde(default = "system_backup_wings_restore_threads")]
                    pub restore_threads: usize,

                    #[serde(default)]
                    pub archive_format: #[derive(Clone, Copy, Deserialize, Serialize, Default)] #[serde(rename_all = "snake_case")] pub enum SystemBackupsWingsArchiveFormat {
                        Tar,
                        #[default]
                        TarGz,
                        TarZstd,
                        Zip,
                    },
                },
                #[serde(default)]
                pub s3: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackupsS3 {
                    #[serde(default = "system_backup_s3_create_threads")]
                    pub create_threads: usize,
                    #[serde(default = "system_backup_s3_part_upload_timeout")]
                    pub part_upload_timeout: u64,
                    #[serde(default = "system_backup_s3_retry_limit")]
                    pub retry_limit: u64,
                },
                #[serde(default)]
                pub ddup_bak: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackupsDdupBak {
                    #[serde(default = "system_backup_ddup_bak_create_threads")]
                    pub create_threads: usize,

                    #[serde(default)]
                    pub compression_format: #[derive(Clone, Copy, Deserialize, Serialize, Default)] #[serde(rename_all = "snake_case")] pub enum SystemBackupsDdupBakCompressionFormat {
                        None,
                        #[default]
                        Deflate,
                        Gzip,
                        Brotli
                    },
                },
                #[serde(default)]
                pub restic: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackupsRestic {
                    #[serde(default = "system_backup_restic_repository")]
                    pub repository: String,
                    #[serde(default = "system_backup_restic_password_file")]
                    pub password_file: String,

                    #[serde(default = "system_backup_restic_retry_lock_seconds")]
                    pub retry_lock_seconds: u64,
                    #[serde(default)]
                    pub environment: BTreeMap<String, String>,
                },
                #[serde(default)]
                pub btrfs: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackupsBtrfs {
                    #[serde(default = "system_backup_btrfs_restore_threads")]
                    pub restore_threads: usize,

                    #[serde(default = "system_backup_btrfs_create_read_only")]
                    pub create_read_only: bool,
                },
                #[serde(default)]
                pub zfs: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemBackupsZfs {
                    #[serde(default = "system_backup_zfs_restore_threads")]
                    pub restore_threads: usize,
                },
            },

            #[serde(default)]
            pub transfers: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct SystemTransfers {
                #[serde(default)]
                /// MiB/s
                pub download_limit: u64,
            },
        },
        #[serde(default)]
        pub docker: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct Docker {
            #[serde(default = "docker_socket")]
            pub socket: String,
            #[serde(default)]
            pub server_name_in_container_name: bool,
            #[serde(default = "docker_delete_container_on_stop")]
            pub delete_container_on_stop: bool,

            #[serde(default)]
            pub network: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct DockerNetwork {
                #[serde(default = "docker_network_interface")]
                pub interface: String,
                #[serde(default)]
                pub disable_interface_binding: bool,
                #[serde(default = "docker_network_dns")]
                pub dns: Vec<String>,

                #[serde(default = "docker_network_name")]
                pub name: String,
                #[serde(default)]
                pub ispn: bool,
                #[serde(default = "docker_network_driver")]
                pub driver: String,
                #[serde(default = "docker_network_mode")]
                pub mode: String,
                #[serde(default)]
                pub is_internal: bool,
                #[serde(default = "docker_network_enable_icc")]
                pub enable_icc: bool,
                #[serde(default = "docker_network_network_mtu")]
                pub network_mtu: u64,

                #[serde(default)]
                pub interfaces: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct DockerNetworkInterfaces {
                    #[serde(default)]
                    pub v4: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct DockerNetworkInterfacesV4 {
                        #[serde(default = "docker_network_interfaces_v4_subnet")]
                        pub subnet: String,
                        #[serde(default = "docker_network_interfaces_v4_gateway")]
                        pub gateway: String,
                    },
                    #[serde(default)]
                    pub v6: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct DockerNetworkInterfacesV6 {
                        #[serde(default = "docker_network_interfaces_v6_subnet")]
                        pub subnet: String,
                        #[serde(default = "docker_network_interfaces_v6_gateway")]
                        pub gateway: String,
                    },
                },
            },

            #[serde(default)]
            pub domainname: String,
            #[serde(default)]
            pub registries: HashMap<String, #[derive(Deserialize, Serialize)] pub struct DockerRegistryConfiguration {
                pub username: String,
                pub password: String,
            }>,

            #[serde(default = "docker_tmpfs_size")]
            pub tmpfs_size: u64,
            #[serde(default = "docker_container_pid_limit")]
            pub container_pid_limit: u64,

            #[serde(default)]
            pub installer_limits: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct DockerInstallerLimits {
                #[serde(default = "docker_installer_limits_timeout")]
                pub timeout: u64,

                #[serde(default = "docker_installer_limits_memory")]
                /// MiB
                pub memory: u64,
                #[serde(default = "docker_installer_limits_cpu")]
                /// %
                pub cpu: u64,
            },

            #[serde(default)]
            pub overhead: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct DockerOverhead {
                #[serde(default)]
                pub r#override: bool,
                #[serde(default = "docker_overhead_default_multiplier")]
                pub default_multiplier: f64,

                #[serde(default)]
                /// Memory Limit MiB -> Multiplier
                pub multipliers: BTreeMap<i64, f64>,
            },

            #[serde(default)]
            pub userns_mode: String,

            #[serde(default)]
            pub log_config: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct DockerLogConfig {
                #[serde(default = "docker_log_config_type")]
                pub r#type: String,
                #[serde(default = "docker_log_config_config")]
                pub config: BTreeMap<String, String>,
            },
        },

        #[serde(default)]
        pub throttles: #[derive(Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct Throttles {
            #[serde(default = "throttles_enabled")]
            pub enabled: bool,
            #[serde(default = "throttles_lines")]
            pub lines: u64,
            #[serde(default = "throttles_line_reset_interval")]
            pub line_reset_interval: u64,
        },

        pub remote: String,
        #[serde(default)]
        pub remote_query: #[derive(Clone, Copy, Deserialize, Serialize, DefaultFromSerde)] #[serde(default)] pub struct RemoteQuery {
            #[serde(default = "remote_query_timeout")]
            pub timeout: u64,
            #[serde(default = "remote_query_boot_servers_per_page")]
            pub boot_servers_per_page: u64,
            #[serde(default = "remote_query_retry_limit")]
            pub retry_limit: u64,
        },

        #[serde(default)]
        pub allowed_mounts: Vec<String>,
        #[serde(default)]
        pub allowed_origins: Vec<String>,

        #[serde(default)]
        pub allow_cors_private_network: bool,
        #[serde(default)]
        pub ignore_panel_config_updates: bool,
    }
}

impl DockerOverhead {
    /// \`\`\`yaml
    /// multipliers:
    ///   1024: 1.05
    ///   2048: 1.10
    /// \`\`\`
    /// means, <=1024MiB ram = 1.05 multiplier,
    /// <=2048MiB ram = 1.10 multiplier,
    /// >2048MiB ram = 1.05 multiplier (default_multiplier)
    pub fn get_mutiplier(&self, memory: i64) -> f64 {
        if !self.r#override {
            if memory <= 2048 {
                return 1.15;
            } else if memory <= 4096 {
                return 1.10;
            }

            return 1.05;
        }

        for m in self.multipliers.keys().copied().rev() {
            if memory > m {
                continue;
            }

            return self.multipliers[&m];
        }

        self.default_multiplier
    }

    #[inline]
    pub fn get_memory(&self, memory: i64) -> i64 {
        let multiplier = self.get_mutiplier(memory);

        (memory as f64 * multiplier) as i64
    }
}

pub struct Config {
    inner: UnsafeCell<InnerConfig>,

    pub path: String,
    pub ignore_certificate_errors: bool,
    pub client: crate::remote::client::Client,
    pub jwt: crate::remote::jwt::JwtClient,
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

impl Config {
    pub fn open(
        path: &str,
        debug: bool,
        ignore_debug: bool,
        ignore_certificate_errors: bool,
    ) -> Result<(Arc<Self>, WorkerGuard), anyhow::Error> {
        let file = File::open(path).context(format!("failed to open config file {path}"))?;
        let reader = std::io::BufReader::new(file);
        let config: InnerConfig = serde_yml::from_reader(reader)
            .context(format!("failed to parse config file {path}"))?;

        let client = crate::remote::client::Client::new(&config, ignore_certificate_errors);
        let jwt = crate::remote::jwt::JwtClient::new(&config.token);
        let mut config = Self {
            inner: UnsafeCell::new(config),

            path: path.to_string(),
            ignore_certificate_errors,
            client,
            jwt,
        };

        config.ensure_directories()?;

        let latest_log_path = std::path::Path::new(&config.system.log_directory).join("cyrogopus.log");
        let latest_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&latest_log_path)
            .context("failed to open latest log file")?;

        let rolling_appender = tracing_appender::rolling::Builder::new()
            .filename_prefix("cyrogopus")
            .filename_suffix("log")
            .max_log_files(30)
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .build(&config.system.log_directory)
            .context("failed to create rolling log file appender")?;

        let (file_appender, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
            .buffered_lines_limit(50)
            .lossy(false)
            .finish(latest_file.and(rolling_appender));

        config.ensure_user()?;
        config.ensure_passwd()?;
        config.save()?;

        if debug {
            config.unsafe_mut().debug = true;
        }

        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
                .with_writer(std::io::stdout.and(file_appender))
                .with_target(false)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_max_level(if config.debug && !ignore_debug {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                })
                .finish(),
        )
        .unwrap();

        if config.api.send_offline_server_logs && config.docker.delete_container_on_stop {
            tracing::warn!(
                "You have enabled sending offline server logs, but also deleting containers on stop. This will result in no logs being sent for stopped servers."
            );
        }

        Ok((Arc::new(config), guard))
    }

    pub fn save_new(path: &str, config: InnerConfig) -> Result<(), anyhow::Error> {
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap())
            .context(format!("failed to create config directory {path}"))?;
        let file = File::create(path).context(format!("failed to create config file {path}"))?;
        let writer = std::io::BufWriter::new(file);
        serde_yml::to_writer(writer, &config)
            .context(format!("failed to write config file {path}"))?;

        Ok(())
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let file = File::create(&self.path)
            .context(format!("failed to create config file {}", self.path))?;
        let writer = std::io::BufWriter::new(file);
        serde_yml::to_writer(writer, unsafe { &*self.inner.get() })
            .context(format!("failed to write config file {}", self.path))?;

        Ok(())
    }

    #[inline]
    pub fn find_ip(
        &self,
        headers: &HeaderMap,
        connect_info: ConnectInfo<std::net::SocketAddr>,
    ) -> std::net::IpAddr {
        for ip in &self.api.trusted_proxies {
            if connect_info.ip() == *ip {
                if let Some(forwarded) = headers.get("X-Forwarded-For")
                    && let Ok(forwarded) = forwarded.to_str()
                    && let Some(ip) = forwarded.split(',').next()
                {
                    return ip.parse().unwrap_or_else(|_| connect_info.ip());
                }

                if let Some(forwarded) = headers.get("X-Real-IP")
                    && let Ok(forwarded) = forwarded.to_str()
                {
                    return forwarded.parse().unwrap_or_else(|_| connect_info.ip());
                }
            }
        }

        connect_info.ip()
    }

    #[allow(clippy::mut_from_ref)]
    pub fn unsafe_mut(&self) -> &mut InnerConfig {
        unsafe { &mut *self.inner.get() }
    }

    fn ensure_directories(&self) -> std::io::Result<()> {
        let directories = vec![
            &self.system.root_directory,
            &self.system.log_directory,
            &self.system.data_directory,
            &self.system.archive_directory,
            &self.system.backup_directory,
            &self.system.tmp_directory,
        ];

        for dir in directories {
            if !std::path::Path::new(dir).exists() {
                std::fs::create_dir_all(dir)?;
                std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
            }
        }

        if self.system.passwd.enabled
            && !std::path::Path::new(&self.system.passwd.directory).exists()
        {
            std::fs::create_dir_all(&self.system.passwd.directory)?;
            std::fs::set_permissions(
                &self.system.passwd.directory,
                std::fs::Permissions::from_mode(0o755),
            )?;
        }

        Ok(())
    }

    fn ensure_user(&mut self) -> Result<(), anyhow::Error> {
        let release =
            std::fs::read_to_string("/etc/os-release").unwrap_or_else(|_| "unknown".to_string());

        if release.contains("distroless") {
            self.system.username =
                std::env::var("CYROGOPUS_USERNAME").unwrap_or_else(|_| system_username());
            self.system.user.uid = std::env::var("CYROGOPUS_UID")
                .unwrap_or_else(|_| "988".to_string())
                .parse()?;
            self.system.user.gid = std::env::var("CYROGOPUS_GID")
                .unwrap_or_else(|_| "988".to_string())
                .parse()?;

            return Ok(());
        }

        if self.system.user.rootless.enabled {
            let user = users::get_current_uid();
            let group = users::get_current_gid();
            let username = users::get_current_username();

            self.system.username = username.unwrap().into_string().unwrap();
            self.system.user.uid = user;
            self.system.user.gid = group;

            return Ok(());
        }

        if let Some(user) = users::get_user_by_name(&self.system.username) {
            self.system.user.uid = user.uid();
            self.system.user.gid = user.primary_group_id();

            return Ok(());
        }

        let command = if release.contains("alpine") {
            std::process::Command::new("addgroup")
                .arg("-S")
                .arg(&self.system.username)
                .output()
                .context("failed to create group")?;

            format!(
                "adduser -S -D -H -G {} -s /sbin/nologin {}",
                self.system.username, self.system.username
            )
        } else {
            format!(
                "useradd --system --no-create-home --shell /usr/sbin/nologin {}",
                self.system.username
            )
        };

        let split = command.split_whitespace().collect::<Vec<_>>();
        let output = std::process::Command::new(split[0])
            .args(&split[1..])
            .output()
            .context(format!("failed to create user {}", self.system.username))?;
        if !output.status.success() {
            return Err(
                anyhow::anyhow!("failed to create user {}", self.system.username).context(format!(
                    "failed to create user {}: {}",
                    self.system.username,
                    String::from_utf8_lossy(&output.stderr)
                )),
            );
        }

        let user = users::get_user_by_name(&self.system.username)
            .context(format!("failed to get user {}", self.system.username))?;

        self.system.user.uid = user.uid();
        self.system.user.gid = user.primary_group_id();

        Ok(())
    }

    fn ensure_passwd(&self) -> Result<(), anyhow::Error> {
        if self.system.passwd.enabled {
            std::fs::write(
                std::path::Path::new(&self.system.passwd.directory).join("group"),
                format!(
                    "root:x:0:\ncontainer:x:{}:\nnogroup:x:65534:",
                    self.system.user.gid
                ),
            )
            .context(format!(
                "failed to write group file {}",
                std::path::Path::new(&self.system.passwd.directory)
                    .join("group")
                    .display()
            ))?;
            std::fs::set_permissions(
                std::path::Path::new(&self.system.passwd.directory).join("group"),
                std::fs::Permissions::from_mode(0o644),
            )
            .context(format!(
                "failed to set permissions for group file {}",
                std::path::Path::new(&self.system.passwd.directory)
                    .join("group")
                    .display()
            ))?;

            std::fs::write(
                std::path::Path::new(&self.system.passwd.directory).join("passwd"),
                format!(
                    "root:x:0:0::/root:/bin/sh\ncontainer:x:{}:{}::/home/container:/bin/sh\nnobody:x:65534:65534::/var/empty:/bin/sh\n",
                    self.system.user.uid, self.system.user.gid
                ),
            )
            .context(format!(
                "failed to write passwd file {}",
                std::path::Path::new(&self.system.passwd.directory)
                    .join("passwd")
                    .display()
            ))?;
            std::fs::set_permissions(
                std::path::Path::new(&self.system.passwd.directory).join("passwd"),
                std::fs::Permissions::from_mode(0o644),
            )
            .context(format!(
                "failed to set permissions for passwd file {}",
                std::path::Path::new(&self.system.passwd.directory)
                    .join("passwd")
                    .display()
            ))?;
        }

        Ok(())
    }

    pub async fn ensure_network(&self, client: &bollard::Docker) -> Result<(), anyhow::Error> {
        let network = client
            .inspect_network::<String>(&self.docker.network.name, None)
            .await;

        if network.is_err() {
            client
                .create_network(bollard::network::CreateNetworkOptions {
                    name: self.docker.network.name.as_str(),
                    driver: self.docker.network.driver.as_str(),
                    enable_ipv6: true,
                    internal: self.docker.network.is_internal,
                    ipam: bollard::models::Ipam {
                        config: Some(vec![
                            bollard::models::IpamConfig {
                                subnet: Some(self.docker.network.interfaces.v4.subnet.clone()),
                                gateway: Some(self.docker.network.interfaces.v4.gateway.clone()),
                                ..Default::default()
                            },
                            bollard::models::IpamConfig {
                                subnet: Some(self.docker.network.interfaces.v6.subnet.clone()),
                                gateway: Some(self.docker.network.interfaces.v6.gateway.clone()),
                                ..Default::default()
                            },
                        ]),
                        ..Default::default()
                    },
                    options: HashMap::from([
                        ("encryption", "false"),
                        ("com.docker.network.bridge.default_bridge", "false"),
                        (
                            "com.docker.network.bridge.enable_icc",
                            &self.docker.network.enable_icc.to_string(),
                        ),
                        ("com.docker.network.bridge.enable_ip_masquerade", "true"),
                        ("com.docker.network.bridge.host_binding_ipv4", "0.0.0.0"),
                        ("com.docker.network.bridge.name", &self.docker.network.name),
                        (
                            "com.docker.network.driver.mtu",
                            &self.docker.network.network_mtu.to_string(),
                        ),
                    ]),
                    ..Default::default()
                })
                .await
                .context(format!(
                    "failed to create network {}",
                    self.docker.network.name
                ))?;

            let driver = &self.docker.network.driver;
            if driver != "host" && driver != "overlay" && driver != "weavemesh" {
                self.unsafe_mut().docker.network.interface =
                    self.docker.network.interfaces.v4.gateway.clone();
            }
        }

        match self.docker.network.driver.as_str() {
            "host" => {
                self.unsafe_mut().docker.network.interface = "127.0.0.1".to_string();
            }
            "overlay" | "weavemesh" => {
                self.unsafe_mut().docker.network.interface = "".to_string();
                self.unsafe_mut().docker.network.ispn = true;
            }
            _ => {
                self.unsafe_mut().docker.network.ispn = false;
            }
        }

        self.save()?;

        Ok(())
    }
}

impl Deref for Config {
    type Target = InnerConfig;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.get() }
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner.get() }
    }
}
