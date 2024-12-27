use anyhow::Result;
use downloader::api::Application;
use downloader::cli::get_args;
use downloader::configuration::get_configuration;
use downloader::telemetry::{get_subscriber, init_subscriber};
use downloader::utils::download_file;

#[tokio::main]
async fn main() -> Result<()> {
    let tracing_subscriber = get_subscriber("downloader-rs".into(), "info".into(), std::io::stdout);
    init_subscriber(tracing_subscriber);

    let args = get_args();
    if args.api {
        let configuration = get_configuration().expect("Failed to read configuration.");
        let web_api = Application::build(configuration).await?;
        web_api.run_until_stopped().await?;
    } else {
        for file_link in &args.links {
            download_file(file_link).await?;
        }
    }
    Ok(())
}
