# Drift detection and outdatedness

Goal: flag zones that are likely changed after a patch (loot table changed and/or community boundary outdated).

We separate two concepts:

- **Stale**: not enough recent evidence to say anything about current state.
- **Drifting**: strong evidence that the zone’s fish signature changed across a patch boundary.

## 1) Staleness rule

A zone is stale in a query window if either:
- no observations → Unknown
- `age_days_last > stale_days_threshold` (e.g., 120)
- or `ESS_recent < ESS_min_recent` (e.g., 10)

This is independent of drift.

## 2) Patch-scoped drift comparison

Given a patch boundary time `t0` (UTC), choose two windows:

- OLD window: `[t_old_from, t0)`
- NEW window: `[t0, t_new_to)`

Preferred choice:
- `t_old_from = previous_patch_start`
- `t_new_to = next_patch_start or query.to_ts`

If patch table lacks end times, define:
- OLD window length `L_days` (e.g., 60–120) before t0

Compute zone posteriors separately for OLD and NEW with the same params (effort correction and smoothing done within each window).

## 3) Divergence metric

Compute divergence between the two distributions.

Recommended:
- Jensen–Shannon divergence (JSD) on posterior means.

- `D_mean = JSD(p_hat_old, p_hat_new)`

Because JSD is bounded, thresholds are interpretable.

## 4) Uncertainty-aware drift probability

Compute `p_drift = P(D > D_thresh)` by Monte Carlo:

- sample `p_old ~ Dir(alpha_old)` and `p_new ~ Dir(alpha_new)`
- compute `D = JSD(p_old, p_new)`
- repeat N times (e.g., 300)
- `p_drift = fraction(D > D_thresh)`

Choose:
- `D_thresh` (e.g., 0.10)
- N (e.g., 300)
- RNG seed fixed for determinism (seed derived from zone_key + t0 + params)

## 5) Drift flag rule

Flag as drifting if:
- `ESS_old >= ESS_min` and `ESS_new >= ESS_min` (e.g., 10)
- and `p_drift >= p_thresh` (e.g., 0.95)

If ESS is low, report “insufficient evidence for drift test”.

## 6) UI outputs

For a zone:
- status: Fresh / Stale / Drifting / Unknown
- show:
  - `D_mean`
  - `p_drift`
  - ESS_old/new
  - top fish lists side-by-side (OLD vs NEW)

## 7) Interpretation cautions

Drift does not identify the root cause:
- loot table changed
- boundary changed
- sampling bias changed

But it is the correct trigger for:
- warning “zone possibly outdated”
- prompting community validation / redraw
- filtering by patch-date
