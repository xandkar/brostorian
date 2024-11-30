use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
struct Cli {
    #[clap(short, long, default_value_t = tracing::Level::DEBUG)]
    log_level: tracing::Level,

    // Kinda silly to have default true, yes. Just laying out for later switch.
    #[clap(short = 'c', long, default_value_t = true)]
    log_color: bool,

    /// Default assumes a local copy of the file, since original could be locked.
    #[clap(short = 'd', long = "hist-db", default_value = "data/History")]
    chromium_hist_db_file: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    brostorian::tracing::init(cli.log_level, cli.log_color)?;
    tracing::debug!(?cli, "Starting.");
    brostorian::chromium::explore(&cli.chromium_hist_db_file).await?;
    Ok(())
}
