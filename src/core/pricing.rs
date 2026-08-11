use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPricing {
    pub name: String,
    pub cost_per_million_input: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostBreakdown {
    pub model_name: String,
    pub raw_cost: f64,
    pub cleaned_cost: f64,
    pub dollar_savings: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomPricingConfig {
    pub models: Vec<ModelPricing>,
}

fn get_tier_models() -> Vec<ModelPricing> {
    vec![
        ModelPricing {
            name: "Frontier Tier ($5.00/1M)".to_string(),
            cost_per_million_input: 5.00,
        },
        ModelPricing {
            name: "Production Tier ($0.50/1M)".to_string(),
            cost_per_million_input: 0.50,
        },
        ModelPricing {
            name: "Budget Tier ($0.05/1M)".to_string(),
            cost_per_million_input: 0.05,
        },
        ModelPricing {
            name: "Local Tier ($0.00/1M)".to_string(),
            cost_per_million_input: 0.00,
        },
    ]
}

pub fn load_model_pricings() -> Vec<ModelPricing> {
    if let Ok(path_str) = std::env::var("PANKH_PRICING_FILE") {
        let path = PathBuf::from(path_str);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<CustomPricingConfig>(&content) {
                return config.models;
            }
        }
    }

    if let Some(home) = dirs_next_or_home() {
        let config_path = home.join(".config").join("pankh").join("pricing.json");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<CustomPricingConfig>(&content) {
                return config.models;
            }
        }
    }

    get_tier_models()
}

fn dirs_next_or_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Calculates input token costs and dollar savings across model pricing tiers or custom CLI rate
pub fn estimate_costs(
    raw_tokens: usize,
    cleaned_tokens: usize,
    custom_rate: Option<f64>,
) -> Vec<CostBreakdown> {
    let models = if let Some(rate) = custom_rate {
        vec![ModelPricing {
            name: format!("Custom CLI Rate (${:.2}/1M)", rate),
            cost_per_million_input: rate,
        }]
    } else {
        load_model_pricings()
    };

    models
        .into_iter()
        .map(|m| {
            let raw_cost = (raw_tokens as f64 / 1_000_000.0) * m.cost_per_million_input;
            let cleaned_cost = (cleaned_tokens as f64 / 1_000_000.0) * m.cost_per_million_input;
            let dollar_savings = raw_cost - cleaned_cost;

            CostBreakdown {
                model_name: m.name,
                raw_cost: (raw_cost * 10_000.0).round() / 10_000.0,
                cleaned_cost: (cleaned_cost * 10_000.0).round() / 10_000.0,
                dollar_savings: (dollar_savings * 10_000.0).round() / 10_000.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_pricing_estimation() {
        let costs = estimate_costs(1_000_000, 500_000, None);
        assert_eq!(costs.len(), 4);
        let frontier = costs
            .iter()
            .find(|c| c.model_name.contains("Frontier"))
            .unwrap();
        assert_eq!(frontier.raw_cost, 5.0);
        assert_eq!(frontier.cleaned_cost, 2.5);
        assert_eq!(frontier.dollar_savings, 2.5);
    }

    #[test]
    fn test_custom_cli_rate_override() {
        let costs = estimate_costs(1_000_000, 500_000, Some(3.50));
        assert_eq!(costs.len(), 1);
        assert_eq!(costs[0].raw_cost, 3.50);
        assert_eq!(costs[0].dollar_savings, 1.75);
    }
}
