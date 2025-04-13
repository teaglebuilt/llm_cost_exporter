use serde::Deserialize;

use crate::MonitorError;

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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_usage_data() {
        let mut server = Server::new();
        let mock_response = json!({
            "total_usage": 10000 // $100.00 in cents
        });

        let _m = server
            .mock("GET", "/v1/usage")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let monitor = OpenAIMonitor {
            api_key: "test_key".to_string(),
        };

        let usage = monitor.get_usage_data().await.unwrap();
        assert_eq!(usage.total_usage, 10000.0);
    }

    #[tokio::test]
    async fn test_get_subscription_data() {
        let mut server = Server::new();
        let mock_response = json!({
            "hard_limit_usd": 200.0,
            "has_payment_method": true
        });

        let _m = server
            .mock("GET", "/v1/dashboard/billing/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let monitor = OpenAIMonitor {
            api_key: "test_key".to_string(),
        };

        let sub = monitor.get_subscription_data().await.unwrap();
        assert_eq!(sub.hard_limit_usd, 200.0);
        assert!(sub.has_payment_method);
    }

    #[tokio::test]
    async fn test_get_usage_with_balance() {
        let mut server = Server::new();

        let _usage_mock = server
            .mock("GET", "/v1/usage")
            .with_body(json!({"total_usage": 5000}).to_string())
            .create();

        let _sub_mock = server
            .mock("GET", "/v1/dashboard/billing/subscription")
            .with_body(
                json!({
                    "hard_limit_usd": 100.0,
                    "has_payment_method": true
                })
                .to_string(),
            )
            .create();

        let monitor = OpenAIMonitor {
            api_key: "test_key".to_string(),
        };

        let usage = monitor.get_usage_data().await.unwrap();
        assert_eq!(usage.total_usage, 50.0); // $50 from 5000 cents
    }
}
