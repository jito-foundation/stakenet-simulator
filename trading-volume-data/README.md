# JitoSOL/SOL Trading Volume Data Analysis

This directory contains trading volume data and analysis queries for modeling a **5% SOL Reserve APY strategy** within the stakenet-simulator. The data captures all JitoSOL/SOL swaps that went through AMMs (Automated Market Makers) on Solana.

## Overview

The queries analyze swap data between:
- **SOL** (Native Solana token): `So11111111111111111111111111111111111111112`
- **JitoSOL** (Liquid staking token): `J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn`

All queries are **epoch-aligned** using Solana's epoch schedule (epochs 740-848) to match the stakenet-simulator's rebalancing cycles. Each epoch is approximately 2 days.

## Data Files

### Query 1: Epoch-Level Trading Volumes and Flows
**Files:** `query-1.sql`, `query-1.csv`

**Purpose:** Core metrics for fee revenue estimation and flow analysis

**Key Metrics:**
- `TOTAL_VOLUME_SOL`: Total swap volume per epoch (used for LP fee calculations)
- `NET_SOL_FLOW`: Net directional flow (positive = SOL leaving pool)
- `SOL_TO_JITOSOL_VOLUME` / `JITOSOL_TO_SOL_VOLUME`: Directional volumes
- `ESTIMATED_FEES_SOL`: Estimated LP fees at 0.25% (25 bps)
- `AVG_PRICE_RATIO`: Average JitoSOL/SOL price ratio
- `P95_SWAP_SOL`: 95th percentile swap size (whale activity indicator)

**Example Insights:**
- Epoch 740: 279,361 SOL volume → ~698 SOL in fees
- Average swap size: 5.02 SOL
- Price ratio: ~0.849 (JitoSOL trades at ~85% of SOL price)

### Query 3: Epoch Volatility for Impermanent Loss
**Files:** `query-3.sql`, `query-3.csv`

**Purpose:** Calculate price volatility and impermanent loss (IL) per epoch

**Key Metrics:**
- `EPOCH_VOLATILITY`: Standard deviation of hourly prices within epoch
- `EPOCH_RANGE_PCT`: Price range as percentage of average
- `EPOCH_MIN_PRICE` / `EPOCH_MAX_PRICE`: Price boundaries for IL calculation
- `ESTIMATED_IL_PCT`: Estimated impermanent loss percentage

**Example Insights:**
- Epoch 740: 0.96% price range, -0.12% IL
- Low volatility = minimal impermanent loss for SOL/JitoSOL pair
- Price ratio stable around 1.177 (JitoSOL appreciates vs SOL due to staking rewards)

### Query 4: Pool Competition Analysis
**Files:** `query-4.sql`, `query-4.csv`

**Purpose:** Analyze market share across different protocols/pools

**Key Metrics:**
- `PROTOCOL`: AMM protocol (Orca, Raydium, Meteora, etc.)
- `VOLUME_SOL`: Volume per protocol per epoch
- `MARKET_SHARE_PCT`: Percentage of total epoch volume
- `UNIQUE_POOLS_USED`: Number of different pools

**Example Insights (Epoch 740):**
- Orca Whirlpool V2: 66.16% market share
- Meteora DLMM: 16.90% market share
- Market is concentrated in top 3-4 protocols
- Your reserve would compete for fees across these pools

### Query 5: Overall Summary Statistics
**Files:** `query-5.sql`, `query-5.csv`

**Purpose:** High-level statistics for model calibration

**Key Metrics:**
- Total period: Jan 1 - May 31, 2025 (151 active days)
- Total volume: 33.15M SOL
- Average daily volume: 219,569 SOL
- Median swap: 0.3 SOL (retail size)
- P95 swap: 6.53 SOL (whale threshold)

## SOL Reserve Strategy Modeling

### Strategy Parameters
- **Reserve Size:** 5% of total SOL deposits
- **Fee Tier:** 0.25% (25 basis points) typical for stable pairs
- **Rebalancing:** Every epoch (2 days)

### APY Calculation Framework

```
Per Epoch:
LP_fees = epoch_volume × fee_rate × (reserve_size / total_pool_liquidity)
IL_loss = calculate_IL(min_price, max_price)
Net_return = LP_fees - IL_loss - rebalancing_costs

Annual APY = compound_returns_over_epochs
```

### Key Findings

1. **Volume Patterns:**
   - Average epoch volume: ~400,000 SOL
   - High variation between epochs (183K - 827K SOL in sample)
   - Consistent activity (all epochs have trades)

2. **Fee Revenue Potential:**
   - At 0.25% fee rate: ~1,000 SOL fees per epoch on average volume
   - Your share depends on pool depth (reserve_size / total_liquidity)
   - Need ~20x leverage (volume/reserve) to match staking APY

3. **Impermanent Loss:**
   - Minimal due to high correlation between SOL and JitoSOL
   - Typically < 0.2% per epoch
   - JitoSOL steadily appreciates vs SOL due to staking rewards

4. **Competition:**
   - Dominated by Orca (60-70% market share)
   - Would need to provide liquidity across multiple pools for maximum fees

## Integration with Stakenet Simulator

The epoch-aligned data can be directly integrated into the simulator:

```rust
// Per epoch in simulator
let epoch_volume = trading_data[epoch].total_volume_sol;
let reserve_sol = total_stake * 0.05;
let pool_depth = estimate_pool_depth(epoch);
let fee_share = reserve_sol / pool_depth;
let lp_fees = epoch_volume * 0.0025 * fee_share;
let il_loss = trading_data[epoch].estimated_il_pct * reserve_sol;
let net_return = lp_fees - il_loss;
```

## Data Collection Period
- **Start:** January 1, 2025
- **End:** May 31, 2025
- **Epochs:** 740 - 848 (matching simulator range)
- **Total Swaps:** 5.48M transactions

## SQL Query Structure

All queries use epoch-slot mapping to align with Solana's epoch schedule:
- Epoch 740: slots 319,680,000 - 320,111,999
- Epoch 848: slots 366,336,000 - 366,767,999

Trades are mapped to epochs using the `BLOCK_SLOT` column, ensuring perfect alignment with the stakenet-simulator's rebalancing cycles.