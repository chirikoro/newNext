use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "hayabusa")]
#[command(about = "Hayabusa (隼) - Rust Full-Stack Web Framework CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Hayabusa project
    New {
        /// Project name
        name: String,
    },
    /// Start the development server with hot reload
    Dev {
        /// Port to listen on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
    /// Build for production (generates static pages)
    Build {
        /// Output directory
        #[arg(short, long, default_value = "dist")]
        output: String,
    },
    /// Start the production server
    Start {
        /// Port to listen on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            commands::new::run(&name).await;
        }
        Commands::Dev { port } => {
            commands::dev::run(port).await;
        }
        Commands::Build { output } => {
            commands::build::run(&output).await;
        }
        Commands::Start { port } => {
            commands::dev::run(port).await;
        }
    }
}
