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
    // Setup error handling with color output
    color_eyre::install()?;

    // Setup a tracing subscriber that prints logs to stdout
    tracing_subscriber::fmt::init();

    // get CLI args
    let cli = Cli::parse();

    // create socket address
    let socket = SocketAddr::new(cli.ip, cli.port);

    // run the server
    prime_time::run(socket).await?;

    Ok(())
}
