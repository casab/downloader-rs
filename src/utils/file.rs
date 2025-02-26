use anyhow::Result;

use crate::clients::S3Client;

#[tracing::instrument(name = "Download a file")]
pub async fn download_file(
    file_link: &str,
    s3_client: Option<S3Client>,
) -> Result<String, manic::ManicError> {
    let workers = 5;
    let client = manic::Downloader::new(file_link, workers).await?;
    let data = client.download().await?;

    let path = client.filename();

    if let Some(s3_client) = s3_client {
        let response = s3_client
            .put_object(path, &data.to_vec().await)
            .await
            .map_err(|err| manic::ManicError::MultipleErrors(err.to_string()))?;
        tracing::info!("S3 Response Status Code: {}", response.status_code());
    } else {
        data.save_to_file(path).await?;
    }
    Ok(path.to_string())
}
