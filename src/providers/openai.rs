pub struct OpenAIMonitor {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIUsageResponse {
    pub total_usage: f64,
}

#[derive(Debug, Deserialize)]
pub struct OpenAISubscriptionResponse {
    pub hard_limit_usd: f64,
    pub has_payment_method: bool,
}

impl OpenAIMonitor {
    pub async fn get_usage_data(&self) -> Result<OpenAIUsageResponse, MonitorError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.openai.com/v1/usage")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .json::<OpenAIUsageResponse>()
            .await?;
        Ok(response)
    }

    pub async fn get_subscription_data(&self) -> Result<OpenAISubscriptionResponse, MonitorError> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.openai.com/v1/dashboard/billing/subscription")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .json::<OpenAISubscriptionResponse>()
            .await?;
        Ok(response)
    }
}
