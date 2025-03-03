use anyhow::{anyhow, Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Response structure for the Orca API
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaApiResponse {
    pub data: Vec<OrcaPoolInfo>,
    pub meta: OrcaMetaInfo,
}

/// Structure for Orca API metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaMetaInfo {
    pub cursor: OrcaCursor,
}

/// Structure for Orca API pagination cursor
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaCursor {
    pub previous: Option<String>,
    pub next: Option<String>,
}

/// Structure for an Orca pool
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaPoolInfo {
    pub address: String,
    #[serde(rename = "whirlpoolsConfig")]
    pub whirlpools_config: String,
    #[serde(rename = "whirlpoolBump")]
    pub whirlpool_bump: Vec<u8>,
    #[serde(rename = "tickSpacing")]
    pub tick_spacing: u16,
    #[serde(rename = "feeRate")]
    pub fee_rate: u32,
    #[serde(rename = "protocolFeeRate")]
    pub protocol_fee_rate: u32,
    pub liquidity: String,
    #[serde(rename = "sqrtPrice")]
    pub sqrt_price: String,
    #[serde(rename = "tickCurrentIndex")]
    pub tick_current_index: i32,
    #[serde(rename = "tokenMintA")]
    pub token_mint_a: String,
    #[serde(rename = "tokenVaultA")]
    pub token_vault_a: String,
    #[serde(rename = "tokenMintB")]
    pub token_mint_b: String,
    #[serde(rename = "tokenVaultB")]
    pub token_vault_b: String,
    pub price: String,
    #[serde(rename = "tvlUsdc")]
    pub tvl_usdc: String,
    #[serde(rename = "tokenBalanceA")]
    pub token_balance_a: String,
    #[serde(rename = "tokenBalanceB")]
    pub token_balance_b: String,
    #[serde(rename = "poolType")]
    pub pool_type: String,
    #[serde(rename = "tokenA")]
    pub token_a: OrcaTokenInfo,
    #[serde(rename = "tokenB")]
    pub token_b: OrcaTokenInfo,
    pub stats: OrcaStats,
    pub rewards: Vec<OrcaReward>,
}

/// Structure for token information
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaTokenInfo {
    pub address: String,
    #[serde(rename = "programId")]
    pub program_id: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(default, rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Structure for pool statistics
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaStats {
    #[serde(rename = "24h")]
    pub day: OrcaStatsPeriod,
    #[serde(rename = "7d")]
    pub week: OrcaStatsPeriod,
    #[serde(rename = "30d")]
    pub month: OrcaStatsPeriod,
}

/// Structure for period-specific statistics
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaStatsPeriod {
    pub volume: Option<String>,
    pub fees: Option<String>,
    pub rewards: Option<Value>,
    #[serde(rename = "yieldOverTvl")]
    pub yield_over_tvl: Option<String>,
}

/// Structure for reward information
#[derive(Debug, Deserialize, Serialize)]
pub struct OrcaReward {
    pub mint: String,
    pub vault: String,
    pub authority: String,
    pub emissions_per_second_x64: String,
    pub growth_global_x64: String,
    pub active: bool,
    #[serde(rename = "emissionsPerSecond")]
    pub emissions_per_second: String,
}

/// Fetches pool information from Orca API for the given token mints
///
/// # Arguments
///
/// * `token_a_mint` - The address of the first token mint
/// * `token_b_mint` - The address of the second token mint
/// * `limit` - Maximum number of results to return (optional, defaults to 50)
///
/// # Returns
///
/// Returns a Result containing the parsed pool information or an error
pub async fn fetch_orca_pools(
    token_a_mint: &str,
    token_b_mint: &str,
    limit: Option<u32>,
) -> Result<OrcaApiResponse> {
    // Set default limit if not provided
    let limit = limit.unwrap_or(50);

    // Build the API URL with query parameters
    let url = format!(
        "https://api.orca.so/v2/solana/pools?tokensBothOf={},{}&limit={}",
        token_a_mint, token_b_mint, limit
    );

    // Make the request
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to Orca API")?;

    // Check if the request was successful
    if !response.status().is_success() {
        return Err(anyhow!(
            "API request failed with status: {}",
            response.status()
        ));
    }

    // Get the response text for debugging if needed
    let response_text = response
        .text()
        .await
        .context("Failed to get response text from Orca API")?;

    // Parse the JSON text
    let pool_data: OrcaApiResponse =
        serde_json::from_str(&response_text).context("Failed to parse Orca API JSON response")?;

    Ok(pool_data)
}

/// Example usage of the Orca API
pub async fn orca_api_example_usage() -> Result<()> {
    let sol_mint = "So11111111111111111111111111111111111111112"; // wSOL
    let jup_mint = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"; // JUP

    let response = fetch_orca_pools(jup_mint, sol_mint, Some(10)).await?;

    println!("Found {} Orca pools", response.data.len());

    for (i, pool) in response.data.iter().enumerate() {
        println!(
            "Pool {}: {} <-> {}",
            i + 1,
            pool.token_a.symbol,
            pool.token_b.symbol
        );
        println!("  Address: {}", pool.address);
        println!("  Tick Spacing: {}", pool.tick_spacing);
        println!("  Fee Rate: {}%", pool.fee_rate as f64 / 10000.0);
        println!("  Pool Type: {}", pool.pool_type);
        println!("  Price: {}", pool.price);
        println!("  TVL (USD): {}", pool.tvl_usdc);

        // Get 24h volume if available
        if let Some(volume) = &pool.stats.day.volume {
            println!("  24h Volume: ${}", volume);
        }

        // Get 24h fees if available
        if let Some(fees) = &pool.stats.day.fees {
            println!("  24h Fees: ${}", fees);
        }

        println!();
    }

    Ok(())
}
