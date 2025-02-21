use anyhow::{anyhow, Result};
use reqwest::header::CONTENT_DISPOSITION;

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

#[tracing::instrument(name = "Download a file")]
pub async fn download_file(file_link: &str) -> Result<(), manic::ManicError> {
    let workers = 5;
    let client = manic::Downloader::new(file_link, workers).await?;
    let data = client.download().await?;
    data.save_to_file(client.filename()).await?;
    Ok(())
}
