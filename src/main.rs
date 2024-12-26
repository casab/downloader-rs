use anyhow::{anyhow, Result};
use clap::Parser;
use reqwest::header::CONTENT_DISPOSITION;
use std::fs::File;
use std::io;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, num_args=1..)]
    link: Vec<String>,
}

fn get_file_name(resp: &reqwest::Response) -> Result<String> {
    let file_name = if let Some(header) = resp.headers().get(CONTENT_DISPOSITION) {
        header
            .to_str()
            .expect("failed to convert header value to string")
            .split("filename=")
            .collect::<Vec<&str>>()[1]
            .to_string()
    } else {
        resp.url()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_string()
    };
    if file_name.is_empty() {
        return Err(anyhow!("failed to get file name"));
    }
    Ok(file_name)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    for file_link in args.link.iter() {
        let resp = reqwest::get(file_link).await.expect("request failed");

        let filename = get_file_name(&resp)?;

        let mut file = File::create(filename).expect("failed to create file");
        io::copy(
            &mut resp
                .bytes()
                .await
                .expect("failed to read response body")
                .as_ref(),
            &mut file,
        )
        .expect("failed to copy response body to file");
    }
    Ok(())
}
