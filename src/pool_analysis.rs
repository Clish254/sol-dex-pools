use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Structure to hold standardized pool information across different AMMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardizedPool {
    /// Which AMM this pool belongs to (Raydium, Orca, Meteora)
    pub amm: String,
    /// Name of the pool (usually token pair)
    pub name: String,
    /// Pool's on-chain address
    pub address: String,
    /// Current token price in USD
    pub price_usd: f64,
    /// Total liquidity value in USD
    pub liquidity_usd: f64,
    /// Trading volume in USD (24h)
    pub volume_24h: Option<f64>,
    /// Trading fee percentage
    pub fee_percentage: f64,
    /// Token addresses in the pool
    pub token_addresses: Vec<String>,
    /// Additional metadata specific to each AMM
    pub metadata: serde_json::Value,
}

/// Pool health analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolHealthAnalysis {
    /// Original pool data
    pub pool: StandardizedPool,
    /// Overall health score (0.0 to 1.0)
    pub health_score: f64,
    /// Liquidity score component (0.0 to 1.0)
    pub liquidity_score: f64,
    /// Volume score component (0.0 to 1.0)
    pub volume_score: f64,
    /// Fee score component (0.0 to 1.0, lower fees = higher score)
    pub fee_score: f64,
    /// Price stability score (0.0 to 1.0)
    pub price_stability: Option<f64>,
}

/// Structure for configuring the health score calculation
#[derive(Debug, Clone)]
pub struct HealthScoreConfig {
    /// Weight for liquidity in overall score (default: 0.5)
    pub liquidity_weight: f64,
    /// Weight for trading volume in overall score (default: 0.3)
    pub volume_weight: f64,
    /// Weight for fee in overall score (default: 0.1)
    pub fee_weight: f64,
    /// Weight for price stability in overall score (default: 0.1)
    pub stability_weight: f64,
    /// Maximum expected liquidity for normalization (in USD)
    pub max_expected_liquidity: f64,
    /// Maximum expected volume for normalization (in USD)
    pub max_expected_volume: f64,
    /// Maximum expected fee (higher than this gets minimum score)
    pub max_expected_fee: f64,
}

impl Default for HealthScoreConfig {
    fn default() -> Self {
        Self {
            liquidity_weight: 0.5,
            volume_weight: 0.3,
            fee_weight: 0.1,
            stability_weight: 0.1,
            max_expected_liquidity: 10_000_000.0, // $10M
            max_expected_volume: 5_000_000.0,     // $5M
            max_expected_fee: 1.0,                // 1%
        }
    }
}

/// Calculate health score for a pool
pub fn calculate_health_score(
    pool: &StandardizedPool,
    config: &HealthScoreConfig,
) -> PoolHealthAnalysis {
    // Calculate liquidity score (logarithmic scale)
    let liquidity_score = if pool.liquidity_usd > 0.0 {
        let log_score =
            (pool.liquidity_usd.log10() / config.max_expected_liquidity.log10()).min(1.0);
        log_score.max(0.0)
    } else {
        0.0
    };

    // Calculate volume score (logarithmic scale)
    let volume_score = match pool.volume_24h {
        Some(volume) if volume > 0.0 => {
            let log_score = (volume.log10() / config.max_expected_volume.log10()).min(1.0);
            log_score.max(0.0)
        }
        _ => 0.0,
    };

    // Calculate fee score (lower is better, so invert)
    let fee_score = (1.0 - (pool.fee_percentage / config.max_expected_fee)).max(0.0);

    // Price stability is optional and may not be available for all pools
    let price_stability = None; // This would require historical data

    // Calculate composite health score
    let mut health_score = (liquidity_score * config.liquidity_weight)
        + (volume_score * config.volume_weight)
        + (fee_score * config.fee_weight);

    // Add stability component if available
    if let Some(stability) = price_stability {
        health_score += stability * config.stability_weight;
    }

    PoolHealthAnalysis {
        pool: pool.clone(),
        health_score,
        liquidity_score,
        volume_score,
        fee_score,
        price_stability,
    }
}

/// Find the healthiest pool from a list based on calculated health scores
pub fn find_healthiest_pool(pools: &[StandardizedPool]) -> Option<PoolHealthAnalysis> {
    if pools.is_empty() {
        return None;
    }

    let config = HealthScoreConfig::default();

    pools
        .iter()
        .map(|pool| calculate_health_score(pool, &config))
        .max_by(|a, b| {
            // Compare by health score, handling potential NaN values
            match (a.health_score.is_nan(), b.health_score.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => a
                    .health_score
                    .partial_cmp(&b.health_score)
                    .unwrap_or(Ordering::Equal),
            }
        })
}

/// Convert token amount to USD based on token type and current prices
pub fn convert_to_usd(
    token_address: &str,
    token_amount: f64,
    sol_price_usd: f64,
    known_token_prices: &[(String, f64)],
) -> Option<f64> {
    // Check if this is SOL
    if token_address == "So11111111111111111111111111111111111111112" {
        return Some(token_amount * sol_price_usd);
    }

    // Check if we have a price for this token
    for (address, price) in known_token_prices {
        if address == token_address {
            return Some(token_amount * price);
        }
    }

    None
}
