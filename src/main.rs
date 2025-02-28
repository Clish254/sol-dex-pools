use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

use dotenvy::dotenv;
use orca_whirlpools::InitializedPool as OrcaPoolInfo;
use splice_test::{
    meteora::{fetch_meteora_pools, MeteoraPoolResponse, PoolInfo as MeteoraPoolInfo},
    raydium::{fetch_raydium_pools, RaydiumPoolResponse},
    whirlpools::fetch_initialized_whirlpools,
};
use std::env;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10); // 10 second timeout for API requests

/// Structure for pool analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAnalysis {
    amm: String,
    name: String,
    pool_address: String,
    price_usd: f64,
    liquidity_usd: f64,
    fee_percentage: f64,
    volume_24h: Option<f64>,
    score: f64, // Health score
}

async fn get_pools_data(token_a_mint: &str, token_b_mint: &str) -> Result<Vec<PoolAnalysis>> {
    dotenv().ok();
    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set in .env");
    // Results collection
    let results = Arc::new(Mutex::new(Vec::new()));

    // Clone values for each task
    let token_a = token_a_mint.to_string();
    let token_b = token_b_mint.to_string();
    let results_raydium = Arc::clone(&results);
    let results_orca = Arc::clone(&results);
    let results_meteora = Arc::clone(&results);

    // Run all fetches concurrently using tokio::join
    let (raydium_result, orca_result, meteora_result) = tokio::join!(
        async {
            // Raydium task
            match timeout(
                REQUEST_TIMEOUT,
                fetch_raydium_pools(&token_a, &token_b, Some(5), Some(1)),
            )
            .await
            {
                Ok(Ok(raydium_data)) => {
                    process_raydium_pools(raydium_data, results_raydium).await;
                    Ok(())
                }
                Ok(Err(e)) => Err(format!("Raydium error: {}", e)),
                Err(_) => Err("Raydium request timed out".to_string()),
            }
        },
        async {
            // Orca task - need to handle non-Send error
            // Wrap in timeout to avoid hanging
            match timeout(
                REQUEST_TIMEOUT,
                fetch_initialized_whirlpools(&rpc_url, &token_a, &token_b, None),
            )
            .await
            {
                Ok(Ok(orca_pools)) => {
                    process_orca_pools(orca_pools, results_orca).await;
                    Ok(())
                }
                Ok(Err(e)) => Err(format!("Orca error: {}", e)),
                Err(_) => Err("Orca request timed out".to_string()),
            }
        },
        async {
            // Meteora task
            match timeout(
                REQUEST_TIMEOUT,
                fetch_meteora_pools(&token_a, &token_b, Some(1), Some(5)),
            )
            .await
            {
                Ok(Ok(meteora_data)) => {
                    process_meteora_pools(meteora_data, results_meteora).await;
                    Ok(())
                }
                Ok(Err(e)) => Err(format!("Meteora error: {}", e)),
                Err(_) => Err("Meteora request timed out".to_string()),
            }
        }
    );

    // Log any errors for debugging
    if let Err(e) = raydium_result {
        eprintln!("Warning: Raydium fetch failed: {}", e);
    }
    if let Err(e) = orca_result {
        eprintln!("Warning: Orca fetch failed: {}", e);
    }
    if let Err(e) = meteora_result {
        eprintln!("Warning: Meteora fetch failed: {}", e);
    }

    // Get the locked results
    let pool_results = results.lock().await;

    Ok(pool_results.clone())
}

