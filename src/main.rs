use anyhow::{anyhow, Context};
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_sts::Client as StsClient;
use prometheus::{opts, Encoder, GaugeVec, Registry, TextEncoder};
use std::time::Duration;
use std::time::UNIX_EPOCH;
use thiserror::Error;
use tokio::time;

#[derive(Debug)]
pub struct BedrockConfig {
    pub assume_role: AssumeRoleConfig,
}

#[derive(Debug)]
pub struct AssumeRoleConfig {
    pub enabled: bool,
    pub role_arn: String,
    pub session_name: String,
}

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("API request failed")]
    ApiError(#[from] reqwest::Error),
    #[error("AWS SDK error")]
    AwsError(#[from] aws_sdk_bedrockruntime::Error),
    #[error("Invalid response format")]
    InvalidResponse,
    #[error("Configuration error: {0}")]
    ConfigError(#[from] anyhow::Error),
}

#[async_trait]
trait LLMMonitor {
    async fn get_usage(&self) -> Result<LLMUsage, MonitorError>;
}

#[derive(Debug, Default)]
struct LLMUsage {
    pub cost_usd: f64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub request_count: u64,
}

struct OpenAIMonitor {
    #[allow(dead_code)]
    api_key: String,
}

struct BedrockMonitor {
    #[allow(dead_code)]
    client: aws_sdk_bedrockruntime::Client,
}

struct ClaudeMonitor {
    #[allow(dead_code)]
    api_key: String,
}

#[async_trait]
impl LLMMonitor for OpenAIMonitor {
    async fn get_usage(&self) -> Result<LLMUsage, MonitorError> {
        Ok(LLMUsage::default())
    }
}

#[async_trait]
impl LLMMonitor for BedrockMonitor {
    async fn get_usage(&self) -> Result<LLMUsage, MonitorError> {
        Ok(LLMUsage::default())
    }
}

#[async_trait]
impl LLMMonitor for ClaudeMonitor {
    async fn get_usage(&self) -> Result<LLMUsage, MonitorError> {
        Ok(LLMUsage::default())
    }
}

struct LLMMetrics {
    cost: GaugeVec,
    tokens: GaugeVec,
    requests: GaugeVec,
}

impl LLMMetrics {
    fn new(registry: &Registry) -> Self {
        let cost = GaugeVec::new(
            opts!("llm_cost_usd", "Cost of LLM API usage in USD"),
            &["provider", "model"],
        )
        .unwrap();

        let tokens = GaugeVec::new(
            opts!("llm_tokens", "Tokens used by LLM API"),
            &["provider", "model", "type"],
        )
        .unwrap();

        let requests = GaugeVec::new(
            opts!("llm_requests", "Number of LLM API requests"),
            &["provider", "model"],
        )
        .unwrap();

        registry.register(Box::new(cost.clone())).unwrap();
        registry.register(Box::new(tokens.clone())).unwrap();
        registry.register(Box::new(requests.clone())).unwrap();

        Self {
            cost,
            tokens,
            requests,
        }
    }

    fn update(&self, provider: &str, model: &str, usage: &LLMUsage) {
        self.cost
            .with_label_values(&[provider, model])
            .set(usage.cost_usd);
        self.tokens
            .with_label_values(&[provider, model, "prompt"])
            .set(usage.prompt_tokens as f64);
        self.tokens
            .with_label_values(&[provider, model, "completion"])
            .set(usage.completion_tokens as f64);
        self.requests
            .with_label_values(&[provider, model])
            .set(usage.request_count as f64);
    }
}

async fn run_metrics_server(registry: Registry) -> Result<(), std::io::Error> {
    use warp::Filter;

    let metrics_route = warp::path!("metrics").map(move || {
        let encoder = TextEncoder::new();
        let mut buffer = vec![];
        let metric_families = registry.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    });

    warp::serve(metrics_route).run(([0, 0, 0, 0], 8000)).await;

    Ok(())
}

async fn create_bedrock_client(
    config: &BedrockConfig,
) -> Result<aws_sdk_bedrockruntime::Client, MonitorError> {
    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

    if config.assume_role.enabled {
        let sts_client = StsClient::new(&shared_config);
        let assumed_role = sts_client
            .assume_role()
            .role_arn(&config.assume_role.role_arn)
            .role_session_name(&config.assume_role.session_name)
            .send()
            .await
            .map_err(|e| MonitorError::ConfigError(anyhow!(e)))?;

        let creds = assumed_role
            .credentials
            .ok_or_else(|| MonitorError::ConfigError(anyhow!("No credentials in STS response")))?;

        let expiration = UNIX_EPOCH + Duration::from_secs(creds.expiration.secs() as u64);

        let aws_creds = Credentials::new(
            creds.access_key_id,
            creds.secret_access_key,
            Some(creds.session_token),
            Some(expiration),
            "assumed-role",
        );

        let config = aws_sdk_bedrockruntime::config::Builder::from(&shared_config)
            .credentials_provider(aws_creds)
            .build();

        Ok(aws_sdk_bedrockruntime::Client::from_conf(config))
    } else {
        Ok(aws_sdk_bedrockruntime::Client::new(&shared_config))
    }
}

#[tokio::main]
async fn main() -> Result<(), MonitorError> {
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let openai_monitor = OpenAIMonitor { api_key: api_key };

    let bedrock_config = BedrockConfig {
        assume_role: AssumeRoleConfig {
            enabled: false,
            role_arn: "".to_string(),
            session_name: "".to_string(),
        },
    };

    let bedrock_monitor = BedrockMonitor {
        client: create_bedrock_client(&bedrock_config).await?,
    };

    let claude_monitor = ClaudeMonitor {
        api_key: std::env::var("ANTHROPIC_API_KEY")
            .map_err(|e| MonitorError::ConfigError(anyhow!("ANTHROPIC_API_KEY not set: {}", e)))?,
    };

    let registry = Registry::new();
    let metrics = LLMMetrics::new(&registry);

    tokio::spawn(async move {
        if let Err(e) = run_metrics_server(registry).await {
            eprintln!("Metrics server error: {}", e);
            // log this or return wrapped error
        }
    });

    // monitoring loop
    let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes

    loop {
        interval.tick().await;
        if let Ok(usage) = openai_monitor.get_usage().await {
            metrics.update("openai", "gpt-4", &usage);
        }
        if let Ok(usage) = bedrock_monitor.get_usage().await {
            metrics.update("bedrock", "claude-2", &usage);
        }
        if let Ok(usage) = claude_monitor.get_usage().await {
            metrics.update("anthropic", "claude-2", &usage);
        }
    }
}
