WITH all_swaps AS (
  SELECT
    CASE
      WHEN token_sold_mint = 'So11111111111111111111111111111111111111112'
      THEN token_sold_amount
      ELSE token_bought_amount
    END as sol_amount,
    usd_amount,
    DATE(block_timestamp) as swap_date
  FROM dex.trades
  WHERE ((token_sold_mint = 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn'
          AND token_bought_mint = 'So11111111111111111111111111111111111111112')
      OR (token_sold_mint = 'So11111111111111111111111111111111111111112'
          AND token_bought_mint = 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn'))
    AND block_timestamp >= '2025-01-01'
    AND block_timestamp <= '2025-06-01'
),
date_range AS (
  SELECT
    MIN(swap_date) as min_date,
    MAX(swap_date) as max_date,
    DATEDIFF('day', MIN(swap_date), MAX(swap_date)) + 1 as total_days
  FROM all_swaps
)
SELECT
  COUNT(*) as total_swaps,
  SUM(sol_amount) as total_volume_sol,
  AVG(sol_amount) as avg_swap_sol,
  MEDIAN(sol_amount) as median_swap_sol,
  PERCENTILE_CONT(0.25) WITHIN GROUP (ORDER BY sol_amount) as p25_swap_sol,
  PERCENTILE_CONT(0.75) WITHIN GROUP (ORDER BY sol_amount) as p75_swap_sol,
  PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY sol_amount) as p95_swap_sol,
  -- Daily averages
  SUM(sol_amount) / MAX(dr.total_days) as avg_daily_volume_sol,
  COUNT(*) / MAX(dr.total_days) as avg_daily_swaps,
  -- Date range info
  MIN(swap_date) as first_swap_date,
  MAX(swap_date) as last_swap_date,
  COUNT(DISTINCT swap_date) as active_days
FROM all_swaps, date_range dr
GROUP BY dr.total_days;
