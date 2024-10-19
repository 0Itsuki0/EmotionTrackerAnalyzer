pub mod structs;

use std::collections::HashMap;

use anyhow::{Context, Ok, Result};
use aws_sdk_dynamodb::{operation::query::QueryOutput, types::AttributeValue};
use serde_dynamo::{from_items, to_item};
use structs::EmotionTableEntry;

use crate::utilities::get_previous_weekday;
use super::{common_structs::EmotionScores, line_service::MessageEventRequest};


#[derive(Debug, Clone)]
pub struct DynamoService {
    client: aws_sdk_dynamodb::Client,
}


// query related
impl DynamoService {
    pub fn new(client: &aws_sdk_dynamodb::Client) -> Self {
        Self {
            client: client.to_owned()
        }
    }

    pub async fn query_yesterday(&self, table_name: &str) -> Result<Vec<EmotionTableEntry>>{
        let yesterday = get_previous_weekday()?;
        let attribute_values: HashMap<String, AttributeValue> = HashMap::from([
            (":date".to_owned(), AttributeValue::S(yesterday.clone())),
        ]);

        let mut builder = self.client.clone()
            .query()
            // .limit(1)
            .scan_index_forward(true)
            .table_name(table_name)
            .index_name("gsi-date")
            .key_condition_expression("#date = :date")
            .expression_attribute_names("#date", "date")
            .set_expression_attribute_values(Some(attribute_values));

        let output = builder.clone().send().await?;
        // let items = results.items.context("items not available")?;
        let mut entries= self.output_to_entries(&output)?;

        let mut last_evaluated_key = output.last_evaluated_key;

        while last_evaluated_key.is_some() {
            builder = builder.clone().set_exclusive_start_key(last_evaluated_key.clone());
            let output = builder.clone().send().await?;
            entries.append(&mut self.output_to_entries(&output)?);
            last_evaluated_key = output.last_evaluated_key;
        }

        println!("entries for {}: {:?}", yesterday, entries);
        Ok(entries)
    }

    fn output_to_entries(&self, output: &QueryOutput) -> Result<Vec<EmotionTableEntry>> {
        let items = output.clone().items.context("items not available")?;
        let entries: Vec<EmotionTableEntry> = from_items(items)?;
        Ok(entries)
    }

    pub async fn register_entry(&self, table_name: &str, message_request: &MessageEventRequest, scores: &EmotionScores) -> Result<EmotionTableEntry>{
        let entry = EmotionTableEntry::new(message_request, scores)?;
        self
            .client.clone()
            .put_item()
            .table_name(table_name)
            .set_item(Some(to_item(&entry)?))
            .send()
            .await?;
        Ok(entry)
    }

}


// data export related
impl DynamoService {

    pub async fn export_data(&self, table_arn: &str, bucket: &str) -> Result<()> {
        let response = self.client.export_table_to_point_in_time()
            .table_arn(table_arn)
            .s3_bucket(bucket)
            .export_format(aws_sdk_dynamodb::types::ExportFormat::DynamodbJson)
            .send()
            .await?;

        println!("response: {:?}", response);
        Ok(())
    }

}