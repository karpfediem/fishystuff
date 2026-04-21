#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

xlsx2csv Fishing_Table.xlsx table.csv
awk 'BEGIN{FS=OFS=","} NR==1{print "index", $0} NR>1{print NR-1, $0}' table.csv > indexed.csv
dolt table import --replace-table "indexed" "indexed.csv"
dolt sql -c < sql/update_zone_index.sql
rm table.csv indexed.csv
