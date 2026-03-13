# Boundary QA and update suggestions (no dataleaks)

This section describes optional analytics to help maintain community zone masks.

The goal is not automatic edits, but:
- identify suspicious boundaries
- prioritize where to collect direct logs
- propose candidate adjustments with evidence

## 1) Tile-level signatures

Compute the same evidence distribution method, but per tile:

- tile signature `p_hat[tile, fish]`
- tile ESS and last seen

Only compute for tiles with sufficient ESS.

## 2) Edge divergence map

For adjacent tiles (4-neighborhood), compute:
- `edge_div = JSD(p_hat[tile_a], p_hat[tile_b])`

Optionally weight by ESS:
- `edge_div_weighted = edge_div * (1 - exp(-min(ESS_a, ESS_b)/tau))`

## 3) Boundary consistency checks vs zone mask

Let `zone(tile)` be the zone RGB sampled at the tile center (or majority water pixel).

Define:
- boundary edges: `zone(a) != zone(b)`
- interior edges: `zone(a) == zone(b)`

Heuristics:
- boundary edges with **low** divergence → boundary may be misplaced (zones look similar)
- interior edges with **high** divergence → zone may be heterogeneous (needs split or boundary move)

Produce:
- a ranked list of “most suspicious edges”
- and an overlay heatmap

## 4) Patch-aware boundary QA

Run the same QA in two windows around a patch boundary:
- compare edge divergence maps
- edges whose divergence changes strongly may indicate boundary/loot shifts

## 5) Suggesting candidate updates (advanced)

In a limited region of interest:
- treat tiles as nodes
- edge weights = divergence
- compute a constrained segmentation (region merging or MRF labeling) to propose a new partition
- map the partition back to RGB labels by overlap (or create new guessed RGBs)

Important:
- never auto-apply
- always present as “candidate mask version” for review

## 6) Deliverables

- `boundary_edges.json` for UI overlay
- `suspicious_edges.csv` report
- optional region-of-interest suggested mask patch (image diff)
