
use anyhow::{Context, Result};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};

pub const MANIFEST_JSON: &str = "manifest-files.json";
const DATA_FOLDER: &str = "data/";

#[derive(Debug, Clone)]
pub struct S3Service {
    client: aws_sdk_s3::Client,
}

impl S3Service {
    pub fn new(client: &aws_sdk_s3::Client) -> Self {
        Self {
            client: client.to_owned()
        }
    }

    pub async fn move_data(&self, bucket_name: &str, manifest_url: &str, processed_data_prefix: &str) -> Result<()> {
        if let Ok(old_data_keys) = self.list_objects(bucket_name, processed_data_prefix).await {
            self.delete_objects(bucket_name, &old_data_keys).await?;
        };

        let new_data_folder = manifest_url.replace(MANIFEST_JSON, DATA_FOLDER);
        println!("data_object: {}", new_data_folder);
        let source_keys = self.list_objects(bucket_name, &new_data_folder).await?;
        for key in source_keys {
            self.copy_object(bucket_name, &key, &format!("{}{}", processed_data_prefix, self.get_file_name(&key)?)).await?;
        }
        Ok(())
    }

    async fn copy_object(&self, bucket_name: &str, source_key: &str, destination_key: &str) -> Result<()> {
        println!("copy from {} to  {}", source_key, destination_key);
        let _response = self.client.copy_object()
            .copy_source(format!("{}/{}", bucket_name, source_key))
            .bucket(bucket_name)
            .key(destination_key)
            .send()
            .await?;

        // println!("copy response: {:?}", response);

        Ok(())
    }

    async fn delete_objects(&self, bucket_name: &str, object_keys: &Vec<String>) -> Result<()> {
        let mut delete_object_ids: Vec<ObjectIdentifier> = vec![];
        for key in object_keys {
            println!("delete object: {}", key);
            let obj_id = ObjectIdentifier::builder()
            .key(key)
            .build()?;
            delete_object_ids.push(obj_id);
        }
        self.client.delete_objects()
            .bucket(bucket_name)
            .delete(
                Delete::builder()
                    .set_objects(Some(delete_object_ids))
                    .build()?
            )
            .send()
            .await?;

        Ok(())
    }

    pub async fn list_objects(&self, bucket_name: &str, prefix: &str) -> Result<Vec<String>> {
        let response = self.client
            .list_objects_v2()
            .bucket(bucket_name)
            .prefix(prefix)
            .send()
            .await?;

        let keys: Vec<String> = response.contents
            .context("S3 List objects: Contents undefined")?
            .into_iter()
            .map(|x| x.key.context("Key Undefined").unwrap_or("".to_owned()) )
            .filter(|x| !x.is_empty())
            .collect();

        println!("keys: {:?}", keys.clone());

        Ok(keys)
    }

    fn get_file_name(&self, key: &str) -> Result<String> {
        Ok(key.split("/").last().context("file name not found")?.to_owned())
    }
}