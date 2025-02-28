use anyhow::{anyhow, Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};

// Define structures that match the JSON response
#[derive(Debug, Deserialize, Serialize)]
pub struct RaydiumPoolResponse {
    pub id: String,
    pub success: bool,
    pub data: PoolData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PoolData {
    pub count: u32,
    #[serde(rename = "data")]
    pub pools: Vec<PoolInfo>,
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PoolInfo {
    #[serde(rename = "type")]
    pub pool_type: String,
    #[serde(rename = "programId")]
    pub program_id: String,
    pub id: String,
    #[serde(rename = "mintA")]
    pub mint_a: TokenInfo,
    #[serde(rename = "mintB")]
    pub mint_b: TokenInfo,
    pub price: f64,
    #[serde(rename = "mintAmountA")]
    pub mint_amount_a: f64,
    #[serde(rename = "mintAmountB")]
    pub mint_amount_b: f64,
    #[serde(rename = "feeRate")]
    pub fee_rate: f64,
    pub tvl: f64,
    pub day: PeriodInfo,
    pub week: PeriodInfo,
    pub month: PeriodInfo,
    // Additional fields can be added as needed
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenInfo {
    #[serde(rename = "chainId")]
    pub chain_id: u32,
    pub address: String,
    #[serde(rename = "programId")]
    pub program_id: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    // Additional token fields can be added as needed
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PeriodInfo {
    pub volume: f64,
    #[serde(rename = "volumeQuote")]
    pub volume_quote: f64,
    #[serde(rename = "volumeFee")]
    pub volume_fee: f64,
    pub apr: f64,
    #[serde(rename = "feeApr")]
    pub fee_apr: f64,
    #[serde(rename = "priceMin")]
    pub price_min: f64,
    #[serde(rename = "priceMax")]
    pub price_max: f64,
    #[serde(rename = "rewardApr")]
    pub reward_apr: Vec<f64>,
}

/// Fetches pool information from Raydium for the given token mints
///
/// # Arguments
///
/// * `mint1` - The address of the first token mint
/// * `mint2` - The address of the second token mint
/// * `page_size` - Number of results per page (optional, defaults to 10)
/// * `page` - Page number (optional, defaults to 1)
///
/// # Returns
///
/// Returns a Result containing the parsed pool information or an error
pub async fn fetch_raydium_pools(
    mint1: &str,
    mint2: &str,
    page_size: Option<u32>,
    page: Option<u32>,
) -> Result<RaydiumPoolResponse> {
    // Set default pagination values if not provided
    let page_size = page_size.unwrap_or(10);
    let page = page.unwrap_or(1);

    // Build the API URL with query parameters
    let url = format!(
        "https://api-v3.raydium.io/pools/info/mint?mint1={}&mint2={}&poolType=all&poolSortField=default&sortType=desc&pageSize={}&page={}",
        mint1, mint2, page_size, page
    );

    // Make the request
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to Raydium API")?;

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
        .context("Failed to get response text from Raydium API")?;

    // Parse the JSON text
    let pool_data: RaydiumPoolResponse = serde_json::from_str(&response_text)
        .context("Failed to parse Raydium API JSON response")?;

    Ok(pool_data)
}

// Example usage
pub async fn raydium_example_usage() -> Result<()> {
    let sol_mint = "So11111111111111111111111111111111111111112";
    let jup_mint = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

    let pools = fetch_raydium_pools(sol_mint, jup_mint, Some(2), Some(1)).await?;

    if pools.success {
        println!("Found {} pools", pools.data.count);

        for (i, pool) in pools.data.pools.iter().enumerate() {
            println!(
                "Pool {}: {} <-> {}",
                i + 1,
                pool.mint_a.symbol,
                pool.mint_b.symbol
            );
            println!("  ID: {}", pool.id);
            println!("  Price: {}", pool.price);
            println!("  TVL: ${:.2}", pool.tvl);
            println!("  24h Volume: ${:.2}", pool.day.volume);
            println!("  Fee Rate: {:.4}%", pool.fee_rate * 100.0);
            println!();
        }
    } else {
        println!("API request was not successful");
    }

    Ok(())
}
