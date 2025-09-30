use chrono::Datelike;
use clap::ArgMatches;
use std::sync::Arc;

const TARGET: &str = env!("CARGO_TARGET");

pub async fn version(_matches: &ArgMatches, _config: Option<&Arc<crate::config::Config>>) -> i32 {
    println!(
        "github.com/chledev/cyrogopus {}:{} ({TARGET})",
        crate::VERSION,
        crate::GIT_COMMIT
    );
    println!(
        "copyright © 2025 - {} Chle & Contributors",
        chrono::Local::now().year()
    );

    0
}
