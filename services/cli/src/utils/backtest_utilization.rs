use crate::{commands::DAYS_PER_YEAR, error::CliError, utils::RebalancingCycle};
use num_traits::cast::ToPrimitive;
use sqlx::{Pool, Postgres, types::BigDecimal};
use stakenet_simulator_db::{
    active_stake_jito_sol::ActiveStakeJitoSol, inactive_stake_jito_sol::InactiveStakeJitoSol,
};

pub fn calculate_apy(r: f64, t: f64, n: f64) -> f64 {
    // APY = (1 + r)^(n/t) - 1
    (1.0 + r).powf(n / t) - 1.0
}

pub fn calculate_aggregated_apy(
    rebalancing_cycles: &[RebalancingCycle],
    total_lookback_period: u16,
) -> Result<f64, CliError> {
    if rebalancing_cycles.is_empty() {
        return Ok(0.0);
    }

    // Get the initial total stake amount
    let initial_total_stake = rebalancing_cycles[0].starting_total_lamports;

    if initial_total_stake == 0 {
        return Ok(0.0);
    }

    // Calculate total rewards earned across all cycles (excluding deposits/withdrawals)
    let total_rewards_earned: u64 = rebalancing_cycles
        .iter()
        .map(|cycle| cycle.total_rewards_earned)
        .sum();

    // Calculate total net deposits/withdrawals for verification
    let total_deposits: i64 = rebalancing_cycles
        .iter()
        .map(|cycle| cycle.total_deposit_withdrawals)
        .sum();

    // Get final total stake for comparison
    let final_total_stake = rebalancing_cycles
        .last()
        .ok_or(CliError::ArithmeticError)?
        .ending_total_lamports;

    // Log validation information
    tracing::info!(
        "APY calculation breakdown - Initial stake: {:.6} SOL, Final stake: {:.6} SOL",
        initial_total_stake as f64 / 1_000_000_000.0,
        final_total_stake as f64 / 1_000_000_000.0
    );
    tracing::info!(
        "Total rewards earned: {:.6} SOL, Net deposits/withdrawals: {:.6} SOL",
        total_rewards_earned as f64 / 1_000_000_000.0,
        total_deposits as f64 / 1_000_000_000.0
    );

    // Verify that rewards + deposits = total change
    let expected_change = (total_rewards_earned as i64) + total_deposits;
    let actual_change = final_total_stake as i64 - initial_total_stake as i64;
    if expected_change != actual_change {
        tracing::warn!(
            "Validation mismatch! Expected change: {:.6} SOL, Actual change: {:.6} SOL",
            expected_change as f64 / 1_000_000_000.0,
            actual_change as f64 / 1_000_000_000.0
        );
    }

    // Calculate return rate using only rewards (not deposits/withdrawals)
    let overall_return_rate = total_rewards_earned
        .to_f64()
        .ok_or(CliError::ArithmeticError)?
        / initial_total_stake
            .to_f64()
            .ok_or(CliError::ArithmeticError)?;

    // Log the old (incorrect) calculation for comparison
    let old_incorrect_return_rate =
        (final_total_stake as f64 - initial_total_stake as f64) / initial_total_stake as f64;
    tracing::info!(
        "Return rate - Old (incorrect with deposits): {:.4}%, New (correct rewards only): {:.4}%",
        old_incorrect_return_rate * 100.0,
        overall_return_rate * 100.0
    );

    // Convert to APY
    let lookback_period_in_days = total_lookback_period
        .to_f64()
        .ok_or(CliError::ArithmeticError)?
        * 2.0; // Assuming 2 days per epoch

    if lookback_period_in_days >= DAYS_PER_YEAR {
        return Err(CliError::LookBackPeriodTooBig);
    }

    let apy = calculate_apy(overall_return_rate, lookback_period_in_days, DAYS_PER_YEAR);

    Ok(apy)
}

fn calculate_stake_utilization(
    total_active_balance: &BigDecimal,
    total_inactive_balance: &BigDecimal,
) -> Result<f64, CliError> {
    let total_stake = total_active_balance.clone() + total_inactive_balance.clone();

    if total_stake == BigDecimal::from(0) {
        return Ok(0.0);
    }

    let utilization_rate = total_active_balance
        .to_f64()
        .ok_or(CliError::ArithmeticError)?
        / total_stake.to_f64().ok_or(CliError::ArithmeticError)?;

    Ok(utilization_rate)
}

pub async fn calculate_stake_utilization_rate(
    db_connection: &Pool<Postgres>,
    lookback_period: u16,
    current_epoch: u16,
) -> Result<f64, CliError> {
    if lookback_period > current_epoch {
        return Err(CliError::LookBackPeriodTooBig);
    }

    let (active_stake_data, inactive_stake_data) = futures::join!(
        ActiveStakeJitoSol::fetch_balance_for_epoch_range(
            db_connection,
            current_epoch as u64,
            lookback_period as u64,
        ),
        InactiveStakeJitoSol::fetch_balance_for_epoch_range(
            db_connection,
            current_epoch as u64,
            lookback_period as u64,
        )
    );

    let active_stake_data = active_stake_data?;
    let inactive_stake_data = inactive_stake_data?;

    if active_stake_data.count != inactive_stake_data.count {
        return Err(CliError::RecordCountMismatch {
            active_count: active_stake_data.count,
            inactive_count: inactive_stake_data.count,
        });
    }

    calculate_stake_utilization(&active_stake_data.balance, &inactive_stake_data.balance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::types::BigDecimal;

    #[test]
    fn test_apy_calculation() {
        let r = 0.02; // 2% return
        let t = 2.0; // 2-day period
        let n = 365.0; // Days in a year
        let apy = calculate_apy(r, t, n);
        assert!((apy - 36.113).abs() < 0.001, "APY calculation is incorrect");
    }

    #[test]
    fn test_calculate_stake_utilization_rate_from_balances() {
        // INACTIVE BALANCE is 0
        let active_balance = BigDecimal::from(100);
        let inactive_balance = BigDecimal::from(0);
        let result = calculate_stake_utilization(&active_balance, &inactive_balance);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1.0);

        // ACTIVE BALANCE is 0
        let active_balance = BigDecimal::from(0);
        let inactive_balance = BigDecimal::from(100);
        let result = calculate_stake_utilization(&active_balance, &inactive_balance);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.0);

        // TOTAL BALANCE is 0
        let active_balance = BigDecimal::from(0);
        let inactive_balance = BigDecimal::from(0);
        let result = calculate_stake_utilization(&active_balance, &inactive_balance);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.0);

        let active_balance = BigDecimal::from(800);
        let inactive_balance = BigDecimal::from(200);
        let result = calculate_stake_utilization(&active_balance, &inactive_balance);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.8);
    }
}
