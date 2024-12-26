use anyhow::{anyhow, Result};
use clap::Parser;
use futures_util::StreamExt;
use reqwest::header::CONTENT_DISPOSITION;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, num_args=1..)]
    link: Vec<String>,
}

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    for file_link in args.link.iter() {
        println!("Downloading: {file_link}");

        let resp = reqwest::get(file_link).await?;
        let filename = get_file_name(&resp)?;

        println!("Saving as: {filename}");

        let mut file = File::create(&filename).await?;
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }

        println!("Download complete: {filename}");
    }
    Ok(())
}
