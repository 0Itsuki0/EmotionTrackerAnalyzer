
use std::collections::HashMap;
use anyhow::Context;
use aws_lambda_events::eventbridge::EventBridgeEvent;
use lambda_runtime::{service_fn, tracing::{self}, Error, LambdaEvent};
use serde_json::{json, Value};


use lib::env_keys::TABLE_NAME;
use lib::service::{common_structs::EmotionScores, CommonService};


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
    let table_name = std::env::var(TABLE_NAME)?;
    let entries = service.dynamo.query_yesterday(&table_name).await?;

    let mut map: HashMap<String, Vec<(EmotionScores, String)>> = HashMap::new();
    for entry in entries {
        let user_id = entry.user_id;
        let score = entry.scores;
        let text = entry.text;
        if map.contains_key(&user_id) {
            map.entry(user_id).and_modify(|vec| vec.push((score, text)));
        } else {
            map.insert(user_id, vec![(score, text)]);
        }
    };

    if map.is_empty() {
        return Ok(())
    }

    let thread_ts = service.line.send_daily_thread().await?;


    for (user_id, results) in map.into_iter() {
        let scores: Vec<EmotionScores> = results.clone().into_iter().map(|r| r.0).collect();
        let advice = service.bedrock.get_daily_advice(&scores).await?;
        println!("userId: {}, advice: {:?}", user_id, advice);

        let max_anger = results.clone().into_iter().reduce(|e1, e2| {
            if e1.0.anger > e2.0.anger {
                return e1
            } else { return e2 }
        }).context("failed to find max anger")?;

        let max_contempt = results.clone().into_iter().reduce(|e1, e2| {
            if e1.0.contempt > e2.0.contempt {
                return e1
            } else { return e2 }
        }).context("failed to find max contempt")?;

        let max_disgust = results.clone().into_iter().reduce(|e1, e2| {
            if e1.0.disgust > e2.0.disgust {
                return e1
            } else { return e2 }
        }).context("failed to find max disgust")?;

        service.line.send_daily_advice(&thread_ts, &user_id, &advice, &max_anger, &max_contempt, &max_disgust).await?;
    }

    Ok(())
}
