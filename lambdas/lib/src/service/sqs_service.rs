

use anyhow::Result;

use super::line_service::MessageEventRequest;


#[derive(Debug, Clone)]
pub struct SQSService {
    client: aws_sdk_sqs::Client,
}

impl SQSService {
    pub fn new(client: &aws_sdk_sqs::Client) -> Self {
        Self {
            client: client.to_owned()
        }
    }

    pub async fn send(&self, queue_url: &String, message: &MessageEventRequest) -> Result<()> {
        println!("Sending message to queue with URL: {}", queue_url);

        let response = self.client
            .send_message()
            .queue_url(queue_url)
            .message_body(serde_json::to_string(&message)?)
            .message_deduplication_id(&message.event_id)
            .message_group_id(&message.event_id)
            .send()
            .await?;

        println!("Send message to the queue: {:?}", response);

        Ok(())

    }
}