# Design Specification: Dynamic Future-Proof LLM Cost Estimator Engine

## Summary
Implement a future-proof LLM Cost Estimator engine (`src/core/pricing.rs`) for Pankh. Displays input token costs and dollar savings across `--stats` and `--diff-clean` CLI commands using model tier baselines (Frontier, Production, Budget, Local) and support for user-defined JSON pricing configs (`~/.config/pankh/pricing.json`), environment variables (`PANKH_PRICING_FILE`), or direct CLI rate overrides (`--price-per-m <RATE>`).

---

## 1. Future-Proof Tier-Based Pricing Architecture

To ensure Pankh is never obsolete regardless of how fast AI models evolve:

### Model Tier Baselines (`src/core/pricing.rs`):
- **Frontier Model Tier (e.g. Flagship Reasoning/Coding Models):** \$5.00 / 1M input tokens
- **Production Model Tier (e.g. Mid-tier Production Models):** \$0.50 / 1M input tokens
- **Budget / Open-Weight Tier (e.g. High-efficiency Models):** \$0.05 / 1M input tokens
- **Local Model Tier (e.g. Ollama / vLLM):** \$0.00 / 1M input tokens

### Dynamic Custom Configuration Overrides:
1. **CLI Rate Flag (`pankh --stats --price-per-m 3.00`):** Pass custom rate directly.
2. **Local JSON Config (`~/.config/pankh/pricing.json`):**
   ```json
   {
     "models": [
       { "name": "Frontier Tier", "cost_per_million_input": 5.00 },
       { "name": "Production Tier", "cost_per_million_input": 0.50 },
       { "name": "Budget Tier", "cost_per_million_input": 0.05 },
       { "name": "Local Tier", "cost_per_million_input": 0.00 }
     ]
   }
   ```
3. **Environment Variable (`PANKH_PRICING_FILE`):** Path to custom JSON pricing file.

---

## 2. Cost Calculation Formula

$$\text{Cost}(\text{tokens}, \text{rate}) = \frac{\text{tokens}}{1,000,000} \times \text{cost\_per\_million\_input}$$

$$\text{Dollar Savings} = \text{Cost}(\text{raw\_tokens}, \text{rate}) - \text{Cost}(\text{cleaned\_tokens}, \text{rate})$$

---

## 3. Data Structures & API (`src/core/pricing.rs`)

```rust
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

pub fn estimate_costs(raw_tokens: usize, cleaned_tokens: usize, custom_rate: Option<f64>) -> Vec<CostBreakdown>;
```

---

## 4. Verification Plan

### Automated Unit & Integration Tests
- `test_tier_pricing_estimation`: Verifies cost calculations across model tiers.
- `test_custom_cli_rate_override`: Verifies `--price-per-m` CLI rate override.
