import { AttributeType, Table, BillingMode } from 'aws-cdk-lib/aws-dynamodb';
import { RemovalPolicy, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";


export class EmotionDatabaseStack extends Stack {
    table: Table;

    constructor(scope: Construct, id: string, props?: StackProps) {
        super(scope, id, props);

        this.table = new Table(this, 'EmotionTable', {
            partitionKey: { name: 'event_id', type: AttributeType.STRING },
            billingMode: BillingMode.PAY_PER_REQUEST,
            removalPolicy: RemovalPolicy.DESTROY,
            pointInTimeRecovery: true
        });

        this.table.addGlobalSecondaryIndex({
            indexName: 'gsi-userid',
            partitionKey: { name: 'user_id', type: AttributeType.STRING },
            sortKey: { name: 'timestamp', type: AttributeType.NUMBER },
        });

        this.table.addGlobalSecondaryIndex({
            indexName: 'gsi-date',
            partitionKey: { name: 'date', type: AttributeType.STRING },
            sortKey: { name: 'timestamp', type: AttributeType.NUMBER },
        });

    }
}