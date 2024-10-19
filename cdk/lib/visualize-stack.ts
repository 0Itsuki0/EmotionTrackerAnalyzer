import { join } from 'path';
import { RustFunction } from 'cargo-lambda-cdk';
import { Table } from 'aws-cdk-lib/aws-dynamodb';
import { Duration, RemovalPolicy, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";
import { Effect, ManagedPolicy, PolicyDocument, PolicyStatement, Role, ServicePrincipal } from 'aws-cdk-lib/aws-iam';
import { S3EventSource } from 'aws-cdk-lib/aws-lambda-event-sources';
import { Rule, Schedule } from 'aws-cdk-lib/aws-events';
import { LambdaFunction } from 'aws-cdk-lib/aws-events-targets';
import { Bucket, EventType } from 'aws-cdk-lib/aws-s3';
import { CfnTable } from 'aws-cdk-lib/aws-glue';
import { CfnAnalysis, CfnDashboard, CfnDataSet, CfnDataSource, CfnTemplate } from 'aws-cdk-lib/aws-quicksight';


export interface VisualizerStackProps extends StackProps {
    table: Table;
}

export class EmotionVisualizerStack extends Stack {
    private contextKey = this.node.tryGetContext("context");
    private context = this.node.tryGetContext(this.contextKey);
    private processedFolder = "processed/";
    private databaseName = "emotion-database"
    private datasetName = "EmotionDataSet"
    private glueTableName = "emotion_data_table"

    private quicksightUserName = this.context["QUICKSIGHT_USER_NAME"]
    private timezone = this.context["QUICKSIGHT_TIMEZONE"] ?? "Asia/Tokyo"


    constructor(scope: Construct, id: string, props: VisualizerStackProps) {
        super(scope, id, props);

        const table = props.table;

        const dataBucket = new Bucket(this, 'EmotionDataBucket', {
            removalPolicy: RemovalPolicy.RETAIN,
        })

        const exportStartLambda =  new RustFunction(this, 'ExportStartLambda', {
            // Path to the root directory.
            manifestPath: join(__dirname, '..', '..', 'lambdas/dyanmo_export_start_handler/'),
            environment: {
                'TABLE_ARN': table.tableArn,
                "BUCKET_NAME": dataBucket.bucketName
            },
            timeout: Duration.minutes(5)
        });
        exportStartLambda.addToRolePolicy(new PolicyStatement({
            effect: Effect.ALLOW,
            actions: [
                'dynamodb:ExportTableToPointInTime'
            ],
            resources: [table.tableArn],
        }))

        exportStartLambda.addToRolePolicy(new PolicyStatement({
            effect: Effect.ALLOW,
            actions: [
                "s3:AbortMultipartUpload",
                "s3:PutObject",
                "s3:PutObjectAcl"
            ],
            resources: [dataBucket.arnForObjects("*")],
        }))

        // Weekly SAT 15:00 UTC
        const weeklyRule = new Rule(this, 'ExportWeeklyRule', {
            schedule: Schedule.cron({
                minute: '00',
                hour: '15',
                weekDay: 'SAT',
            }),
            targets: [new LambdaFunction(exportStartLambda, {
                retryAttempts: 0
            })]
        })


        const exportFinishLambda =  new RustFunction(this, 'ExportFinishLambda', {
            // Path to the root directory.
            manifestPath: join(__dirname, '..', '..', 'lambdas/dyanmo_export_finish_handler/'),
            environment: {
                'PROCESSED_S3_FOLDER': this.processedFolder,
                "BUCKET_NAME": dataBucket.bucketName
            },
            timeout: Duration.minutes(5)
        });

        exportFinishLambda.addEventSource(new S3EventSource(dataBucket, {
            events: [EventType.OBJECT_CREATED],
            filters: [{
                suffix: "manifest-files.json"
            }]
        }))
        dataBucket.grantReadWrite(exportFinishLambda)

        const glueTable = new CfnTable(this, "EmotionDatatable", {
            databaseName: this.databaseName,
            catalogId: this.account,
            tableInput: {
                name: this.glueTableName,
                parameters: {
                    "classification": "json"
                },
                storageDescriptor: {
                    columns: [
                        {
                            "name": "item",
                            "type": "struct<event_id:struct<S:string>,surprise:struct<N:string>,timestamp:struct<N:string>,text:struct<S:string>,contempt:struct<N:string>,fear:struct<N:string>,joy:struct<N:string>,user_id:struct<S:string>,date:struct<S:string>,channel_id:struct<S:string>,month:struct<S:string>,sad:struct<N:string>,anger:struct<N:string>,channel_type:struct<S:string>,disgust:struct<N:string>>"
                        }
                    ],
                    inputFormat: "org.apache.hadoop.mapred.TextInputFormat",
                    outputFormat: "org.apache.hadoop.hive.ql.io.HiveIgnoreKeyTextOutputFormat",
                    serdeInfo: {
                        serializationLibrary: "org.openx.data.jsonserde.JsonSerDe",
                        parameters: {
                            "paths": "item"
                        }
                    },
                    location: dataBucket.s3UrlForObject(`${this.processedFolder}`)
                },

            },
        })

        const principal = `arn:aws:quicksight:${this.region}:${this.account}:user/default/${this.quicksightUserName}`

        const athenaAccessRole = new Role(this, "EmotionAthenaAccessRole", {
            roleName: "EmotionAthenaAccessRole",
            assumedBy: new ServicePrincipal('quicksight.amazonaws.com'),
            managedPolicies: [
                ManagedPolicy.fromAwsManagedPolicyName("service-role/AWSQuicksightAthenaAccess")
            ],
            inlinePolicies: {
                "EmotionQuickSightS3AcessPolicy": new PolicyDocument({
                    statements: [
                        new PolicyStatement({
                            effect: Effect.ALLOW,
                            actions: ["s3:ListAllMyBuckets"],
                            resources: ["arn:aws:s3:::*"]
                        }),
                        new PolicyStatement({
                            effect: Effect.ALLOW,
                            actions: [
                                "s3:ListBucket",
                                "s3:ListBucketMultipartUploads",
                                "s3:GetBucketLocation"
                            ],
                            resources: [dataBucket.bucketArn]
                        }),
                        new PolicyStatement({
                            effect: Effect.ALLOW,
                            actions: [
                                "s3:GetObject",
                                "s3:List*",
                                "s3:AbortMultipartUpload",
                                "s3:PutObject",
                                "s3:GetObjectVersion",
                                "s3:ListMultipartUploadParts"
                            ],
                            resources: [dataBucket.arnForObjects("*")]
                        }),
                    ]
                })
            }
        })


        const quicksightDatasource = new CfnDataSource(this, 'EmotionDataSource', {
            awsAccountId: this.account,
            dataSourceId: 'EmotionDataSource',
            name: 'EmotionDataSource',
            type: 'ATHENA',
            dataSourceParameters: {
                athenaParameters: {
                    // workGroup: 'primary',
                    roleArn: athenaAccessRole.roleArn
                },
            },
            permissions: [{
                principal: principal,
                actions: [
                    "quicksight:UpdateDataSourcePermissions",
                    "quicksight:DescribeDataSourcePermissions",
                    "quicksight:PassDataSource",
                    "quicksight:DescribeDataSource",
                    "quicksight:DeleteDataSource",
                    "quicksight:UpdateDataSource",
                ]
            }]
        });

        const quicksightDataset = new CfnDataSet(this, 'EmotionDataSet', {
            name: this.datasetName,
            awsAccountId: this.account,
            dataSetId: 'EmotionDataSet',
            physicalTableMap: {
                EmotionDataSetPhysicalTableMap: {
                    customSql: {
                        dataSourceArn: quicksightDatasource.attrArn,
                        columns: [
                            {
                                name: "user_id",
                                type: "STRING"
                            },
                            {
                                name: "event_id",
                                type: "STRING"
                            },
                            {
                                name: "text_message",
                                type: "STRING"
                            },
                            {
                                name: "channel_id",
                                type: "STRING"
                            },
                            {
                                name: "channel_type",
                                type: "STRING"
                            },
                            {
                                name: "date",
                                type: "DATETIME"
                            },
                            {
                                name: "month",
                                type: "STRING"
                            },
                            {
                                name: "anger",
                                type: "INTEGER"
                            },
                            {
                                name: "contempt",
                                type: "INTEGER"
                            },
                            {
                                name: "disgust",
                                type: "INTEGER"
                            },
                            {
                                name: "fear",
                                type: "INTEGER"
                            },
                            {
                                name: "joy",
                                type: "INTEGER"
                            },
                            {
                                name: "sad",
                                type: "INTEGER"
                            },
                            {
                                name: "surprise",
                                type: "INTEGER"
                            },
                            {
                                name: "timestamp",
                                type: "INTEGER"
                            }
                        ],
                        name: "EmotionDataCustomSql",
                        sqlQuery: `
                        SELECT
                            Item.user_id.S AS user_id,
                            Item.event_id.S AS event_id,
                            Item.text.S AS text_message,
                            Item.channel_id.S AS channel_id,
                            Item.channel_type.S AS channel_type,
                            CAST(Item.date.S AS date) AS date,
                            Item.month.S AS month,
                            CAST(Item.anger.N AS DECIMAL(38, 2)) AS anger,
                            CAST(Item.contempt.N AS DECIMAL(38, 2)) AS contempt,
                            CAST(Item.disgust.N AS DECIMAL(38, 2)) AS disgust,
                            CAST(Item.fear.N AS DECIMAL(38, 2)) AS fear,
                            CAST(Item.joy.N AS DECIMAL(38, 2)) AS joy,
                            CAST(Item.sad.N AS DECIMAL(38, 2)) AS sad,
                            CAST(Item.surprise.N AS DECIMAL(38, 2)) AS surprise,
                            CAST(Item.timestamp.N AS Int) AS timestamp
                        FROM "AwsDataCatalog"."${this.databaseName}"."${this.glueTableName}"
                        `
                    }
                }
            },
            logicalTableMap: {
                quickSightAthenaDataSetPhysicalTableMap: {
                    alias: this.glueTableName,
                    source: {
                        physicalTableId: 'EmotionDataSetPhysicalTableMap',
                    },
                },
            },
            importMode: 'DIRECT_QUERY',
            permissions: [{
                principal: principal,
                actions: [
                    "quicksight:PassDataSet",
                    "quicksight:DescribeIngestion",
                    "quicksight:CreateIngestion",
                    "quicksight:UpdateDataSet",
                    "quicksight:DeleteDataSet",
                    "quicksight:DescribeDataSet",
                    "quicksight:CancelIngestion",
                    "quicksight:DescribeDataSetPermissions",
                    "quicksight:ListIngestions",
                    "quicksight:UpdateDataSetPermissions"
                ],
            }],
            // datasetParameters
        });


        const sheetId = "EmotionDataSheet"
        const datetimeColumnName = "datetime"
        const userColumnName = "user_id"
        const negativeScoreColumnName = "NegativeEmotion"
        const datetime: CfnAnalysis.CalculatedFieldProperty = {
            dataSetIdentifier: this.datasetName,
            expression: "epochDate(timestamp)",
            name: datetimeColumnName
        }

        const negativeScore: CfnAnalysis.CalculatedFieldProperty = {
            dataSetIdentifier: this.datasetName,
            expression: "(anger+contempt+disgust)/3",
            name: negativeScoreColumnName
        }

        const dateFilterGroupId = "EmotionDataDateFilterGroup"
        const userFilterGroupId = "EmotionDataUserFilterGroup"
        const dateFilterId = "EmotionDataDateFilter"
        const userFilterId = "EmotionDataUserFilter"

        const dateFilterGroup: CfnAnalysis.FilterGroupProperty = {
            crossDataset: "SINGLE_DATASET",
            filterGroupId: dateFilterGroupId,
            filters: [{
                timeRangeFilter: {
                    column: {
                        columnName: datetimeColumnName,
                        dataSetIdentifier: this.datasetName
                    },
                    filterId: dateFilterId,
                    nullOption: "NON_NULLS_ONLY",
                    rangeMinimumValue: {
                        rollingDate: {
                            dataSetIdentifier: this.datasetName,
                            expression: "truncDate('MM', now())"
                        }
                    },
                    rangeMaximumValue: {
                        rollingDate: {
                            dataSetIdentifier: this.datasetName,
                            expression: "now()"
                        }
                    },
                    includeMinimum: true,
                    includeMaximum: true,
                    timeGranularity: "DAY"
                }
            }],
            scopeConfiguration: {
                selectedSheets: {
                    sheetVisualScopingConfigurations: [{
                        scope: "ALL_VISUALS",
                        sheetId: sheetId
                    }]
                }
            },
            status: "ENABLED"
        }

        const userFilterGroup: CfnAnalysis.FilterGroupProperty = {
            crossDataset: "SINGLE_DATASET",
            filterGroupId: userFilterGroupId,
            filters: [{
                categoryFilter: {
                    filterId: userFilterId,
                    column: {
                        columnName: userColumnName,
                        dataSetIdentifier: this.datasetName
                    },
                    configuration: {
                        filterListConfiguration: {
                            nullOption: "NON_NULLS_ONLY",
                            matchOperator: "CONTAINS",
                            selectAllOptions: "FILTER_ALL_VALUES"
                        }
                    }

                }
            }],
            scopeConfiguration: {
                selectedSheets: {
                    sheetVisualScopingConfigurations: [{
                        scope: "ALL_VISUALS",
                        sheetId: sheetId
                    }]
                }
            },
            status: "ENABLED"
        }

        const dateControl: CfnAnalysis.FilterControlProperty = {
            dateTimePicker: {
                commitMode: "AUTO",
                filterControlId: "DateControl",
                sourceFilterId: dateFilterId,
                title: "Date Range",
                type: "DATE_RANGE"
            }
        }
        const userControl: CfnAnalysis.FilterControlProperty = {
            dropdown: {
                displayOptions: {
                    selectAllOptions: {visibility: "VISIBLE"}
                },
                filterControlId: "UserControl",
                sourceFilterId: userFilterId,
                title: "User",
                type: "MULTI_SELECT"
            }
        }

        const tableVisual: CfnAnalysis.VisualProperty = {
            tableVisual: {
                title: {
                    formatText: { plainText: "Emotion Data Overview" }
                },
                visualId: "EmotionDataTableVisual",
                actions: [],
                chartConfiguration: {
                    // fieldOptions,
                    fieldWells: {
                        tableUnaggregatedFieldWells: {
                            values: [
                                {
                                    column: {
                                        columnName: userColumnName,
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-${userColumnName}`
                                },
                                {
                                    column: {
                                        columnName: datetime.name,
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-${datetime.name}`
                                },
                                {
                                    column: {
                                        columnName: "anger",
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-anger`

                                },
                                {
                                    column: {
                                        columnName: "contempt",
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-contempt`

                                },
                                {
                                    column: {
                                        columnName: "disgust",
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-disgust`

                                },
                                {
                                    column: {
                                        columnName: "surprise",
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-surprise`

                                },
                                {
                                    column: {
                                        columnName: "fear",
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-fear`

                                },
                                {
                                    column: {
                                        columnName: "joy",
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-joy`

                                },
                                {
                                    column: {
                                        columnName: "sad",
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTableVisual-sad`
                                }
                            ]
                        }
                    }
                }
                // conditionalFormatting: {

                // }
            }
        }

        const timeSeriesLineChart: CfnAnalysis.VisualProperty = {
            lineChartVisual: {
                title: {
                    formatText: { plainText: "Negative Score(Max) By Day" }
                },
                visualId: "EmotionDataTimeSeriesLineChart",
                chartConfiguration: {
                    fieldWells: {
                        lineChartAggregatedFieldWells: {
                            category: [{
                                dateDimensionField: {
                                    column: {
                                        columnName: datetime.name,
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTimeSeriesLineChart-${datetime.name}`,
                                    dateGranularity: "DAY"
                                }
                            }],
                            values: [{
                                numericalMeasureField: {
                                    aggregationFunction: {
                                        simpleNumericalAggregation: "MAX"
                                    },
                                    column: {
                                        columnName: negativeScore.name,
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTimeSeriesLineChart-${negativeScore.name}`
                                }
                            }],
                            colors: [{
                                categoricalDimensionField: {
                                    column: {
                                        columnName: userColumnName,
                                        dataSetIdentifier: this.datasetName
                                    },
                                    fieldId: `EmotionDataTimeSeriesLineChart-${userColumnName}`
                                }
                            }]
                        }
                    }
                }
            }
        }

        const quicksightAnalysis = new CfnAnalysis(this, "EmotionDataAnalysis",  {
            analysisId: "EmotionDataAnalysis",
            name: "EmotionDataAnalysis",
            awsAccountId: this.account,
            definition: {
                dataSetIdentifierDeclarations: [{
                    dataSetArn: quicksightDataset.attrArn,
                    identifier: this.datasetName
                }],
                analysisDefaults: {
                    defaultNewSheetConfiguration: {
                        interactiveLayoutConfiguration: {
                            grid: {
                                canvasSizeOptions: {
                                    screenCanvasSizeOptions: {
                                        resizeOption: "FIXED",
                                        optimizedViewPortWidth: "1600px"
                                    }
                                }
                            }
                        },
                        sheetContentType: "INTERACTIVE"
                    }
                },
                sheets: [{
                    sheetId: sheetId,
                    name: sheetId,
                    // title: sheetId,
                    filterControls: [dateControl, userControl],
                    sheetControlLayouts: undefined,
                    visuals: [tableVisual, timeSeriesLineChart],
                }],
                calculatedFields: [datetime, negativeScore],
                filterGroups: [dateFilterGroup, userFilterGroup],
                options: {
                    timezone: this.timezone
                }
            },
            permissions: [{
                principal: principal,
                actions: [
                    "quicksight:RestoreAnalysis",
                    "quicksight:UpdateAnalysisPermissions",
                    "quicksight:DeleteAnalysis",
                    "quicksight:QueryAnalysis",
                    "quicksight:DescribeAnalysisPermissions",
                    "quicksight:DescribeAnalysis",
                    "quicksight:UpdateAnalysis"
                ],
            }]
        })

        const quicksightTemplate = new CfnTemplate(this, "EmotionDataTemplate", {
            templateId: "EmotionDataTemplate",
            awsAccountId: this.account,
            name: "EmotionDataTemplate",
            permissions: [{
                principal: principal,
                actions: [
                    "quicksight:UpdateTemplatePermissions",
                    "quicksight:DescribeTemplatePermissions",
                    "quicksight:UpdateTemplateAlias",
                    "quicksight:DeleteTemplateAlias",
                    "quicksight:DescribeTemplateAlias",
                    "quicksight:ListTemplateAliases",
                    "quicksight:ListTemplates",
                    "quicksight:CreateTemplateAlias",
                    "quicksight:DeleteTemplate",
                    "quicksight:UpdateTemplate",
                    "quicksight:ListTemplateVersions",
                    "quicksight:DescribeTemplate",
                    "quicksight:CreateTemplate"
                ],
            }],
            sourceEntity: {
                sourceAnalysis: {
                    arn: quicksightAnalysis.attrArn,
                    dataSetReferences: [{
                        dataSetArn: quicksightDataset.attrArn,
                        dataSetPlaceholder: "EmotionDataSet"
                    }]
                }
            }
        })

        const quicksightDashboard = new CfnDashboard(this, "EmotionDataDashboard", {
            dashboardId: "EmotionDataDashboard",
            awsAccountId: this.account,
            name: "EmotionDataDashboard",
            permissions: [{
                principal: principal,
                actions: [
                    "quicksight:DescribeDashboard",
                    "quicksight:ListDashboardVersions",
                    "quicksight:UpdateDashboardPermissions",
                    "quicksight:QueryDashboard",
                    "quicksight:UpdateDashboard",
                    "quicksight:DeleteDashboard",
                    "quicksight:UpdateDashboardPublishedVersion",
                    "quicksight:DescribeDashboardPermissions"
                ]
            }],
            sourceEntity: {
                sourceTemplate: {
                    arn: quicksightTemplate.attrArn,
                    dataSetReferences: [{
                        dataSetArn: quicksightDataset.attrArn,
                        dataSetPlaceholder: "EmotionDataSet"
                    }]
                }
            }
        })

        quicksightDatasource.applyRemovalPolicy(RemovalPolicy.DESTROY)
        quicksightDataset.applyRemovalPolicy(RemovalPolicy.DESTROY)
        quicksightAnalysis.applyRemovalPolicy(RemovalPolicy.DESTROY)
        quicksightTemplate.applyRemovalPolicy(RemovalPolicy.DESTROY)
        quicksightDashboard.applyRemovalPolicy(RemovalPolicy.DESTROY)

    }

}