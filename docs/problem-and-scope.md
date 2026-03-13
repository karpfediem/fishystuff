# Problem and scope

## Problem statement

The community historically relied on dataleaks providing:

- a worldmap-sized **zone bitmap** (ZoneRGB per pixel)
- detailed **drop tables** (MainGroup/SubGroup) with item lists and rates

Current constraint: **no new dataleaks**, so for new regions/fish:

- boundaries of loot-table zones are unknown / unreliable
- new subgroup leaks may appear without item contents or without linkage to maingroups
- we must derive useful, maintainable information from **observations** (primarily ranking)

## What is observable today

Primary dataset:
- **fish size ranking entries** (global coverage; many fish have ≥10 entries)

Record schema (example):
`Date;EncyclopediaKey;Length;FamilyName;CharacterName;X;Y;Z`

We intentionally treat player names as non-essential and avoid storing them.

Secondary / future dataset:
- **direct catch logs** at known locations and contexts (few contributors; high value)

## What we can know reliably

- Fishable domain: exact `watermap.png` where **water pixels are RGB (0,0,255)**.
- Community zone geometry: zone mask images (RGB per pixel), updated rarely.
- RGB→name mapping and metadata: from Dolt `zones_merged` view exported as CSV.

## What we cannot know from ranking alone

- true loot-table probabilities (drop rates)
- full fish lists (absence is not observed)
- context-conditional variants (Guru thresholds, mastery brackets) unless explicitly logged

Therefore the outputs must be labeled as:
- **evidence distributions**
- accompanied by uncertainty
- with explicit “unknown / insufficient evidence” states

## Goals of the system

1) Provide a fast interactive map:
- pick a pixel → show the **community zone name** (RGB) and its evidence distribution.

2) Make results filterable by time / patch:
- select patch start date → recompute using only samples after it.

3) Debias spatial sampling:
- use an **effort map** to reduce “popular spots” dominance.

4) Provide confidence and freshness:
- show credible intervals / effective sample sizes
- warn if zone is stale or likely changed after a patch.

5) Support historical viewing:
- show distributions using older Dolt commits and older time windows.
