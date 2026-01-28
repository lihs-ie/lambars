#!/bin/bash
set -e

echo "Initializing LocalStack SQS queues..."

awslocal sqs create-queue \
    --queue-name bank-events-dlq \
    --attributes '{
        "MessageRetentionPeriod": "1209600"
    }'

DLQ_ARN=$(awslocal sqs get-queue-attributes \
    --queue-url http://localhost:4566/000000000000/bank-events-dlq \
    --attribute-names QueueArn \
    --query 'Attributes.QueueArn' \
    --output text)

awslocal sqs create-queue \
    --queue-name bank-events \
    --attributes '{
        "VisibilityTimeout": "30",
        "MessageRetentionPeriod": "345600",
        "RedrivePolicy": "{\"deadLetterTargetArn\":\"'"$DLQ_ARN"'\",\"maxReceiveCount\":\"3\"}"
    }'

awslocal sqs create-queue \
    --queue-name bank-projections-dlq \
    --attributes '{
        "MessageRetentionPeriod": "1209600"
    }'

PROJ_DLQ_ARN=$(awslocal sqs get-queue-attributes \
    --queue-url http://localhost:4566/000000000000/bank-projections-dlq \
    --attribute-names QueueArn \
    --query 'Attributes.QueueArn' \
    --output text)

awslocal sqs create-queue \
    --queue-name bank-projections \
    --attributes '{
        "VisibilityTimeout": "60",
        "MessageRetentionPeriod": "345600",
        "RedrivePolicy": "{\"deadLetterTargetArn\":\"'"$PROJ_DLQ_ARN"'\",\"maxReceiveCount\":\"3\"}"
    }'

echo "SQS queues created successfully:"
awslocal sqs list-queues

echo "LocalStack initialization complete!"
