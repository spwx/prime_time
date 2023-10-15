use color_eyre::eyre::Result;
use std::net::{IpAddr, SocketAddr};

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// IP address to bind to
    #[arg(default_value = "127.0.0.1")]
    ip: IpAddr,

    /// Port to bind to
    #[arg(default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let socket = SocketAddr::new(cli.ip, cli.port);

    prime_time::run(socket).await?;

    Ok(())
}
