# Zone signatures (evidence distributions)

## Query inputs

- `map_version_id` → zone mask image to use
- `[from_ts, to_ts)` in UTC
- `half_life_days` (optional)
- effort params: `tile_px`, `sigma_tiles`, weight clipping
- `per_fish_normalize` (bool)
- `alpha0` (Dirichlet prior strength)
- `top_k` for UI

## 1) Assign each event to a zone RGB

For each in-window event with `water_ok=true`:

1) Read pixel from zone mask image at `(water_px, water_py)`:
   - `zone_rgb = (R,G,B)` at that pixel
2) `zone_key = "R,G,B"`

This assigns each event to the **community-defined zone**.

## 2) Accumulate weighted evidence counts

For each event:
- compute weights:
  - `w_time` (recency)
  - `w_eff` (inverse effort)
  - optional `w_fish` (per-fish normalize)
- `w = w_time * w_eff * (w_fish or 1)`

Accumulate:
- `C[zone_key, fish_id] += w`

Also track zone totals with a weight definition suitable for confidence:
- `W_zone = Σ (w_time * w_eff)`  (exclude fish normalization)
- `W2_zone = Σ (w_time * w_eff)^2`
- `last_seen_ts = max(ts)` for events in zone

## 3) Prior and posterior

Because counts are sparse and biased, use Dirichlet smoothing.

### Empirical global prior

Compute global evidence counts across all zones in the query window:
- `C_global[fish] = Σ_z C[z,fish]`
- `p0[fish] = C_global[fish] / Σ_f C_global[f]`

If global totals are zero (no data), return Unknown.

### Dirichlet posterior per zone

For each zone z and fish f:

- `alpha[z,f] = alpha0 * p0[f] + C[z,f]`
- `alpha_total[z] = Σ_f alpha[z,f]`

Posterior mean (the displayed “evidence share”):
- `p_hat[z,f] = alpha[z,f] / alpha_total[z]`

## 4) Credible intervals per fish (UI top-K)

For a given fish f:
- marginal distribution is Beta:
  - `p[z,f] ~ Beta(alpha[z,f], alpha_total[z]-alpha[z,f])`

Compute e.g. 5% and 95% quantiles for UI.
(Implement via an inverse incomplete beta function or a numeric approximation.)

In v1, if implementing beta quantiles is heavy, use:
- normal approximation on logit for large alpha
- or Monte Carlo sampling from Dirichlet for top-K fish only.

## 5) Output for UI

For each zone z:
- `zone_key`, name/metadata from `zones_merged`
- `W_zone`, ESS (see confidence doc), last_seen_ts
- top-K fish:
  - `fish_id`, fish_name
  - `p_hat`
  - credible interval [lo, hi]
  - raw evidence `C[z,f]`

## 6) Important interpretation note

`p_hat` is **not** the in-game drop probability.
It is an **effort-debiased evidence share** derived from ranking observations.

Always label it accordingly in UI (“evidence share”).
