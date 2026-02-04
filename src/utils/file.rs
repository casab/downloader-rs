use anyhow::Result;

use crate::clients::S3Client;

/// Sanitize a filename extracted from a URL to prevent path traversal.
fn sanitize_filename(raw: &str) -> String {
    // Extract just the file name component, stripping any directory path
    let name = std::path::Path::new(raw)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("download");

    // Remove any remaining path separators or null bytes
    let sanitized: String = name
        .chars()
        .filter(|c| *c != '/' && *c != '\\' && *c != '\0')
        .collect();

    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        "download".to_string()
    } else {
        sanitized
    }
}

#[tracing::instrument(name = "Download a file")]
pub async fn download_file(
    file_link: &str,
    s3_client: Option<S3Client>,
) -> Result<String, manic::ManicError> {
    let workers = 5;
    let client = manic::Downloader::new(file_link, workers).await?;
    let data = client.download().await?;

    let safe_filename = sanitize_filename(client.filename());

    if let Some(s3_client) = s3_client {
        let response = s3_client
            .put_object(&safe_filename, &data.to_vec().await)
            .await
            .map_err(|err| manic::ManicError::MultipleErrors(err.to_string()))?;
        tracing::info!("S3 Response Status Code: {}", response.status_code());
    } else {
        data.save_to_file(&safe_filename).await?;
    }
    Ok(safe_filename)
}
