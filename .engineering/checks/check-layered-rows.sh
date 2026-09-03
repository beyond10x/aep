#!/usr/bin/env bash
# task:ova-layered-rows — L1 … L7.  (R5.)
#
# The worked example, and the rule it stands for: `artifacts/relations/*.yaml` lets an adopter
# declare which pairings their tree means, but every name an edge carries must be a variant of a
# closed Rust enum. The store layer is open and the value layer is not, and a single averaged verdict
# would be wrong in one of the two directions — which is to say, it would be the sentence an adopter
# believed.
#
# **The pair moved once.** Artifact status was this check's worked example until `ArtifactStatus`
# gained an `Other(String)` and its ladder took over the guarantee. Both of its layers are open now,
# so it no longer demonstrates anything about a split verdict, and a check anchored on it was
# asserting that a shipped opening had not happened. Relations are the pair that is still split, and
# the audit argues the split at length. The rule under test is unchanged; only the pair that shows it
# is.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ENUM_FILE="crates/govern/aep-domain/src/artifact.rs"
ENUM_DECL="pub enum RelationKind"
DOC_DIR="artifacts/relations"

declare_row L1 "the table carries two relation rows, distinguished by the layer each describes"
declare_row L2 "one is open, decided under artifacts/relations/; the other closed, decided in artifact.rs"
declare_row L3 "both Decided by paths exist, and the closed one's cited line carries the enum declaration"
declare_row L4 "no Declaration appears twice with the same Verdict — a duplicate must differ by verdict"
declare_row L5 "the audit states the one-row-per-layer rule and names both relation rows"
declare_row L6 "merging the two rows into one on a copy turns L1 red"
declare_row L7 "the closed row's line is resolved at check time, so moving the enum turns L3 red"

audit_present || { red_all "no $AUDIT_REL"; finish; exit; }

ROWS="$(table_rows "$AUDIT")"

# The join, on the Decided by path **and** the row being about a relation. The path alone was too
# loose: `artifact.rs` holds `ArtifactStatus` as well as `RelationKind`, so a table where the
# relation row had been deleted and any other row cited the same file answered L1 with the wrong
# row — and L6, which removes both rows and asks again, would have gone red on a citation that had
# nothing to do with the layer rule.
SUBJECT='relation'

pick_row() { # <column index> <needle in that column>
  awk -F'\t' -v c="$(( $1 + 1 ))" -v needle="$2" -v dc="$((COL_DECLARATION + 1))" -v s="$SUBJECT" \
    'NF >= c && index($c, needle) && index(tolower($dc), s) { print; exit }' <<< "$ROWS"
}

OPEN_ROW="$(pick_row "$COL_DECIDED" "$DOC_DIR")"
CLOSED_ROW="$(pick_row "$COL_DECIDED" "$ENUM_FILE")"

# ---- L1 -----------------------------------------------------------------------------------------
R=0
[ -n "$OPEN_ROW" ] || { R=1; why "no row decides a relation verdict under $DOC_DIR/"; }
[ -n "$CLOSED_ROW" ] || { R=1; why "no row decides a relation verdict in $ENUM_FILE"; }
if [ -n "$OPEN_ROW" ] && [ -n "$CLOSED_ROW" ]; then
  a="$(cell "$OPEN_ROW" "$COL_DECLARATION")"
  b="$(cell "$CLOSED_ROW" "$COL_DECLARATION")"
  [ "$a" != "$b" ] \
    || { R=1; why "both relation rows carry the same Declaration '$a' — neither says which layer it is about"; }
  note "document layer: '$a' · value layer: '$b'"
fi
row L1 "$R"

# ---- L2 -----------------------------------------------------------------------------------------
R=0
if [ -z "$OPEN_ROW" ] || [ -z "$CLOSED_ROW" ]; then
  R=1; why "one or both relation rows are missing, so their verdicts cannot be compared"
else
  vo="$(cell "$OPEN_ROW" "$COL_VERDICT")"
  vc="$(cell "$CLOSED_ROW" "$COL_VERDICT")"
  [ "$vo" = "open" ] || { R=1; why "the $DOC_DIR/ row has Verdict '$vo', not open"; }
  [ "$vc" = "closed" ] || { R=1; why "the $ENUM_FILE row has Verdict '$vc', not closed"; }
fi
row L2 "$R"

# ---- L3 / L7 ------------------------------------------------------------------------------------
# One resolution, two rows: L3 is "the citation is true today", L7 is "it is re-derived rather than
# remembered". The line number is read out of the cell and applied to the file on this run, so the
# enum moving in `artifact.rs` reddens the row instead of leaving a stale number behind.
L3_R=0; L7_R=0
if [ -z "$CLOSED_ROW" ]; then
  L3_R=1; L7_R=1; why "no closed relation row to resolve"
