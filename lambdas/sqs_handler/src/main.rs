
use aws_lambda_events::sqs::SqsEvent;
use lambda_runtime::{service_fn, tracing::{self}, Error, LambdaEvent};
use lib::{env_keys::{IMMEDIATE_WARNING_THRESHOLD, QUEUE_ARN, TABLE_NAME}, service::{line_service::MessageEventRequest, CommonService}, warnings::{ANGER_WARNING, CONTEMPT_WARNING, DISGUST_WARNING}};
use serde_json::{json, Value};


#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let config = aws_config::load_from_env().await;
    let service = CommonService::new(&config);
    let service_function = service_fn(|event| async { sqs_handler(event, &service).await });
    lambda_runtime::run(service_function).await?;

    Ok(())

}

async fn sqs_handler(event: LambdaEvent<SqsEvent>, service: &CommonService) -> Result<Value, Error> {
    println!("{:?}", event.payload);
    match process_event(event.payload, service).await {
        Ok(_) => {
            println!("finish processing sqs event with success!")
        },
        Err(error) => {
            println!("Error processing sqs event: {:?}", error)
        },
    }
    return Ok(json!({}))
}


async fn process_event(event: SqsEvent, service: &CommonService) -> anyhow::Result<()> {
    let queue_arn = std::env::var(QUEUE_ARN)?;
    let table_name = std::env::var(TABLE_NAME)?;
    let threshold: f64 = std::env::var(IMMEDIATE_WARNING_THRESHOLD)?.parse()?;

    for record in event.records.into_iter() {
        if record.event_source_arn.is_some() && record.event_source_arn.unwrap() != queue_arn {
            println!("wrong event source ");
            continue;
        }

        let Some(message_string) = record.body else {
            continue;
        };

        let message_request = match serde_json::from_str::<MessageEventRequest>(&message_string) {
            Ok(request) => request,
            Err(error) => {
                println!("error parsing message: {:?}", error);
                continue;
            },
        };

        println!("message request: {:?}", message_request);

        let event = message_request.clone().event;
        let scores = service.bedrock.get_emotion_scroe(&event.text).await?;
        let entry = service.dynamo.register_entry(&table_name, &message_request, &scores).await?;
        println!("Entry registered to Dynamo: {:?}", entry);


        if scores.anger > threshold {
            service.line.send_immediate_warning(&event.channel, &event.event_ts, &event.user, ANGER_WARNING).await?
        }
        if scores.disgust > threshold {
            service.line.send_immediate_warning(&event.channel, &event.event_ts, &event.user, DISGUST_WARNING).await?
        }
        if scores.contempt > threshold {
            service.line.send_immediate_warning(&event.channel, &event.event_ts, &event.user, CONTEMPT_WARNING).await?
        }
    }

    Ok(())
}
