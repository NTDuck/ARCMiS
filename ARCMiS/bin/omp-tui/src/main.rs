//! `main` boots tracing, the RPC client, and the app loop.

mod app;
mod config;
mod event;
mod input;
mod pane;
mod prompt;
mod store;
mod ui;

use app::App;
use config::RunConfig;
use omprpc::client::Client;
use omprpc::transport::SpawnConfig;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use store::Store;

fn main() -> anyhow::Result<()> {
    let config = RunConfig::from_args()?;
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(run(config))
}

async fn run(config: RunConfig) -> anyhow::Result<()> {
    let _guard = init_tracing(&config)?;
    let spawn = SpawnConfig {
        omp_binary: config.omp_binary.clone(),
        omp_arguments: config.omp_arguments.clone(),
        working_dir: config.working_dir.clone(),
        approval_mode: config.approval_mode.clone(),
    };
    let mut client = Client::connect(spawn).await?;
    let store = Store::new(config.pane_titles.clone());
    client
        .request(serde_json::json!({
            "type": "set_subagent_subscription",
            "level": "events",
        }))
        .await?;
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    let result = App::new(store, client, terminal).run(&config).await;
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    result
}

fn init_tracing(config: &RunConfig) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let file = std::fs::OpenOptions::new().create(true).append(true).open(&config.log_file)?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    Ok(guard)
}
