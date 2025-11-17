# JitoSOL Reserve Implementation Plan

## Executive Summary

Based on the THESIS.md, we're implementing a **Reserve** strategy, NOT an AMM LP strategy. The Reserve acts as "instant exit insurance" for JitoSOL→SOL conversions, earning premium fees (10-300+ bps) during stress events when AMM liquidity is insufficient or too expensive.

**Key Insight**: AMM LPs earn pennies (0.5-1% APY) because they're over-liquid in normal times. The Reserve earns 10-15% APY by only activating during stress and charging utilization-based fees.

## Strategy Overview

### What We're Building
- **NOT**: Passive AMM LP earning constant small fees
- **BUT**: Active Reserve providing backstop liquidity during market stress
- **Revenue Model**: Event-driven, spiky returns from "impatience premium" and exit fees

### Economic Model
```
Normal Times (90% of epochs):
- AMMs handle all flow efficiently
- Reserve sits idle, earns 0%
- Preserves capital for stress events

Stress Times (10% of epochs):
- AMM liquidity exhausted or too expensive
- Reserve activates with premium pricing
- Earns 100-300 bps on large volumes
- Captures "impatience premium" from panic exits
```

## Implementation Architecture

### 1. Core Data Structures

```rust
pub struct JitoSOLReserve {
    // Reserve state
    pub sol_balance: u64,                    // Available SOL in reserve
    pub max_capacity: u64,                   // Total SOL allocated (5-10% of pool)
    pub current_utilization: f64,            // How much has been used

    // Fee curve parameters
    pub base_fee_bps: u64,                   // Low utilization fee (10 bps)
    pub max_fee_bps: u64,                    // High utilization fee (300+ bps)

    // Market conditions
    pub amm_depth_sol: u64,                  // Current AMM liquidity depth (~14M SOL)
    pub amm_price_impact: f64,               // Price impact on AMMs for size
    pub recent_flow_imbalance: f64,          // Net directional pressure
}

pub struct ReserveActivationEvent {
    pub epoch: u64,
    pub trigger_type: StressTrigger,
    pub overflow_volume: u64,                // Volume that AMMs couldn't handle
    pub avg_fee_bps: u64,                    // Average fee charged
    pub revenue_earned: u64,                 // Total fees collected
    pub peak_utilization: f64,               // Maximum reserve utilization
}

pub enum StressTrigger {
    DirectionalFlow,      // Large net JitoSOL→SOL pressure
    WhaleActivity,        // Individual swaps > 1% of AMM TVL
    VolumeSpike,         // Total volume > 2x AMM TVL
    PriceDeviation,      // JitoSOL trading below intrinsic value
}
```

### 2. Utilization-Based Fee Curve

```rust
impl JitoSOLReserve {
    /// Sanctum-style utilization pricing
    pub fn calculate_fee_bps(&self) -> u64 {
        let u = self.current_utilization;

        match u {
            u if u < 0.2 => 10,      // 0-20%: 10 bps (competitive)
            u if u < 0.5 => 25,      // 20-50%: 25 bps
            u if u < 0.7 => 50,      // 50-70%: 50 bps
            u if u < 0.85 => 100,    // 70-85%: 100 bps
            u if u < 0.95 => 200,    // 85-95%: 200 bps
            _ => 300,                // 95-100%: 300+ bps (emergency)
        }
    }
}
```

### 3. Reserve Activation Logic

```rust
pub fn should_activate_reserve(
    epoch_data: &EpochTradingData,
    amm_tvl: u64,
) -> (bool, StressTrigger) {
    const AMM_TVL: u64 = 14_000_000 * LAMPORTS_PER_SOL;  // ~$14M
    const INFINITY_MULTIPLIER: f64 = 1.5;  // Infinity adds 50% depth

    let effective_liquidity = (AMM_TVL as f64 * INFINITY_MULTIPLIER) as u64;

    // Trigger 1: Directional flow overwhelming AMMs
    if epoch_data.net_sol_flow.abs() > effective_liquidity as f64 * 0.1 {
        return (true, StressTrigger::DirectionalFlow);
    }

    // Trigger 2: Whale trades that would move AMM price significantly
    if epoch_data.p95_swap_sol > effective_liquidity as f64 * 0.01 {
        return (true, StressTrigger::WhaleActivity);
    }

    // Trigger 3: High volume causing repeated AMM imbalances
    if epoch_data.total_volume_sol > effective_liquidity as f64 * 2.0 {
        return (true, StressTrigger::VolumeSpike);
    }

    // Trigger 4: Price deviation (JitoSOL < 0.98 SOL)
    if epoch_data.min_price_ratio < 0.98 {
        return (true, StressTrigger::PriceDeviation);
    }

    (false, StressTrigger::DirectionalFlow)  // No activation
}
```

