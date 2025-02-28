use anyhow::{anyhow, Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MeteoraPoolResponse {
    pub data: Vec<PoolInfo>,
    pub page: u32,
    pub total_count: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PoolInfo {
    pub pool_address: String,
    pub pool_token_mints: Vec<String>,
    pub pool_token_amounts: Vec<String>,
    pub pool_token_usd_amounts: Vec<String>,
    pub vaults: Vec<String>,
    pub vault_lps: Vec<String>,
    pub lp_mint: String,
    pub pool_tvl: String,
    pub farm_tvl: String,
    pub farming_pool: Option<String>,
    pub farming_apy: String,
    pub is_monitoring: bool,
    pub pool_order: u32,
    pub farm_order: u32,
    pub pool_version: u32,
    pub pool_name: String,
    pub lp_decimal: u32,
    pub farm_reward_duration_end: u64,
    pub farm_expire: bool,
    pub pool_lp_price_in_usd: String,
    pub trading_volume: f64,
    pub fee_volume: f64,
    pub weekly_trading_volume: f64,
    pub weekly_fee_volume: f64,
    pub yield_volume: String,
    pub accumulated_trading_volume: String,
    pub accumulated_fee_volume: String,
    pub accumulated_yield_volume: String,
    pub trade_apy: String,
    pub weekly_trade_apy: String,
    pub daily_base_apy: String,
    pub weekly_base_apy: String,
    pub apr: f64,
    pub farm_new: bool,
    pub permissioned: bool,
    pub unknown: bool,
    pub total_fee_pct: String,
    pub is_lst: bool,
    pub is_forex: bool,
    pub created_at: u64,
    pub is_meme: bool,
    pub pool_type: String,
}

/// Fetches pool information from Meteora for the given token mints
///
/// # Arguments
///
/// * `token_a_mint` - The address of the first token mint
/// * `token_b_mint` - The address of the second token mint
/// * `page` - Page number (optional, defaults to 1)
/// * `size` - Number of results per page (optional, defaults to 10)
///
/// # Returns
///
/// Returns a Result containing the parsed pool information or an error
pub async fn fetch_meteora_pools(
    token_a_mint: &str,
    token_b_mint: &str,
    page: Option<u32>,
    size: Option<u32>,
) -> Result<MeteoraPoolResponse> {
    // Set default pagination values if not provided
    let page = page.unwrap_or(1);
    let size = size.unwrap_or(10);

    // Build the API URL with query parameters
    // Sort the token mints alphabetically to ensure consistent requests
    let token_pair = if token_a_mint < token_b_mint {
        format!("{}-{}", token_a_mint, token_b_mint)
    } else {
        format!("{}-{}", token_b_mint, token_a_mint)
    };

    let url = format!(
        "https://amm-v2.meteora.ag/pools/search?page={}&size={}&include_pool_token_pairs={}",
        page, size, token_pair
    );

    // Make the request
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to Meteora API")?;

    // Check if the request was successful
    if !response.status().is_success() {
        return Err(anyhow!(
            "API request failed with status: {}",
            response.status()
        ));
    }

    // Get the response text first for debugging if needed
    let response_text = response
        .text()
        .await
        .context("Failed to get response text from Meteora API")?;

    // Parse the JSON text
    let pool_data: MeteoraPoolResponse = serde_json::from_str(&response_text)
        .context("Failed to parse Meteora API JSON response")?;

    Ok(pool_data)
}

/// Example usage of the Meteora pool finder
pub async fn meteora_example_usage() -> Result<()> {
    let sol_mint = "So11111111111111111111111111111111111111112"; // wSOL
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC

    let pools = fetch_meteora_pools(sol_mint, usdc_mint, Some(1), Some(1)).await?;

    println!(
        "Found {} Meteora pools (page {} of {})",
        pools.data.len(),
        pools.page,
        (pools.total_count as f64 / 10.0).ceil() as u32
    );

    for (i, pool) in pools.data.iter().enumerate() {
        println!("Pool {}: {}", i + 1, pool.pool_name);
        println!("  Address: {}", pool.pool_address);
        println!(
            "  Token Mints: {} <-> {}",
            pool.pool_token_mints[0], pool.pool_token_mints[1]
        );
        println!(
            "  Token Amounts: {} <-> {}",
            pool.pool_token_amounts[0], pool.pool_token_amounts[1]
        );
        // Find the indices for SOL and USDC in the pool tokens
        let (sol_idx, usdc_idx) =
            if pool.pool_token_mints[0] == "So11111111111111111111111111111111111111112" {
                (0, 1)
            } else {
                (1, 0)
            };

        // Calculate the price (USDC amount / SOL amount) for SOL-USDC pools
        let price = match (
            pool.pool_token_amounts[sol_idx].parse::<f64>(),
            pool.pool_token_amounts[usdc_idx].parse::<f64>(),
        ) {
            (Ok(sol_amount), Ok(usdc_amount)) if sol_amount > 0.0 => usdc_amount / sol_amount,
            _ => 0.0, // Handle parsing errors or division by zero
        };

        println!("  TVL: ${}", pool.pool_tvl);
        println!("  Price: {:.6} USDC/SOL", price);
        println!("  24h Trading Volume: ${:.2}", pool.trading_volume);
        println!("  Fee: {}%", pool.total_fee_pct);
        println!("  APR: {:.2}%", pool.apr);
        println!("  Pool Type: {}", pool.pool_type);
        println!();
    }

    Ok(())
}
