#!/usr/bin/env bash
# task:ova-table-shape — T1 … T8.  (R3, R4, R11.)
#
# The shape every column-reading sibling depends on. If this is red, the siblings are reading
# something that is not the table, so their rows are noise — which is why `units.tsv` puts this unit
# ahead of them.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

declare_row T1 "the audit contains exactly one markdown table"
declare_row T2 "its header is the seven column names, in order, cell for cell after trimming"
declare_row T3 "every data row has exactly seven cells"
declare_row T4 "every Verdict cell is exactly open or closed — no third value, no hedge"
declare_row T5 "at least one row is open and at least one is closed"
declare_row T6 "the data row count is at least the candidate count the scan emits"
declare_row T7 "emptying the table turns T5 and T6 red — the quantified rows are not vacuous"
declare_row T8 "the table parser is defined once in lib.sh, and every sibling reads through it"

# ---- T8 -----------------------------------------------------------------------------------------
# Reported first because it does not depend on the audit existing, and because a suite where two
# checks parse the table differently cannot be reasoned about at all.
R=0
grep -q '^table_rows()' "$CHECKS_DIR/lib.sh" || { R=1; why "lib.sh defines no table_rows"; }
grep -q '^cell()' "$CHECKS_DIR/lib.sh" || { R=1; why "lib.sh defines no cell"; }
for script in "$CHECKS_DIR"/check-*.sh; do
  base="${script##*/}"
  grep -q 'source .*lib\.sh' "$script" || { R=1; why "$base does not source lib.sh"; }
  grep -qE '^(table_rows|table_header|cell)\(\)' "$script" \
    && { R=1; why "$base re-defines the table parser instead of reading through lib.sh"; }
done
row T8 "$R"

audit_present || { red_all "no $AUDIT_REL"; finish; exit; }

# ---- T1 -----------------------------------------------------------------------------------------
R=0
BLOCKS="$(table_block_count "$AUDIT")"
if [ "${BLOCKS:-0}" -ne 1 ]; then
  R=1
  why "the audit contains ${BLOCKS:-0} markdown table(s); R3 wants exactly one"
  while IFS= read -r ln; do
    [ -n "$ln" ] && why "table starting at $AUDIT_REL:$ln"
  done < <(table_block_starts "$AUDIT")
fi
row T1 "$R"

# ---- T2 -----------------------------------------------------------------------------------------
# The checks parse by header, so a renamed or reordered column is not cosmetic — it silently moves
# every sibling's cell index.
R=0
HEADER="$(table_header "$AUDIT")"
if [ -z "$HEADER" ]; then
  R=1; why "no header row found"
else
  GOT_N="$(awk -F'\t' '{ print NF }' <<< "$HEADER")"
  [ "${GOT_N:-0}" -eq "${#COLUMNS[@]}" ] \
    || { R=1; why "the header has ${GOT_N:-0} column(s); R3 fixes ${#COLUMNS[@]}"; }
  i=0
  while [ "$i" -lt "${#COLUMNS[@]}" ]; do
    want="${COLUMNS[$i]}"
    got="$(awk -F'\t' -v n="$((i + 1))" 'NF >= n { print $n }' <<< "$HEADER")"
    [ "$got" = "$want" ] \
      || { R=1; why "column $((i + 1)) is '${got:-<missing>}'; R3 fixes '$want'"; }
    i=$((i + 1))
  done
fi
row T2 "$R"

ROWS="$(table_rows "$AUDIT")"
ROW_N="$(printf '%s\n' "$ROWS" | grep -c .)"

# ---- T3 -----------------------------------------------------------------------------------------
R=0
if [ "${ROW_N:-0}" -eq 0 ]; then
  R=1; why "the table carries no data rows"
else
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    w="$(row_width "$line")"
    [ "$w" -eq "${#COLUMNS[@]}" ] \
      || { R=1; why "$AUDIT_REL:$(row_lineno "$line") has $w cell(s), not ${#COLUMNS[@]}"; }
  done <<< "$ROWS"
fi
row T3 "$R"

# ---- T4 -----------------------------------------------------------------------------------------
# R4 is the requirement with no room in it: two values, no hedge. `partial`, `mostly open` and
# `open*` are each the sentence an adopter would have believed.
R=0
if [ "${ROW_N:-0}" -eq 0 ]; then
  R=1; why "no data rows, so no verdict was checked"
else
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    v="$(cell "$line" "$COL_VERDICT")"
    case "$v" in
      open|closed) ;;
      *) R=1; why "$AUDIT_REL:$(row_lineno "$line") has Verdict '${v:-<empty>}' — R4 allows open or closed" ;;
    esac
  done <<< "$ROWS"
fi
row T4 "$R"

# ---- T5 -----------------------------------------------------------------------------------------
OPEN_N="$(rows_with_verdict "$AUDIT" open | grep -c .)"
CLOSED_N="$(rows_with_verdict "$AUDIT" closed | grep -c .)"
R=0
[ "${OPEN_N:-0}" -ge 1 ] || { R=1; why "no row is open"; }
[ "${CLOSED_N:-0}" -ge 1 ] || { R=1; why "no row is closed"; }
note "${OPEN_N:-0} open, ${CLOSED_N:-0} closed, ${ROW_N:-0} data row(s)"
row T5 "$R"

# ---- T6 -----------------------------------------------------------------------------------------
# R11's floor. Every quantified requirement above is true of the empty table; this is what stops it.
R=0
if ! scan_present; then
  R=1; why "no ${SCAN#"$REPO"/}, so the floor cannot be derived"
else
  CAND_N="$( ( cd "$REPO" && bash "$SCAN" 2>/dev/null ) | grep -c . )"
  [ "${ROW_N:-0}" -ge "${CAND_N:-0}" ] \
    || { R=1; why "${ROW_N:-0} data row(s) for ${CAND_N:-0} scan candidate(s)"; }
  note "floor: ${CAND_N:-0} candidate(s) from the scan"
fi
row T6 "$R"

# ---- T7 -----------------------------------------------------------------------------------------
# Shown, not asserted: the same predicates, run against an audit whose table has been emptied on a
# copy. If they still hold there, T5 and T6 are decorations.
R=0
LAB="$(scratch)"
if [ -z "$LAB" ]; then
  R=1; why "no scratch directory could be created"
else
  trap 'rm -rf "$LAB"' EXIT
  EMPTY="$LAB/emptied.md"
  # Keep the header and the separator, drop every data row.
  awk '
    /^[[:space:]]*\|/ { if (!inb) { inb = 1; n = 0 } ; n++; if (n <= 2) print; next }
    { inb = 0; print }
  ' "$AUDIT" > "$EMPTY"
  E_ROWS="$(table_rows "$EMPTY" | grep -c .)"
  E_OPEN="$(rows_with_verdict "$EMPTY" open | grep -c .)"
  E_CLOSED="$(rows_with_verdict "$EMPTY" closed | grep -c .)"
  [ "${E_OPEN:-0}" -eq 0 ] && [ "${E_CLOSED:-0}" -eq 0 ] \
    || { R=1; why "T5's predicate still holds on an emptied table"; }
  if scan_present; then
    CAND_N="${CAND_N:-$( ( cd "$REPO" && bash "$SCAN" 2>/dev/null ) | grep -c . )}"
    if [ "${CAND_N:-0}" -ge 1 ] && [ "${E_ROWS:-0}" -ge "${CAND_N:-0}" ]; then
      R=1; why "T6's floor still holds on an emptied table"
    fi
  else
    R=1; why "no scan, so T6's floor could not be shown to fail on an emptied table"
  fi
fi
row T7 "$R"

finish
