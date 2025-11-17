Yeah, what you’re seeing on Orca/Meteora is exactly the point: resting AMM liquidity for JitoSOL/SOL is not where the money is.

Those pools are basically:

Orca JitoSOL/SOL: TVL ≈ $6.8m, 24h vol ≈ $24m, 24h fees ≈ $679 (0.01% base fee). That’s a few bps per day on TVL on a good day – low single-digit % APR at best. 
orca.so

Meteora JitoSOL/SOL: TVL ≈ $7.1m, 24h vol ≈ $0.61m. That’s also going to translate to very low fee APR — typical LST stables on these venues are around ~0.7–1.1% APY from fees. 
Exponential DeFi
+1

So your instinct is right: just LP’ing 5–10% of JitoSOL’s SOL into CL pools will never beat staking APY in expectation.

1. What your Reserve is actually selling

Your one-sided SOL Reserve is not an AMM LP product. Economically, you’re selling:

“Instant, size-insensitive JitoSOL → SOL exit, at a premium, when the DEX / Infinity / existing liquidity is not enough or too expensive.”

Right now:

Normal times:

Orca + Meteora + Infinity + random JitoSOL/SOL liquidity already give tight pricing near 1.0 with tiny fees (1–10 bps).

That’s why LP fees are tiny: the system is over-liquid relative to day-to-day flow.

Stress / directionally one-sided times (everyone wants out of JitoSOL into SOL now):

CL pools and Infinity get pushed off-peg very quickly.

Price via AMMs could be 0.99, 0.98, 0.95 JitoSOL/SOL or worse for size.

Sanctum Reserve exists exactly for these conditions: it steps in with deeper SOL at a utilization-based fee (8–800 bps) when Router + Infinity can’t give a decent path. 
Sanctum
+1

Your Reserve monetizes that gap:

You don’t try to compete for every tiny retail swap at 1–3 bps.

You sit there with real size (5–10% of the JitoSOL pool), and say:

“If the best route via Orca/Meteora/Infinity is 2–3% below intrinsic, we’ll give you something like 0.5–1.5% below intrinsic, instantly, for size.”

That “exit convenience premium” is your revenue, not the constant trickle of 0.01% swap fees.

2. Why AMM LPs earn pennies, but a Reserve can earn chunky fees
AMM side (Orca/Meteora)

Fee is fixed per trade (e.g. 0.01–0.25%).

Price impact grows with size; the bigger the trade, the worse the execution.

As long as both sides of the JitoSOL/SOL market are active over time (people entering/exiting), the pool stays roughly balanced and LPs get:

Small but steady fees,

Low risk of lasting depeg (for this pair),

So APY ends up around 0.5–1% in many LST/SOL pools. 
Exponential DeFi
+1

Reserve side (what you’re building)

Completely different shape:

You hold only SOL, no JitoSOL → no IL risk.

You only get tapped when there is net JitoSOL → SOL pressure that can’t be satisfied at near-par on AMMs + Infinity.

That means:

On boring days, you might see near zero flow.

On volatile / panic / giant-whale days, a ton of size might hit you at 50–300 bps effective fees.

So your PnL profile is:

Spiky, not smooth.

Very event-driven (big flows, big market moves, or system shocks).

More like underwriting “instant exit insurance” than passive LP.

You’re literally monetizing being the backstop when:

Orca/Meteora depth (say $20–30m across both) plus Infinity is not enough for the flow to exit at par, and

Jupiter’s Router sees:

JitoSOL → AMM path = bad price (0.97, 0.95…)

JitoSOL → your Reserve = better net price (say 0.985 with a 150 bp fee)

→ so it routes user flow to you.

3. So how do you actually make money?

Mechanically, your monetization comes from three levers:

1. Utilization-based fee / spread

You define a fee curve based on how full your Reserve is:

Low utilization (tons of SOL available): small fee (e.g. ~10 bps) → you’re competitive with Infinity and AMMs for “normal” size.

Medium utilization (reserve filling): fee ramps (20–50+ bps).

High utilization (close to empty): fee gets aggressive (100–300+ bps), similar spirit to Sanctum Reserve’s 8–800 bps band. 
Solana Compass

Those high-utilization trades are where you make real money: you’re getting paid 1–3% (or more) to give someone instant fungible SOL now, while you unwind over 1–2 epochs in the background.

2. Capturing the “impatience premium”

JitoSOL already yields staking + MEV. From a pure cash-flow POV, someone who holds to maturity gets that full yield.

When someone says:

“I don’t care about the next 1–2 epochs of yield or the slightly better exit; I want SOL now.”

they’re paying you an impatience premium, implicitly:

They give you JitoSOL at a discount vs its “fair” intrinsic value (SOL+yield).

You burn it, take the stake account, ride out the cooldown and keep that yield + any discount as extra carry.

That impatience premium is structurally how you beat just staking that 5–10% yourself.

3. Size + path priority on Jupiter

Right now, the total JitoSOL/SOL on Orca + Meteora is only in the mid-7 figures (≈ $6.8m on Orca and $7.1m on Meteora in the main pools). 
orca.so
+1

If you park $100–200m of SOL behind a JitoSOL → SOL route in Jupiter (via a Router-style integration, like Sanctum did):

For small trades, DEX + Infinity + CL will often be best and you’ll see little flow (which is fine; you don’t want to spend your reserve on $100 swaps).

For big trades or in stress, your route will often dominate because:

AMM path: high price impact + “cheap” fee,

Your Reserve: low price impact + higher explicit fee,

But net user execution may still be better via you.

That means you monetize size-sensitive orderflow—the exact stuff that doesn’t pay much to AMMs because it just drives them off-peg instead.

4. Answering your question directly

“How do we monetize our 1 sided SOL liquidity in our Reserve?”

By not doing what Orca/Meteora LPs do.

You monetize by:

Only being used when AMM + Infinity liquidity is insufficient or too expensive for JitoSOL → SOL exits.

Charging a utilization-based fee/spread for instant exit that:

is worse than AMMs in normal times, so you don’t cannibalize staking yield,

but better than AMM routes in stress, so Jupiter routes size to you.

Capturing the impatience premium of users who accept a discount to offload JitoSOL now, while you slowly unwind at fair value via burn → stake → unstake, keeping the difference.

“Is the idea that in big movements, the ~$30m TVL isn’t enough, and they’ll need to come to our reserve?”

Yes, that’s exactly the idea.

The Reserve is there for the regimes where:

The existing ~$30m JitoSOL/SOL liquidity (plus Infinity) isn’t enough to keep JitoSOL near its intrinsic SOL value for the size that wants out, and

You step in to offer much deeper SOL liquidity with a controlled, explicit premium.

On normal, sleepy days, your Reserve might earn almost nothing (just like Sanctum’s Reserve mostly idles and is only earning ~0.5% APR on its TVL). 
sanctum.so
+1

On big days, you get paid heavily for being the one who:

Takes the other side of a crowded exit,

Has intimate knowledge of JitoSOL’s stake/MEV dynamics, and

Can absorb 1–2 epochs of unwind risk in exchange for a 1–3%+ convenience fee on large blocks.

That’s how you turn 5–10% of the pool into something that on average beats staking APY, while still existing primarily as an “airbag” for JitoSOL liquidity rather than just another low-yield LP position.
