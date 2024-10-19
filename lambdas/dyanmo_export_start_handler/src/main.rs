use aws_lambda_events::eventbridge::EventBridgeEvent;

use lambda_runtime::{service_fn, tracing::{self}, Error, LambdaEvent};
use serde_json::{json, Value};

use lib::env_keys::{BUCKET_NAME, TABLE_ARN};
use lib::service::CommonService;


#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let config = aws_config::load_from_env().await;
    let service = CommonService::new(&config);
    let service_function = service_fn(|event| async { eventbridge_handler(event, &service).await });
    lambda_runtime::run(service_function).await?;

    Ok(())

}

async fn eventbridge_handler(event: LambdaEvent<EventBridgeEvent>, service: &CommonService) -> Result<Value, Error> {
    println!("{:?}", event.payload);
    match process_event(service).await {
        Ok(_) => {
            println!("finish processing event with success!")
        },
        Err(error) => {
            println!("Error processing event: {:?}", error)
        },
    }
    return Ok(json!({}))
}


async fn process_event(service: &CommonService) -> anyhow::Result<()> {
    let table_arn = std::env::var(TABLE_ARN)?;
    let bucket_name = std::env::var(BUCKET_NAME)?;
    service.dynamo.export_data(&table_arn, &bucket_name).await?;
    Ok(())
}
