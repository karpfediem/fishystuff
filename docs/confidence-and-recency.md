# Confidence and recency

This pipeline must expose confidence in a way that is:

- robust under reweighting
- understandable to users
- sensitive to recency
- patch-aware

## 1) Effective sample size (ESS) on weights

For a zone z, define weights for confidence as:
- `u_i = w_time_i * w_eff_i` (exclude per-fish normalization)

Then:
- `W = Σ u_i`
- `W2 = Σ u_i^2`
- `ESS = W^2 / max(W2, eps)`

ESS measures “how many equal-weight samples would carry the same weight mass”.

Use ESS as the main quantitative confidence proxy.

## 2) Recency

Track:
- `last_seen_ts`
- `age_days_last = (to_ts - last_seen_ts) / 86400`

Define a simple freshness score if desired:
- `fresh = exp(-age_days_last / tau_days)`
where `tau_days` could be 30–60.

But always display the raw `age_days_last` and `last_seen_ts`.

## 3) Uncertainty from posterior

Credible interval width for top fish is a useful indicator:
- if CI is wide → not confident about that fish’s share

This is complementary to ESS.

## 4) Recommended UI confidence classification

Define a 3-level badge from ESS and recency:

- High:
  - `ESS >= 50` and `age_days_last <= 30`
- Medium:
  - `ESS >= 15` and `age_days_last <= 90`
- Low:
  - else, but data exists
- Unknown:
  - no data in window

These thresholds should be configurable.

## 5) Recency weighting parameter guidance

Half-life examples:
- `half_life_days = 90`: emphasize last ~3 months
- `half_life_days = 180`: emphasize last ~6 months

If the goal is “what is current now”, default 90 is reasonable.

For historical exploration, disable half-life weighting.
