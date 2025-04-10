# LLM Cost Exporter

Monitor cost metrics for llm providers that can be scraped and ingested by prometheus.

**Supported Providers**
* `openai`
* `anthropic`
* `bedrock`

## Instructions

### Prerequisites

Obvisouly, this is a prometheus exporter so make sure you have prometheus up and running.

#### Running on Kubernetes

To understand how to set this up on kubernetes beyond the basic helm installation, then refer to this example for [Cost Alerting on Kubernetes](./examples/k8s-cost-alerting/README.md)

**Installing helm chart**
 
* `OpenAI`
     ```
    helm upgrade --install llm-cost-monitor ./chart \
      --namespace ${NAMESPACE} \
      --set providers.openai.secretName=openai-api-secret
    ```
* `Anthropic`
    ```
    helm upgrade --install llm-cost-monitor ./chart \
      --namespace ${NAMESPACE} \
      --set providers.anthropic secretName=anthropic-api-secret
    ```
* `AWS Bedrock`

  - with credentials
    ```
    helm upgrade --install llm-cost-monitor ./llm-cost-monitor \
      --namespace llm-monitoring \
      --set providers.bedrock.credentials.enabled=true \
      --set providers.bedrock.credentials.aws_access_key_id=$AWS_ACCESS_KEY_ID \
      --set providers.bedrock.credentials.aws_secret_access_key=$AWS_SECRET_ACCESS_KEY
    ```
  - with multiple accounts
    ```
    helm upgrade --install llm-cost-monitor ./chart \
      --namespace llm-monitoring \
      --set providers.bedrock.enabled=true \
      --set providers.bedrock.iam.roleArn=arn:aws:iam::123456789012:role/bedrock-access-role
    ```

#### Running with Docker

Feel free to use the [Compose Stack Example](./examples/compose-stack/README.md) for a live local example

Otherwise, here is the basic instructions.

add the host of the llm exporter to your `prometheus.yml`.

```yaml
scrape_configs:
  - job_name: 'llm_cost_monitor'
    static_configs:
      - targets: ['localhost:8000']
```