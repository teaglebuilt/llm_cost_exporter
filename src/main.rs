use anyhow::Context;
use async_trait::async_trait;
use prometheus::{opts, Encoder, GaugeVec, Registry, TextEncoder};
use std::time::Duration;
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

#[async_trait]
impl LLMMonitor for OpenAIMonitor {
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

#[tokio::main]
async fn main() -> Result<(), MonitorError> {
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let openai_monitor = OpenAIMonitor { api_key: api_key };

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
    }
}
