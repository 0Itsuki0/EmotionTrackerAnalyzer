pub mod dynamo_service;
pub mod bedrock_service;
pub mod sqs_service;
pub mod line_service;
pub mod s3_service;
pub mod common_structs;

use aws_config::SdkConfig;


#[derive(Debug, Clone)]
pub struct CommonService {
    pub dynamo: dynamo_service::DynamoService,
    pub bedrock: bedrock_service::BedrockService,
    pub sqs: sqs_service::SQSService,
    pub s3: s3_service::S3Service,
    pub line: line_service::LineService,
}

impl CommonService {
    pub fn new(config: &SdkConfig) -> Self {
        let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
        let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
        let sqs_client = aws_sdk_sqs::Client::new(&config);
        let s3_client = aws_sdk_s3::Client::new(&config);

        let line_client = line_service::LineService::new();

        Self {
            dynamo: dynamo_service::DynamoService::new(&dynamo_client),
            bedrock: bedrock_service::BedrockService::new(&bedrock_client),
            sqs: sqs_service::SQSService::new(&sqs_client),
            s3: s3_service::S3Service::new(&s3_client),
            line: line_client
        }
    }
}