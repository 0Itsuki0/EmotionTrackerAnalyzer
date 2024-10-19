
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::response::Json;
use lib::env_keys::QUEUE_URL;
use lib::service::line_service::{EventChallengeRequest, MessageEventRequest};
use serde_json::{json, Value};
use lib::service::CommonService;


fn build_error_response(message: &str) -> Response {
    let mut json_header = HeaderMap::new();
    json_header.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let mut response = Response::new(json!({
        "success": false,
        "message": message
    }).to_string());
    *response.status_mut() = StatusCode::BAD_REQUEST;
    return (json_header, response).into_response();
}

fn build_success_response(body: &Value) -> Response {
    let mut json_header = HeaderMap::new();
    json_header.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    let response = Response::new(body.to_string());
    return (json_header, response).into_response();
}


pub async fn webhook_received(
    State(service): State<CommonService>,
    Json(params): Json<Value>
) -> Response {

    println!("params: {}", params);

    let challenge_request = serde_json::from_value::<EventChallengeRequest>(params.clone());
    if challenge_request.is_ok() {
        let challenge_request = challenge_request.unwrap();
        let verification_result = service.line.verify_challenge(&challenge_request);
        if verification_result.is_err() || verification_result.unwrap() == false {
            return build_error_response("Error Verifying.");
        } else {
            let response_body = json!({
                "challenge": challenge_request.challenge
            });
            return build_success_response(&response_body);
        }
    }

    let message_request = match serde_json::from_value::<MessageEventRequest>(params.clone()) {
        Ok(request) => request,
        Err(error) => {
            println!("Error converting to Message request: {:?}", error);
            return build_success_response(&json!({}));
        },
    };

    match service.line.verify_message_request(&message_request) {
        Ok(_) => {},
        Err(error) => {
            println!("Error verifying Message request: {:?}", error);
            return build_success_response(&json!({}));
        },
    }

    let Ok(queue_url) = std::env::var(QUEUE_URL) else {
        println!("SQS URL not availabe");
        return build_success_response(&json!({}));
    };


    match service.sqs.send(&queue_url, &message_request).await {
        Ok(_) => {},
        Err(error) => {
            println!("Error sending to sqs: {}", error);
        },
    }

    return build_success_response(&json!({}));
}
