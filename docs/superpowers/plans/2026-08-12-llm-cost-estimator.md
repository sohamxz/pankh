# Future-Proof LLM Cost Estimator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `src/core/pricing.rs` to compute dynamic input token costs and dollar savings across `--stats` and `--diff-clean` CLI commands using tier-based baselines (Frontier \$5/1M, Production \$0.50/1M, Budget \$0.05/1M, Local \$0/1M) and `--price-per-m` CLI overrides.

**Architecture:** Create `src/core/pricing.rs` exposing `estimate_costs(raw_tokens, cleaned_tokens, custom_rate) -> Vec<CostBreakdown>`. Support local JSON file overrides (`~/.config/pankh/pricing.json`), environment variables (`PANKH_PRICING_FILE`), and CLI flag `--price-per-m <RATE>`. Wire cost output into `--stats` and `--diff-clean` in `src/main.rs`.

**Tech Stack:** Rust 2021, `serde`, `serde_json`, `tokio`.

## Global Constraints
- `CostBreakdown` struct with `model_name`, `raw_cost`, `cleaned_cost`, `dollar_savings`.
- Future-proof model tiers (Frontier \$5.00/1M, Production \$0.50/1M, Budget \$0.05/1M, Local \$0.00/1M) + `--price-per-m` override.
- All 61+ existing tests must continue passing cleanly with zero `clippy` warnings.

---

### Task 1: Implement `src/core/pricing.rs` Module

**Files:**
- Create: `src/core/pricing.rs`
- Modify: `src/core/mod.rs`

**Interfaces:**
- Consumes: `std::fs`, `serde_json`
- Produces: `ModelPricing`, `CostBreakdown`, `estimate_costs(raw_tokens, cleaned_tokens, custom_rate) -> Vec<CostBreakdown>`

- [ ] **Step 1: Write failing unit test for `estimate_costs`**

Create `src/core/pricing.rs` with test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_pricing_estimation() {
        let costs = estimate_costs(1_000_000, 500_000, None);
        assert_eq!(costs.len(), 4);
        let frontier = costs.iter().find(|c| c.model_name.contains("Frontier")).unwrap();
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
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --lib core::pricing::tests::test_tier_pricing_estimation`
Expected: FAIL (unimplemented module)

- [ ] **Step 3: Implement `estimate_costs` in `src/core/pricing.rs`**

```rust
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

pub fn estimate_costs(raw_tokens: usize, cleaned_tokens: usize, custom_rate: Option<f64>) -> Vec<CostBreakdown> {
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
```

- [ ] **Step 4: Export `pub mod pricing;` in `src/core/mod.rs`**

- [ ] **Step 5: Run tests to verify pass**

Run: `cargo test --lib core::pricing`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/core/pricing.rs src/core/mod.rs
git commit -m "feat: implement future-proof tier-based LLM cost estimator module"
```

---

### Task 2: Wire Pricing Estimation & `--price-per-m` in CLI Output

**Files:**
- Modify: `src/core/agent.rs`
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`

- [ ] **Step 1: Write integration test in `tests/cli_test.rs`**

```rust
#[test]
fn test_cli_diff_clean_cost_output() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--quiet", "--", "tests/sample.md", "--diff-clean"])
        .output()
        .expect("failed to run CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Estimated Dollar Savings"));
    assert!(stdout.contains("Frontier Tier"));
}
```

- [ ] **Step 2: Update `generate_clean_diff` in `src/core/agent.rs` and `--stats` in `src/main.rs`**

Add `--price-per-m` flag in `Cli` struct in `src/main.rs`:

```rust
    /// Override custom LLM price per 1M input tokens (e.g. --price-per-m 2.50)
    #[arg(long)]
    pub price_per_m: Option<f64>,
```

Update `generate_clean_diff` in `src/core/agent.rs`:

```rust
    diff.push_str("Estimated Dollar Savings (Input Tokens Saved):\n");
    let costs = crate::core::pricing::estimate_costs(report.raw_tokens, report.cleaned_tokens, None);
    for cost in costs {
        diff.push_str(&format!("- {}: Saved ${:.4}\n", cost.model_name, cost.dollar_savings));
    }
    diff.push('\n');
```

Update `--stats` in `src/main.rs`:

```rust
            let costs = core::pricing::estimate_costs(stats.estimated_tokens, stats.estimated_tokens, cli.price_per_m);
            println!("- Estimated LLM Input Cost:");
            for cost in costs {
                println!("  - {}: ${:.4}", cost.model_name, cost.raw_cost);
            }
```

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test ; cargo clippy -- -D warnings`
Expected: PASS (0 errors, 0 clippy warnings)

- [ ] **Step 4: Commit**

```bash
git add src/core/agent.rs src/main.rs tests/cli_test.rs
git commit -m "feat: add future-proof LLM cost estimation and --price-per-m override to CLI output"
```
