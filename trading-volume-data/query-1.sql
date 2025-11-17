WITH epoch_slots AS (
  SELECT * FROM (
    VALUES
      (740, 319680000, 320111999),
      (741, 320112000, 320543999),
      (742, 320544000, 320975999),
      (743, 320976000, 321407999),
      (744, 321408000, 321839999),
      (745, 321840000, 322271999),
      (746, 322272000, 322703999),
      (747, 322704000, 323135999),
      (748, 323136000, 323567999),
      (749, 323568000, 323999999),
      (750, 324000000, 324431999),
      (751, 324432000, 324863999),
      (752, 324864000, 325295999),
      (753, 325296000, 325727999),
      (754, 325728000, 326159999),
      (755, 326160000, 326591999),
      (756, 326592000, 327023999),
      (757, 327024000, 327455999),
      (758, 327456000, 327887999),
      (759, 327888000, 328319999),
      (760, 328320000, 328751999),
      (761, 328752000, 329183999),
      (762, 329184000, 329615999),
      (763, 329616000, 330047999),
      (764, 330048000, 330479999),
      (765, 330480000, 330911999),
      (766, 330912000, 331343999),
      (767, 331344000, 331775999),
      (768, 331776000, 332207999),
      (769, 332208000, 332639999),
      (770, 332640000, 333071999),
      (771, 333072000, 333503999),
      (772, 333504000, 333935999),
      (773, 333936000, 334367999),
      (774, 334368000, 334799999),
      (775, 334800000, 335231999),
      (776, 335232000, 335663999),
      (777, 335664000, 336095999),
      (778, 336096000, 336527999),
      (779, 336528000, 336959999),
      (780, 336960000, 337391999),
      (781, 337392000, 337823999),
      (782, 337824000, 338255999),
      (783, 338256000, 338687999),
      (784, 338688000, 339119999),
      (785, 339120000, 339551999),
      (786, 339552000, 339983999),
      (787, 339984000, 340415999),
      (788, 340416000, 340847999),
      (789, 340848000, 341279999),
      (790, 341280000, 341711999),
      (791, 341712000, 342143999),
      (792, 342144000, 342575999),
      (793, 342576000, 343007999),
      (794, 343008000, 343439999),
      (795, 343440000, 343871999),
      (796, 343872000, 344303999),
      (797, 344304000, 344735999),
      (798, 344736000, 345167999),
      (799, 345168000, 345599999),
      (800, 345600000, 346031999),
      (801, 346032000, 346463999),
      (802, 346464000, 346895999),
      (803, 346896000, 347327999),
      (804, 347328000, 347759999),
      (805, 347760000, 348191999),
      (806, 348192000, 348623999),
      (807, 348624000, 349055999),
      (808, 349056000, 349487999),
      (809, 349488000, 349919999),
      (810, 349920000, 350351999),
      (811, 350352000, 350783999),
      (812, 350784000, 351215999),
      (813, 351216000, 351647999),
      (814, 351648000, 352079999),
      (815, 352080000, 352511999),
      (816, 352512000, 352943999),
      (817, 352944000, 353375999),
      (818, 353376000, 353807999),
      (819, 353808000, 354239999),
      (820, 354240000, 354671999),
      (821, 354672000, 355103999),
      (822, 355104000, 355535999),
      (823, 355536000, 355967999),
      (824, 355968000, 356399999),
      (825, 356400000, 356831999),
      (826, 356832000, 357263999),
      (827, 357264000, 357695999),
      (828, 357696000, 358127999),
      (829, 358128000, 358559999),
      (830, 358560000, 358991999),
      (831, 358992000, 359423999),
      (832, 359424000, 359855999),
      (833, 359856000, 360287999),
      (834, 360288000, 360719999),
      (835, 360720000, 361151999),
      (836, 361152000, 361583999),
      (837, 361584000, 362015999),
      (838, 362016000, 362447999),
      (839, 362448000, 362879999),
      (840, 362880000, 363311999),
      (841, 363312000, 363743999),
      (842, 363744000, 364175999),
      (843, 364176000, 364607999),
      (844, 364608000, 365039999),
      (845, 365040000, 365471999),
      (846, 365472000, 365903999),
      (847, 365904000, 366335999),
      (848, 366336000, 366767999)
  ) AS t(epoch, start_slot, end_slot)
),
swap_data AS (
  SELECT
    es.epoch,
    t.block_timestamp,
    CASE
      WHEN t.token_sold_mint = 'So11111111111111111111111111111111111111112' THEN 'SOL_TO_JITOSOL'
      ELSE 'JITOSOL_TO_SOL'
    END as swap_direction,
    CASE
      WHEN t.token_sold_mint = 'So11111111111111111111111111111111111111112'
      THEN t.token_sold_amount
      ELSE t.token_bought_amount
    END as sol_amount,
    CASE
      WHEN t.token_sold_mint = 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn'
      THEN t.token_sold_amount
      ELSE t.token_bought_amount
    END as jitosol_amount,
    t.usd_amount
  FROM dex.trades t
  INNER JOIN epoch_slots es ON t.block_slot BETWEEN es.start_slot AND es.end_slot
  WHERE ((t.token_sold_mint = 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn'
          AND t.token_bought_mint = 'So11111111111111111111111111111111111111112')
      OR (t.token_sold_mint = 'So11111111111111111111111111111111111111112'
          AND t.token_bought_mint = 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn'))
)
SELECT
  epoch,
  MIN(DATE(block_timestamp)) as epoch_start_date,
  MAX(DATE(block_timestamp)) as epoch_end_date,
  COUNT(*) as num_swaps,
  -- Total volume in SOL terms
  SUM(sol_amount) as total_volume_sol,
  -- Directional volumes
  SUM(CASE WHEN swap_direction = 'SOL_TO_JITOSOL' THEN sol_amount ELSE 0 END) as sol_to_jitosol_volume,
  SUM(CASE WHEN swap_direction = 'JITOSOL_TO_SOL' THEN sol_amount ELSE 0 END) as jitosol_to_sol_volume,
  -- Net flow (positive = more SOL leaving pool, negative = more SOL entering)
  SUM(CASE WHEN swap_direction = 'SOL_TO_JITOSOL' THEN sol_amount ELSE -sol_amount END) as net_sol_flow,
  -- USD volumes
  SUM(usd_amount) as total_volume_usd,
  -- Average swap sizes
  AVG(sol_amount) as avg_swap_size_sol,
  AVG(usd_amount) as avg_swap_size_usd,
  -- Largest swaps (for understanding whale activity)
  MAX(sol_amount) as max_swap_sol,
  PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY sol_amount) as p95_swap_sol,
  -- Price ratio metrics (JitoSOL/SOL)
  AVG(jitosol_amount / NULLIF(sol_amount, 0)) as avg_price_ratio,
  MIN(jitosol_amount / NULLIF(sol_amount, 0)) as min_price_ratio,
  MAX(jitosol_amount / NULLIF(sol_amount, 0)) as max_price_ratio,
  STDDEV(jitosol_amount / NULLIF(sol_amount, 0)) as price_ratio_stddev,
  -- Estimated LP fees (assuming 0.25% fee tier)
  SUM(sol_amount) * 0.0025 as estimated_fees_sol
FROM swap_data
GROUP BY epoch
ORDER BY epoch;
