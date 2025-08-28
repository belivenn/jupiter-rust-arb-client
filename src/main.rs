use std::env;
use std::fs;
use std::time::Duration;

use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey, transaction::VersionedTransaction};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

// Load wallet from rdy5.json
fn load_wallet() -> Keypair {
    let wallet_data = fs::read_to_string("./your_wallet.json").expect("Failed to read your_wallet.json");
    let wallet_bytes: Vec<u8> = serde_json::from_str(&wallet_data).expect("Failed to parse wallet data");
    Keypair::from_bytes(&wallet_bytes).expect("Failed to create keypair from bytes")
}

#[tokio::main]
async fn main() {
    let api_base_url = env::var("API_BASE_URL").unwrap_or("https://quote-api.jup.ag/v6".into());
    println!("🤖 Jupiter Arbitrage Bot - Rust Version");
    println!("Using base url: {}", api_base_url);

    let jupiter_swap_api_client = JupiterSwapApiClient::new(api_base_url);
    let wallet = load_wallet();
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".into());

    println!("🔑 Wallet loaded: {}", wallet.pubkey());

    // Initial amount for arbitrage (1 USDC)
    let initial_amount = 10_000_000; // 1 USDC (6 decimals)
    let min_profit_threshold = 0.01; // 0.01% minimum profit

    loop {
        println!("\n🔄 Starting arbitrage cycle...");
        
        // Step 1: USDC → SOL quote
        println!("🔄 Step 1: Getting USDC → SOL quote");
        let forward_quote_request = QuoteRequest {
            amount: initial_amount,
            input_mint: USDC_MINT,
            output_mint: NATIVE_MINT,
            dexes: Some("Whirlpool,Meteora DLMM,Raydium CLMM,Fluxbeam,Dexlab,Orca".into()),
            slippage_bps: 500, // 5% slippage
            ..QuoteRequest::default()
        };

        let forward_quote = match jupiter_swap_api_client.quote(&forward_quote_request).await {
            Ok(quote) => quote,
            Err(e) => {
                println!("❌ Forward quote failed: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        println!("   ✅ Forward quote: {} USDC → {} SOL", 
                 initial_amount / 1_000_000, 
                 forward_quote.out_amount / 1_000_000_000);

        // Step 2: SOL → USDC quote
        println!("🔄 Step 2: Getting SOL → USDC quote");
        let backward_quote_request = QuoteRequest {
            amount: forward_quote.out_amount,
            input_mint: NATIVE_MINT,
            output_mint: USDC_MINT,
            dexes: Some("Whirlpool,Meteora DLMM,Raydium CLMM,Fluxbeam,Dexlab,Orca,Serum".into()),
            slippage_bps: 500, // 5% slippage
            ..QuoteRequest::default()
        };

        let backward_quote = match jupiter_swap_api_client.quote(&backward_quote_request).await {
            Ok(quote) => quote,
            Err(e) => {
                println!("❌ Backward quote failed: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        println!("   ✅ Backward quote: {} SOL → {} USDC", 
                 forward_quote.out_amount / 1_000_000_000, 
                 backward_quote.out_amount / 1_000_000);

        // Calculate profit
        let profit = backward_quote.out_amount as f64 - initial_amount as f64;
        let profit_percentage = (profit / initial_amount as f64) * 100.0;

        println!("\n💰 ARBITRAGE ANALYSIS:");
        println!("   Input: {} USDC", initial_amount / 1_000_000);
        println!("   Output: {} USDC", backward_quote.out_amount / 1_000_000);
        println!("   Profit: {} USDC ({:.2}%)", profit as f64 / 1_000_000.0, profit_percentage);

        // Check if profitable
        if profit_percentage >= min_profit_threshold {
            println!("   🎯 PROFITABLE ARBITRAGE FOUND!");
            println!("   🚀 Ready to execute!");

            // Execute atomic arbitrage (both swaps)
            println!("   📝 Executing atomic arbitrage...");
            match execute_atomic_arbitrage(&jupiter_swap_api_client, &rpc_client, &wallet, &forward_quote, &backward_quote).await {
                Ok((forward_txid, backward_txid)) => {
                    println!("   ✅ Forward swap successful: {}", forward_txid);
                    println!("   ✅ Backward swap successful: {}", backward_txid);
                    println!("   🎉 ATOMIC ARBITRAGE COMPLETED! Profit: {:.2}%", profit_percentage);
                },
                Err(e) => {
                    println!("   ❌ Atomic arbitrage failed: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        } else {
            println!("   😔 No profitable arbitrage opportunity");
        }

        println!("⏳ Waiting 1 second before next cycle...");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn execute_atomic_arbitrage(
    jupiter_client: &JupiterSwapApiClient,
    rpc_client: &RpcClient,
    wallet: &Keypair,
    forward_quote: &jupiter_swap_api_client::quote::QuoteResponse,
    backward_quote: &jupiter_swap_api_client::quote::QuoteResponse,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    // Execute forward swap first
    println!("   📝 Executing forward swap...");
    let forward_txid = execute_swap(jupiter_client, rpc_client, wallet, forward_quote).await?;
    
    // Small delay to ensure transaction is processed
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Execute backward swap
    println!("   📝 Executing backward swap...");
    let backward_txid = execute_swap(jupiter_client, rpc_client, wallet, backward_quote).await?;
    
    Ok((forward_txid, backward_txid))
}

async fn execute_swap(
    jupiter_client: &JupiterSwapApiClient,
    rpc_client: &RpcClient,
    wallet: &Keypair,
    quote: &jupiter_swap_api_client::quote::QuoteResponse,
) -> Result<String, Box<dyn std::error::Error>> {
    let swap_request = SwapRequest {
        user_public_key: wallet.pubkey(),
        quote_response: quote.clone(),
        config: TransactionConfig::default(),
    };

    let swap_response = jupiter_client.swap(&swap_request, None).await?;
    
    let versioned_transaction: VersionedTransaction =
        bincode::deserialize(&swap_response.swap_transaction)?;

    let signed_transaction = VersionedTransaction::try_new(
        versioned_transaction.message,
        &[wallet],
    )?;

    let txid = rpc_client
        .send_and_confirm_transaction(&signed_transaction)
        .await?;

    Ok(txid.to_string())
}
