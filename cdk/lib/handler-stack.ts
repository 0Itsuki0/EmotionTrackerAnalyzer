import { join } from 'path';
import { RustFunction } from 'cargo-lambda-cdk';
import { EndpointType, LambdaRestApi } from 'aws-cdk-lib/aws-apigateway'
import { Table } from 'aws-cdk-lib/aws-dynamodb';
import { Duration, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";
import { Effect, PolicyStatement } from 'aws-cdk-lib/aws-iam';
import { Queue } from 'aws-cdk-lib/aws-sqs';
import { SqsEventSource } from 'aws-cdk-lib/aws-lambda-event-sources';
import { Rule, Schedule } from 'aws-cdk-lib/aws-events';
import { LambdaFunction } from 'aws-cdk-lib/aws-events-targets';


export interface HandlerStackProps extends StackProps {
    table: Table;
}

export class EmotionHandlerStack extends Stack {
    private contextKey = this.node.tryGetContext("context");
    private context = this.node.tryGetContext(this.contextKey);
    private slackToken = this.context["SLACK_VERIFICATION_TOKEN"];
    private botToken = this.context["BOT_OAUTH_TOKEN"];
    private warningThreshold = this.context["IMMEDIATE_WARNING_THRESHOLD"] ?? "0.6";
    private resultChannelId = this.context["RESULT_CHANNEL_ID"];
    private chatModel = this.context["CHAT_MODEL"] ?? "anthropic.claude-3-haiku-20240307-v1:0";

    constructor(scope: Construct, id: string, props: HandlerStackProps) {
        super(scope, id, props);

        const table = props.table;

        // sqs
        const queue = new Queue(this, 'SlackEventQueue.fifo', {
            visibilityTimeout: Duration.minutes(10),
            fifo: true,
        });


        // apigateway lambda
        const apigatewayLambda = new RustFunction(this, 'EmotionAPIGatewayLambda', {
            // Path to the root directory.
            manifestPath: join(__dirname, '..', '..', 'lambdas/receive_handler/'),
            environment: {
                "SLACK_VERIFICATION_TOKEN": this.slackToken,
                "QUEUE_URL": queue.queueUrl,
            }
        });

        queue.grantSendMessages(apigatewayLambda)

        const restApi = new LambdaRestApi(this, 'EmotionAPIGateway', {
            handler: apigatewayLambda,
            endpointTypes: [EndpointType.REGIONAL],
        });

        const sqsLambda =  new RustFunction(this, 'EmotionSQSLambda', {
            // Path to the root directory.
            manifestPath: join(__dirname, '..', '..', 'lambdas/sqs_handler/'),
            environment: {
                "SLACK_VERIFICATION_TOKEN": this.slackToken,
                "QUEUE_ARN": queue.queueArn,
                'TABLE_NAME': table.tableName,
                "BOT_OAUTH_TOKEN": this.botToken,
                "IMMEDIATE_WARNING_THRESHOLD": this.warningThreshold,
                "CHAT_MODEL": this.chatModel
            },
            timeout: Duration.minutes(5)
        });

        queue.grantConsumeMessages(sqsLambda)
        sqsLambda.addEventSource(
            new SqsEventSource(queue, {
                batchSize: 1,
            })
        )
        table.grantReadWriteData(sqsLambda)
        sqsLambda.addToRolePolicy(new PolicyStatement({
            effect: Effect.ALLOW,
            actions: [
                'bedrock:InvokeModel'
            ],
            resources: ['*'],
        }))

        const dailyLambda =  new RustFunction(this, 'EmotionDailyLambda', {
            // Path to the root directory.
            manifestPath: join(__dirname, '..', '..', 'lambdas/daily_handler/'),
            environment: {
                'TABLE_NAME': table.tableName,
                "RESULT_CHANNEL_ID": this.resultChannelId,
                "BOT_OAUTH_TOKEN": this.botToken,
                "CHAT_MODEL": this.chatModel
            },
            timeout: Duration.minutes(5)
        });

        table.grantReadWriteData(dailyLambda)
        dailyLambda.addToRolePolicy(new PolicyStatement({
            effect: Effect.ALLOW,
            actions: [
                'bedrock:InvokeModel'
            ],
            resources: ['*'],
        }))
        // 09:00 JST Everyday
        const dailyRule = new Rule(this, 'EmotionDailyRule', {
            schedule: Schedule.cron({
                minute: '0',
                hour: '0',
                weekDay: 'MON-FRI',
            }),
            targets: [new LambdaFunction(dailyLambda, {
                retryAttempts: 0
            })]
        })
    }
}