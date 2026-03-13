# Effort model (water-aware de-biasing)

## Motivation

Ranking events are not IID catch samples; they are heavily affected by *where players fish*.

We estimate a spatial effort surface `e(tile)` and debias individual events by inverse effort.

## 1) Query scope

Effort is computed per query window:

- `[from_ts_utc, to_ts_utc)`
- optional recency decay: `half_life_days`

Use the same time weighting for:
- effort estimation
- zone evidence estimation

## 2) Raw tile event mass

For each event in-window with `water_ok = true`, define time weight:

- if half-life disabled: `w_time = 1`
- else:
  - `age_days = (to_ts - ts) / 86400`
  - `w_time = 2^(-age_days / half_life_days)`

Accumulate:
- `E_raw[tile] += w_time`

## 3) Water area per tile

Precompute once for a given `tile_px`:

- `M[tile] = count of pixels where is_water(px,py) within that tile`

## 4) Water-aware smoothing (normalized convolution)

To avoid land contamination near coasts, blur *both* E and M:

- `E_blur = gaussian_blur(E_raw, sigma_tiles)`
- `M_blur = gaussian_blur(M, sigma_tiles)`

Then define effort intensity per water area:
- `effort[tile] = E_blur[tile] / max(M_blur[tile], eps)`

`eps` prevents division by zero.

### Gaussian blur implementation
Use separable 1D convolution in x then y.
Kernel radius can be `ceil(3*sigma_tiles)`.

## 5) Inverse-effort weights

Compute a robust scale, e.g. median over tiles with `E_raw>0`:

- `eff_med = median(effort[tile] | E_raw[tile] > 0)`

Event effort weight:
- `w_eff = eff_med / max(effort[tile], eps_eff)`

Clip weights to avoid variance explosion:
- `w_eff = clamp(w_eff, w_eff_min, w_eff_max)`
  - defaults: `w_eff_min=0.1`, `w_eff_max=10`

## 6) Interpretation

Inverse-effort weighting approximates:
- downweighting observations from high-effort tiles
- upweighting observations from low-effort tiles

This cannot fully remove all biases (e.g., fish-specific hunting), but it is a strong first-order correction.

## 7) Optional per-fish normalization (recommended for ranking)

Ranking frequency differs massively by fish (some fish are popular or easier to trophy).

Define per-fish normalization for the current query window:

- `S_f = Σ_{events with fish=f} w_time`
- `w_fish = 1 / max(S_f, eps_fish)`

Final event weight (for zone signatures):
- `w = w_time * w_eff * w_fish`

If per-fish normalization is enabled, also compute confidence metrics using the **non-normalized weights** (see confidence doc), because `w_fish` changes the meaning of “sample size”.

