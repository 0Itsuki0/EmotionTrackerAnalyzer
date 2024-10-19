use aws_lambda_events::s3::S3Event;
use lambda_runtime::{service_fn, tracing::{self}, Error, LambdaEvent};
use serde_json::{json, Value};

use lib::env_keys::{BUCKET_NAME, PROCESSED_S3_FOLDER};
use lib::service::{CommonService, s3_service::MANIFEST_JSON};


#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let config = aws_config::load_from_env().await;
    let service = CommonService::new(&config);
    let service_function = service_fn(|event| async { export_finish_handler(event, &service).await });
    lambda_runtime::run(service_function).await?;

    Ok(())

}

async fn export_finish_handler(event: LambdaEvent<S3Event>, service: &CommonService) -> Result<Value, Error> {
    println!("{:?}", event.payload);
    match process_event(event.payload, service).await {
        Ok(_) => {
            println!("finish processing event with success!")
        },
        Err(error) => {
            println!("Error processing event: {:?}", error)
        },
    }
    return Ok(json!({}))
}


async fn process_event(event: S3Event, service: &CommonService) -> anyhow::Result<()> {

    let bucket_name = std::env::var(BUCKET_NAME)?;
    let process_data_folder = std::env::var(PROCESSED_S3_FOLDER)?;

    for record in event.records {
        if record.s3.bucket.name.is_none() ||
            record.s3.bucket.name.unwrap() != bucket_name ||
            record.s3.object.key.is_none() {
            continue;
        }

        let object_key = record.s3.object.key.unwrap();
        if !object_key.contains(MANIFEST_JSON) {
            continue;
        }
        service.s3.move_data(&bucket_name, &object_key, &process_data_folder).await?;
    }
    Ok(())
}
