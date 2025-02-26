use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{Bucket, BucketConfiguration, Region};

use crate::configuration::S3Settings;

pub type S3Client = Box<Bucket>;

pub async fn get_s3_client(settings: S3Settings) -> Result<S3Client, S3Error> {
    let credentials = Credentials::default()?;

    let bucket_name = settings.bucket_name;

    let region = if let Some(endpoint) = settings.endpoint {
        Region::Custom {
            region: settings.region,
            endpoint,
        }
    } else {
        settings.region.parse()?
    };

    let mut bucket =
        Bucket::new(&bucket_name, region.clone(), credentials.clone())?.with_path_style();

    if !bucket.exists().await? {
        bucket = Bucket::create_with_path_style(
            &bucket_name,
            region,
            credentials,
            BucketConfiguration::default(),
        )
        .await?
        .bucket;
    }

    Ok(bucket)
}
