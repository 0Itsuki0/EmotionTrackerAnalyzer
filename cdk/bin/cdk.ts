#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { EmotionHandlerStack } from '../lib/handler-stack';
import { EmotionDatabaseStack } from '../lib/database-stack';
import { EmotionVisualizerStack } from '../lib/visualize-stack';

const app = new cdk.App();

const contextKey = app.node.tryGetContext('context')
const context = app.node.tryGetContext(contextKey);
const region = context["REGION"] ?? process.env.CDK_DEFAULT_REGION

const dbStack = new EmotionDatabaseStack(app, 'ItsukiEmotionDBStack', {
    env: {
        region: region
    }
})
const handlerStack = new EmotionHandlerStack(app, 'ItsukiEmotionHandlerStack', {
    table: dbStack.table,
    env: {
        region: region
    }
})

const visualizerStack = new EmotionVisualizerStack(app, 'ItsukiEmotionVisualizerStack', {
    table: dbStack.table,
    env: {
        region: region
    }
})