else
  [ -d "$REPO/$DOC_DIR" ] || { L3_R=1; why "$DOC_DIR/ does not exist"; }
  by="$(cell "$CLOSED_ROW" "$COL_DECIDED")"
  LINE="$(printf '%s' "$by" | tr -d '`' | grep -oE "$ENUM_FILE:[0-9]+" | head -1 | sed 's/.*://')"
  if [ -z "$LINE" ]; then
    L3_R=1; L7_R=1; why "the closed row's Decided by is '$by', not $ENUM_FILE:<line>"
  elif [ ! -f "$REPO/$ENUM_FILE" ]; then
    L3_R=1; why "$ENUM_FILE does not exist"
  else
    AT="$(sed -n "${LINE}p" "$REPO/$ENUM_FILE")"
    if grep -Fq "$ENUM_DECL" <<< "$AT"; then
      note "$ENUM_FILE:$LINE carries '$ENUM_DECL'"
    else
      L3_R=1
      why "$ENUM_FILE:$LINE reads '${AT:-<past end of file>}', not '$ENUM_DECL'"
      ACTUAL="$(grep -nF "$ENUM_DECL" "$REPO/$ENUM_FILE" | head -1 | cut -d: -f1)"
      [ -n "$ACTUAL" ] && why "the declaration is at line $ACTUAL — the citation is stale"
    fi
    # L7's own claim: the number came from the cell, not from this script.
    grep -qE "^[^#]*$ENUM_FILE:[0-9]+" "${BASH_SOURCE[0]}" \
      && { L7_R=1; why "this check hard-codes a line number instead of reading the cell"; }
  fi
fi
row L3 "$L3_R"

# ---- L4 -----------------------------------------------------------------------------------------
# The general form of R5. A repeated Declaration is legitimate only when the verdicts differ; the
# same declaration twice with the same verdict is a duplicate row, not a layer.
R=0
DUPES="$(awk -F'\t' -v d="$((COL_DECLARATION + 1))" -v v="$((COL_VERDICT + 1))" '
  NF >= v { key = $d "\t" $v; seen[key]++ }
  END { for (k in seen) if (seen[k] > 1) print seen[k] "\t" k }
' <<< "$ROWS")"
if [ -n "$DUPES" ]; then
  R=1
  while IFS=$'\t' read -r n decl verdict; do
    [ -z "$n" ] && continue
    why "'$decl' appears $n times with Verdict $verdict — a repeated declaration must differ by verdict"
  done <<< "$DUPES"
fi
row L4 "$R"

# ---- L5 -----------------------------------------------------------------------------------------
R=0
SECTION="$(section_by_heading "$AUDIT" 'layer')"
if [ -z "$SECTION" ]; then
  R=1; why "the audit has no section whose heading names the one-row-per-layer rule"
else
  grep -qiE 'one row per layer|two rows|per layer' <<< "$SECTION" \
    || { R=1; why "that section does not state the one-row-per-layer rule"; }
  for r in "$OPEN_ROW" "$CLOSED_ROW"; do
    [ -z "$r" ] && continue
    d="$(cell "$r" "$COL_DECLARATION")"
    grep -Fq "$d" <<< "$SECTION" || { R=1; why "the section does not name the row '$d'"; }
  done
  [ -n "$OPEN_ROW" ] && [ -n "$CLOSED_ROW" ] \
    || { R=1; why "one of the two rows does not exist, so the section cannot name both"; }
fi
row L5 "$R"

# ---- L6 -----------------------------------------------------------------------------------------
# Shown on a copy: the two rows merged into one qualified row, and L1's own predicate asked again.
R=0
if [ -z "$OPEN_ROW" ] || [ -z "$CLOSED_ROW" ]; then
  R=1; why "there are not two rows to merge"
else
  LAB="$(scratch)"
  if [ -z "$LAB" ]; then
    R=1; why "no scratch directory could be created"
  else
    trap 'rm -rf "$LAB"' EXIT
    MERGED="$LAB/merged.md"
    OL="$(row_lineno "$OPEN_ROW")"; CL="$(row_lineno "$CLOSED_ROW")"
    awk -v a="$OL" -v b="$CL" 'NR != a && NR != b { print }' "$AUDIT" > "$MERGED"
    M_ROWS="$(table_rows "$MERGED")"
    still_open="$(awk -F'\t' -v c="$((COL_DECIDED + 1))" -v d="$DOC_DIR" \
      -v dc="$((COL_DECLARATION + 1))" -v s="$SUBJECT" \
      'NF >= c && index($c, d) && index(tolower($dc), s) { print; exit }' <<< "$M_ROWS")"
    still_closed="$(awk -F'\t' -v c="$((COL_DECIDED + 1))" -v f="$ENUM_FILE" \
      -v dc="$((COL_DECLARATION + 1))" -v s="$SUBJECT" \
      'NF >= c && index($c, f) && index(tolower($dc), s) { print; exit }' <<< "$M_ROWS")"
    { [ -z "$still_open" ] && [ -z "$still_closed" ]; } \
      || { R=1; why "L1's predicate still finds both layers after the rows were merged away"; }
  fi
fi
row L6 "$R"

row L7 "$L7_R"

finish
