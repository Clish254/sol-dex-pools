use anyhow::{anyhow, Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};

/// Response structure for the Meteora DLMM API
#[derive(Debug, Deserialize, Serialize)]
pub struct MeteoraGroupsResponse {
    pub groups: Vec<DlmmGroup>,
    pub total: u32,
}

/// Structure for a DLMM group
#[derive(Debug, Deserialize, Serialize)]
pub struct DlmmGroup {
    pub name: String,
    pub pairs: Vec<DlmmPair>,
}

/// Structure for a token pair
#[derive(Debug, Deserialize, Serialize)]
pub struct DlmmPair {
    pub address: String,
    pub name: String,
    pub mint_x: String,
    pub mint_y: String,
    pub reserve_x: String,
    pub reserve_y: String,
    pub reserve_x_amount: u64,
    pub reserve_y_amount: u64,
    pub bin_step: u32,
    pub base_fee_percentage: String,
    pub max_fee_percentage: String,
    pub protocol_fee_percentage: String,
    pub liquidity: String,
    pub reward_mint_x: String,
    pub reward_mint_y: String,
    pub fees_24h: f64,
    pub today_fees: f64,
    pub trade_volume_24h: f64,
    pub cumulative_trade_volume: String,
    pub cumulative_fee_volume: String,
    pub current_price: f64,
    pub apr: f64,
    pub apy: f64,
    pub farm_apr: f64,
    pub farm_apy: f64,
    pub hide: bool,
    pub is_blacklisted: bool,
    pub fees: DlmmFees,
    pub fee_tvl_ratio: DlmmFees,
    pub volume: DlmmFees,
}

/// Structure for DLMM time-based metrics
#[derive(Debug, Deserialize, Serialize)]
pub struct DlmmFees {
    #[serde(rename = "min_30")]
    pub min_30: f64,
    #[serde(rename = "hour_1")]
    pub hour_1: f64,
    #[serde(rename = "hour_2")]
    pub hour_2: f64,
    #[serde(rename = "hour_4")]
    pub hour_4: f64,
    #[serde(rename = "hour_12")]
    pub hour_12: f64,
    #[serde(rename = "hour_24")]
    pub hour_24: f64,
}

/// Fetches DLMM pool information from Meteora for the given token mints
///
/// # Arguments
///
/// * `token_a_mint` - The address of the first token mint
/// * `token_b_mint` - The address of the second token mint
/// * `page` - Page number (optional, defaults to 0)
/// * `limit` - Number of results per page (optional, defaults to 10)
///
/// # Returns
///
/// Returns a Result containing the parsed DLMM pool information or an error
pub async fn fetch_meteora_dlmm_pools(
    token_a_mint: &str,
    token_b_mint: &str,
    page: Option<u32>,
    limit: Option<u32>,
) -> Result<MeteoraGroupsResponse> {
    // Set default pagination values if not provided
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    // Build the API URL with query parameters
    // Sort the token mints alphabetically to ensure consistent requests
    let token_pair = if token_a_mint < token_b_mint {
        format!("{}-{}", token_a_mint, token_b_mint)
    } else {
        format!("{}-{}", token_b_mint, token_a_mint)
    };

    let url = format!(
        "https://dlmm-api.meteora.ag/pair/all_by_groups?page={}&limit={}&include_pool_token_pairs={}",
        page, limit, token_pair
    );

    // Make the request
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to Meteora DLMM API")?;

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
        .context("Failed to get response text from Meteora DLMM API")?;

    // Parse the JSON text
    let pool_data: MeteoraGroupsResponse = serde_json::from_str(&response_text)
        .context("Failed to parse Meteora DLMM API JSON response")?;

    Ok(pool_data)
}

/// Example usage of the Meteora DLMM pool finder
pub async fn meteora_dlmm_example_usage() -> Result<()> {
    let sol_mint = "So11111111111111111111111111111111111111112"; // wSOL
    let jup_mint = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"; // JUP

    let response = fetch_meteora_dlmm_pools(jup_mint, sol_mint, Some(0), Some(10)).await?;

    println!(
        "Found {} Meteora DLMM groups (total: {})",
        response.groups.len(),
        response.total
    );

    for (i, group) in response.groups.iter().enumerate() {
        println!("Group {}: {}", i + 1, group.name);
        println!("  Number of pairs: {}", group.pairs.len());

        for (j, pair) in group.pairs.iter().enumerate() {
            println!("  Pair {}.{}: {}", i + 1, j + 1, pair.name);
            println!("    Address: {}", pair.address);
            println!("    Bin Step: {}", pair.bin_step);
            println!("    Base Fee: {}%", pair.base_fee_percentage);
            println!("    Max Fee: {}%", pair.max_fee_percentage);
            println!("    Mints: {} <-> {}", pair.mint_x, pair.mint_y);
            println!(
                "    Reserves: {} <-> {}",
                pair.reserve_x_amount, pair.reserve_y_amount
            );
            println!("    Price: ${:.6}", pair.current_price);
            println!("    TVL: ${}", pair.liquidity);
            println!("    24h Volume: ${:.2}", pair.trade_volume_24h);
            println!("    24h Fees: ${:.2}", pair.fees_24h);
            println!("    APR: {:.2}%", pair.apr);
            println!("    APY: {:.2}%", pair.apy);

            if pair.farm_apr > 0.0 {
                println!("    Farm APR: {:.2}%", pair.farm_apr);
                println!("    Farm APY: {:.2}%", pair.farm_apy);
            }

            println!();
        }
    }

    Ok(())
}