async fn process_raydium_pools(
    raydium_data: RaydiumPoolResponse,
    results: Arc<Mutex<Vec<PoolAnalysis>>>,
) {
    if !raydium_data.success || raydium_data.data.pools.is_empty() {
        return;
    }

    let mut pools_lock = results.lock().await;

    for pool in raydium_data.data.pools {
        // Calculate liquidity in USD
        let liquidity_usd = pool.tvl;

        // Calculate health score - weighing factors can be adjusted as needed
        let volume_weight = 0.4; // 40% weight for volume
        let liquidity_weight = 0.5; // 50% weight for liquidity
        let fee_weight = 0.1; // 10% weight for low fees (inverted)

        // Normalize fee (lower is better) - invert percentage
        let normalized_fee = 1.0 - (pool.fee_rate / 0.01); // Assuming 1% is the max fee

        // Calculate score components
        let volume_score = if pool.day.volume > 0.0 {
            (pool.day.volume.log10() / 7.0).min(1.0) // Log scale, assuming $10M daily volume is max score
        } else {
            0.0
        };

        let liquidity_score = if liquidity_usd > 0.0 {
            (liquidity_usd.log10() / 7.0).min(1.0) // Log scale, assuming $10M liquidity is max score
        } else {
            0.0
        };

        // Calculate overall score
        let score = (volume_score * volume_weight)
            + (liquidity_score * liquidity_weight)
            + (normalized_fee * fee_weight);

        pools_lock.push(PoolAnalysis {
            amm: "Raydium".to_string(),
            name: format!("{}-{}", pool.mint_a.symbol, pool.mint_b.symbol),
            pool_address: pool.id.clone(),
            price_usd: pool.price,
            liquidity_usd,
            fee_percentage: pool.fee_rate * 100.0,
            volume_24h: Some(pool.day.volume),
            score,
        });
    }
}

async fn process_orca_pools(orca_pools: Vec<OrcaPoolInfo>, results: Arc<Mutex<Vec<PoolAnalysis>>>) {
    if orca_pools.is_empty() {
        return;
    }

    let mut pools_lock = results.lock().await;

    for pool in orca_pools {
        // Use the current price from the pool
        let price = pool.price;

        // Estimate liquidity in USD - this is a rough estimation
        // Convert raw liquidity to approximate USD value
        // Orca's liquidity is in "virtual" units, need to convert to USD
        let liquidity_factor = 1.0e-9; // Conversion factor, may need adjustment
        let liquidity_usd = pool.data.liquidity as f64 * liquidity_factor * price;

        // Calculate health score
        let liquidity_weight = 0.7; // 70% weight for liquidity
        let fee_weight = 0.3; // 30% weight for low fees (inverted)

        // Normalize fee (lower is better) - invert percentage
        let fee_rate = pool.data.fee_rate as f64 / 10000.0;
        let normalized_fee = 1.0 - (fee_rate / 0.01); // Assuming 1% is the max fee

        // Calculate score components
        let liquidity_score = if liquidity_usd > 0.0 {
            (liquidity_usd.log10() / 7.0).min(1.0) // Log scale, assuming $10M liquidity is max score
        } else {
            0.0
        };

        // Calculate overall score - no volume data available
        let score = (liquidity_score * liquidity_weight) + (normalized_fee * fee_weight);

        pools_lock.push(PoolAnalysis {
            amm: "Orca".to_string(),
            name: format!("Whirlpool-{}", pool.data.tick_spacing),
            pool_address: pool.address.to_string(),
            price_usd: price,
            liquidity_usd,
            fee_percentage: fee_rate * 100.0,
            volume_24h: None, // Orca API doesn't provide volume data directly
            score,
        });
    }
}

