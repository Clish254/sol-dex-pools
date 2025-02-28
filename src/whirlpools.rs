use orca_whirlpools::{
    fetch_whirlpools_by_token_pair, set_whirlpools_config_address, InitializedPool, PoolInfo,
    WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::error::Error;
use std::str::FromStr;

use std::env;

/// Fetches initialized whirlpools for a token pair
///
/// # Arguments
///
/// * `rpc_url` - The Solana RPC URL to connect to
/// * `token_a_mint` - Address of the first token mint as a string
/// * `token_b_mint` - Address of the second token mint as a string
/// * `network` - Network to use (mainnet, devnet, etc.) - defaults to mainnet if None
///
/// # Returns
///
/// Returns a Result containing a vector of InitializedPool objects or an error
pub async fn fetch_initialized_whirlpools(
    rpc_url: &str,
    token_a_mint: &str,
    token_b_mint: &str,
    network: Option<WhirlpoolsConfigInput>,
) -> Result<Vec<InitializedPool>, Box<dyn Error>> {
    // Parse token addresses
    let token_a = Pubkey::from_str(token_a_mint).map_err(|e| {
        format!(
            "Failed to parse token A mint address {}: {}",
            token_a_mint, e
        )
    })?;

    let token_b = Pubkey::from_str(token_b_mint).map_err(|e| {
        format!(
            "Failed to parse token B mint address {}: {}",
            token_b_mint, e
        )
    })?;

    // Set the whirlpools config address based on the network
    let network_config = network.unwrap_or(WhirlpoolsConfigInput::SolanaMainnet);
    set_whirlpools_config_address(network_config)
        .map_err(|e| format!("Failed to set whirlpools config address: {}", e))?;

    // Create RPC client
    let rpc = RpcClient::new(rpc_url.to_string());

    // Fetch all whirlpools for the token pair
    let pool_infos = fetch_whirlpools_by_token_pair(&rpc, token_a, token_b)
        .await
        .map_err(|e| format!("Failed to fetch whirlpools by token pair: {}", e))?;

    // Filter for only initialized pools
    let initialized_pools: Vec<InitializedPool> = pool_infos
        .into_iter()
        .filter_map(|pool_info| {
            if let PoolInfo::Initialized(pool) = pool_info {
                Some(pool)
            } else {
                None
            }
        })
        .collect();

    Ok(initialized_pools)
}

/// Example usage of the whirlpool finder
pub async fn orca_example_usage() -> Result<(), Box<dyn Error>> {
    println!("here");
    // Define inputs

    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set in .env");
    let sol_mint = "So11111111111111111111111111111111111111112"; // wSOL
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC

    // Fetch initialized whirlpools
    let initialized_pools = fetch_initialized_whirlpools(
        &rpc_url, sol_mint, usdc_mint, None, // Use mainnet
    )
    .await?;

    println!(
        "Found {} initialized SOL-USDC whirlpools",
        initialized_pools.len()
    );

    for (i, pool) in initialized_pools.iter().enumerate() {
        println!("Pool {}: {}", i + 1, pool.address);
        println!("  Tick Spacing: {}", pool.data.tick_spacing);
        println!("  Fee Rate: {}%", pool.data.fee_rate as f64 / 10000.0);
        println!("  Liquidity: {}", pool.data.liquidity);
        println!("  Current Tick: {}", pool.data.tick_current_index);
        println!("  Current Price: {}", pool.price);
    }

    Ok(())
}
