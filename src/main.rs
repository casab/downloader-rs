use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use downloader::cli::get_args;
use downloader::telemetry::{get_subscriber, init_subscriber};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use reqwest::header::CONTENT_DISPOSITION;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tracing::instrument(name = "Get file name from the link", skip(resp))]
fn get_file_name(resp: &reqwest::Response) -> Result<String> {
    if let Some(header) = resp.headers().get(CONTENT_DISPOSITION) {
        let header_value = header.to_str()?;
        if let Some(filename) = header_value.split("filename=").nth(1) {
            return Ok(filename.trim_matches('"').to_string());
        }
    }
    resp.url()
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|&s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("failed to get file name"))
}

#[tracing::instrument(name = "Write to a file", skip(stream))]
async fn write_to_file(
    filename: &str,
    mut stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
) -> Result<()> {
    let mut file = File::create(&filename).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow!(e))?;
        file.write_all(&chunk).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let tracing_subscriber = get_subscriber("downloader-rs".into(), "info".into(), std::io::stdout);
    init_subscriber(tracing_subscriber);

    let args = get_args();
    if args.api {
        todo!();
    } else {
        for file_link in args.links.iter() {
            let resp = reqwest::get(file_link)
                .await
                .context("Failed to send request")?;
            let filename = get_file_name(&resp).context("Failed to get file name")?;

            write_to_file(&filename, resp.bytes_stream())
                .await
                .context("Failed to write to a file")?;
        }
    }
    Ok(())
}