async fn process_meteora_pools(
    meteora_data: MeteoraPoolResponse,
    results: Arc<Mutex<Vec<PoolAnalysis>>>,
) {
    if meteora_data.data.is_empty() {
        return;
    }

    let mut pools_lock = results.lock().await;

    for pool in meteora_data.data {
        // Extract price - assuming SOL/USDC pool structure
        let price = match calc_meteora_price(&pool) {
            Some(p) => p,
            None => continue, // Skip this pool if price calculation fails
        };

        // Get liquidity in USD
        let liquidity_usd = match pool.pool_tvl.parse::<f64>() {
            Ok(tvl) => tvl,
            Err(_) => continue, // Skip this pool if TVL parsing fails
        };

        // Parse fee percentage
        let fee_percentage = pool.total_fee_pct.parse::<f64>().unwrap_or(0.0);

        // Calculate health score
        let volume_weight = 0.4; // 40% weight for volume
        let liquidity_weight = 0.5; // 50% weight for liquidity
        let fee_weight = 0.1; // 10% weight for low fees (inverted)

        // Normalize fee (lower is better) - invert percentage
        let normalized_fee = 1.0 - (fee_percentage / 1.0); // Assuming 1% is the max fee

        // Calculate score components
        let volume_score = if pool.trading_volume > 0.0 {
            (pool.trading_volume.log10() / 7.0).min(1.0) // Log scale
        } else {
            0.0
        };

        let liquidity_score = if liquidity_usd > 0.0 {
            (liquidity_usd.log10() / 7.0).min(1.0) // Log scale, assuming $10M liquidity is max score
        } else {
            0.0
        };

        // Calculate overall score
        let score = (volume_score * volume_weight)
            + (liquidity_score * liquidity_weight)
            + (normalized_fee * fee_weight);

        pools_lock.push(PoolAnalysis {
            amm: "Meteora".to_string(),
            name: pool.pool_name.clone(),
            pool_address: pool.pool_address.clone(),
            price_usd: price,
            liquidity_usd,
            fee_percentage,
            volume_24h: Some(pool.trading_volume),
            score,
        });
    }
}

fn calc_meteora_price(pool: &MeteoraPoolInfo) -> Option<f64> {
    // Find the indices for tokens in the pool
    let (token0_amount, token1_amount) = match (
        pool.pool_token_amounts[0].parse::<f64>(),
        pool.pool_token_amounts[1].parse::<f64>(),
    ) {
        (Ok(amt0), Ok(amt1)) => (amt0, amt1),
        _ => return None,
    };

    // Check if this is a SOL pool and calculate price accordingly
    if pool.pool_token_mints[0] == "So11111111111111111111111111111111111111112" {
        // SOL is token0, calculate price as token1/token0
        if token0_amount > 0.0 {
            Some(token1_amount / token0_amount)
        } else {
            None
        }
    } else if pool.pool_token_mints[1] == "So11111111111111111111111111111111111111112" {
        // SOL is token1, calculate price as token0/token1
        if token1_amount > 0.0 {
            Some(token0_amount / token1_amount)
        } else {
            None
        }
    } else {
        // Not a SOL pool, return the ratio anyway
        if token0_amount > 0.0 {
            Some(token1_amount / token0_amount)
        } else {
            None
        }
    }
}

/// Find the healthiest pool across all AMMs based on the calculated score
fn find_healthiest_pool(pools: &[PoolAnalysis]) -> Option<PoolAnalysis> {
    pools
        .iter()
        .max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

/// Entry point for the token price and liquidity analysis
pub async fn token_price_analysis(token_a_mint: &str, token_b_mint: &str) -> Result<PoolAnalysis> {
    // Get all pools data in parallel
    let all_pools = get_pools_data(token_a_mint, token_b_mint).await?;

    if all_pools.is_empty() {
        return Err(anyhow::anyhow!(
            "No valid pools found for the given token pair"
        ));
    }

    // Find the healthiest pool
    match find_healthiest_pool(&all_pools) {
        Some(best_pool) => Ok(best_pool),
        None => Err(anyhow::anyhow!(
            "No valid pools found for the given token pair"
        )),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Fetching data for SOL/USDC pools...");

    let sol_mint = "So11111111111111111111111111111111111111112"; // wSOL
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC

    // Execute the analysis
    match token_price_analysis(sol_mint, usdc_mint).await {
        Ok(best_pool) => {
            println!("\n📊 ANALYSIS RESULTS 📊");
            println!("Best pool found on: {}", best_pool.amm);
            println!("Pool name: {}", best_pool.name);
            println!("Pool address: {}", best_pool.pool_address);
            println!("Price: ${:.6}", best_pool.price_usd);
            println!("Liquidity: ${:.2}", best_pool.liquidity_usd);
            println!("Fee rate: {:.4}%", best_pool.fee_percentage);
            if let Some(volume) = best_pool.volume_24h {
                println!("24h Volume: ${:.2}", volume);
            }
            println!("Health score: {:.4} (out of 1.0)", best_pool.score);
        }
        Err(e) => println!("Error analyzing pools: {}", e),
    }

    Ok(())
}