### 4. Revenue Simulation

```rust
pub fn simulate_reserve_revenue(
    reserve_size: u64,
    epoch_data: &EpochTradingData,
) -> ReserveReturns {
    let (should_activate, trigger) = should_activate_reserve(epoch_data, AMM_TVL);

    if !should_activate {
        return ReserveReturns {
            revenue: 0,
            volume_processed: 0,
            utilization: 0.0,
            activation_trigger: None,
        };
    }

    // Calculate overflow volume that routes to Reserve
    let overflow_volume = match trigger {
        StressTrigger::DirectionalFlow => {
            // Net flow beyond AMM capacity
            (epoch_data.net_sol_flow.abs() - AMM_CAPACITY * 0.1).max(0.0)
        },
        StressTrigger::WhaleActivity => {
            // Large trades that would impact AMM price > 1%
            epoch_data.total_volume_sol * 0.2  // Assume 20% routes to Reserve
        },
        StressTrigger::VolumeSpike => {
            // Excess volume beyond 2x AMM TVL
            (epoch_data.total_volume_sol - AMM_CAPACITY * 2.0).max(0.0)
        },
        StressTrigger::PriceDeviation => {
            // Arbitrage flow to restore peg
            epoch_data.jitosol_to_sol_volume * 0.5  // 50% of exit flow
        },
    };

    // Process volume with utilization-based fees
    let mut total_fees = 0u64;
    let mut volume_processed = 0u64;
    let mut current_utilization = 0.0;

    let chunks = overflow_volume / 10.0;  // Process in 10% increments

    for i in 0..10 {
        if volume_processed >= overflow_volume {
            break;
        }

        let chunk = chunks.min(overflow_volume - volume_processed);
        current_utilization = (i as f64 + 1.0) / 10.0;

        let fee_bps = calculate_fee_for_utilization(current_utilization);
        total_fees += (chunk * fee_bps as f64 / 10000.0) as u64;
        volume_processed += chunk as u64;
    }

    // Add impatience premium (users accepting discount for instant exit)
    let impatience_premium = volume_processed * 50 / 10000;  // 50 bps

    ReserveReturns {
        revenue: total_fees + impatience_premium,
        volume_processed,
        utilization: current_utilization,
        activation_trigger: Some(trigger),
    }
}
```

## Integration with Stakenet Simulator

### 1. Load Trading Volume Data

```rust
// Load epoch-aligned trading data from CSV
pub async fn load_trading_data() -> HashMap<u64, EpochTradingData> {
    let csv_path = "trading-volume-data/query-1.csv";
    let mut reader = csv::Reader::from_path(csv_path)?;

    let mut epoch_data = HashMap::new();
    for result in reader.deserialize() {
        let record: TradingRecord = result?;
        epoch_data.insert(record.epoch, EpochTradingData::from(record));
    }

    epoch_data
}
```

### 2. Modify Rebalancing Simulator

```rust
impl RebalancingSimulator {
    pub async fn run_simulation_with_reserve(
        &mut self,
        db_connection: &Pool<Postgres>,
        trading_data: HashMap<u64, EpochTradingData>,
    ) -> Result<SimulationResults, CliError> {

        for epoch in self.simulation_start_epoch..self.simulation_end_epoch {
            // Existing epoch processing
            self.process_epoch_cycle(db_connection, epoch).await?;

            // Reserve strategy (5% allocation)
            let reserve_allocation = self.total_lamports_staked * 5 / 100;
            let staking_allocation = self.total_lamports_staked * 95 / 100;

            // Get trading data for this epoch
            if let Some(epoch_trading) = trading_data.get(&(epoch as u64)) {
                // Simulate reserve returns
                let reserve_returns = simulate_reserve_revenue(
                    reserve_allocation,
                    epoch_trading,
                );

                // Apply returns
                if reserve_returns.revenue > 0 {
                    self.current_cycle_rewards += reserve_returns.revenue;

                    info!(
                        "Epoch {}: Reserve activated ({}), earned {} SOL on {} volume at {:.1}% utilization",
                        epoch,
                        reserve_returns.activation_trigger,
                        reserve_returns.revenue / LAMPORTS_PER_SOL,
                        reserve_returns.volume_processed / LAMPORTS_PER_SOL,
                        reserve_returns.utilization * 100.0
                    );
                }
            }

            // Continue with normal cycle completion
            if self.is_cycle_complete(epoch) {
                self.complete_cycle(cycle_starting_lamports);
            }
        }

        Ok(self.calculate_final_metrics())
    }
}
```

### 3. Metrics & Reporting

```rust
pub struct ReserveMetrics {
    // Activation statistics
    pub total_epochs: u64,
    pub activation_count: u64,
    pub activation_rate: f64,

    // Revenue breakdown
    pub total_fees_earned: u64,
    pub total_impatience_premium: u64,
    pub total_volume_processed: u64,

    // Performance metrics
    pub avg_utilization_when_active: f64,
    pub avg_fee_bps_charged: f64,
    pub reserve_apy: f64,
    pub blended_apy: f64,  // Combined with 95% staking

    // Trigger analysis
    pub triggers: HashMap<StressTrigger, u64>,
}
```

## Expected Outcomes

### Base Case (Historical Data)
```
Activation Rate: ~10% of epochs
Average Fee When Active: 150 bps
Volume Multiplier: 5x reserve size
Reserve APY: 13.7%
Staking APY: 7.2%
Blended APY: 8.5%
```

### Stress Scenario
```
Activation Rate: 25% of epochs
Average Fee When Active: 200 bps
Volume Multiplier: 10x reserve size
Reserve APY: 50%
Staking APY: 7.2%
Blended APY: 9.3%
```

### Idle Scenario
```
Activation Rate: 5% of epochs
Average Fee When Active: 100 bps
Volume Multiplier: 3x reserve size
Reserve APY: 2.7%
Staking APY: 7.2%
Blended APY: 6.98%
```

## Implementation Timeline

### Phase 1: Data Integration (Week 1)
- [ ] Load trading volume CSVs into simulator
- [ ] Map epochs to trading data
- [ ] Implement activation trigger detection

### Phase 2: Reserve Logic (Week 2)
- [ ] Implement utilization-based fee curve
- [ ] Add overflow volume calculation
- [ ] Create revenue simulation functions

### Phase 3: Simulator Integration (Week 3)
- [ ] Modify RebalancingSimulator for reserve strategy
- [ ] Update APY calculations
- [ ] Add reserve-specific logging

### Phase 4: Testing & Calibration (Week 4)
- [ ] Backtest against historical data
- [ ] Tune activation thresholds
- [ ] Validate against THESIS assumptions

## Risk Considerations

### 1. Activation Risk
- **Risk**: Reserve might not activate as often as modeled
- **Mitigation**: Conservative 10% activation assumption

### 2. Competition Risk
- **Risk**: Other reserves (Sanctum, Infinity) compete for flow
- **Mitigation**: Focus on JitoSOL-specific optimizations

### 3. Liquidity Risk
- **Risk**: Large exits might exceed reserve capacity
- **Mitigation**: Utilization-based pricing protects profitability

## Success Metrics

1. **Reserve Activation Rate**: Target 10-15% of epochs
2. **Average Fee Captured**: Target 100-200 bps when active
3. **Blended APY**: Target 8-10% (vs 7.2% pure staking)
4. **Utilization Efficiency**: Average 50-70% when active

## Conclusion

This Reserve strategy transforms the 5% allocation from a low-yield AMM LP position into a high-value "exit insurance" product that:
- Earns nothing in normal times (preserving capital)
- Captures significant fees during stress (10-300 bps)
- Provides critical liquidity when most needed
- Beats pure staking APY through strategic positioning

The key insight: **We're not competing for every trade; we're monetizing the trades that AMMs can't handle efficiently.**