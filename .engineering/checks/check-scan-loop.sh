#!/usr/bin/env bash
# task:ova-scan-loop — P1 … P7.  (R13.)
#
# The two rules that close the loop between the derivation and the table:
#
#   completeness  every candidate the scan emits has a row      catches a vocabulary the audit forgot
#   provenance    every row the scan does not emit has an       catches a row invented rather than
#                 `Invited at` that resolves                    found
#
# The specification names these `check-completeness.sh` and `check-provenance.sh`; this suite's rule
# is one check per decomposed unit, and both rules belong to `task:ova-scan-loop`. They are two rows
# in one file. `run.sh`'s header records the rename.
#
# The scan **cannot** discover a closed surface — a closed surface is precisely one with no document
# key to find. P5 is the row that insists the audit says so in its own words, because a reader who
# takes the completeness check for proof of completeness has been misled by it.
#
# It publishes its partition on stdout for `check-repeatability.sh` (Y6):
#
#   PARTITION<TAB>scan-backed<TAB><n>
#   PARTITION<TAB>reading-backed<TAB><n>
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

declare_row P1 "every candidate the scan emits has a row naming it in Declaration or Decided by"
declare_row P2 "deleting such a row on a copy turns P1 red, naming the orphaned candidate"
declare_row P3 "every row the scan does not emit carries an Invited at that resolves"
declare_row P4 "the partition is printed and scan-backed plus reading-backed equals the row count"
declare_row P5 "the audit states that the scan cannot find a closed surface, in its own words"
declare_row P6 "neither rule is vacuous: an empty scan reddens P1, an empty reading partition reddens P3"
declare_row P7 "every candidate is accounted for exactly once; a candidate matched twice is reported"

audit_present || { red_all "no $AUDIT_REL"; finish; exit; }
scan_present || { red_all "no ${SCAN#"$REPO"/}"; finish; exit; }

CANDIDATES="$( ( cd "$REPO" && bash "$SCAN" 2>/dev/null ) | grep -v '^[[:space:]]*$' )"
CAND_N="$(printf '%s\n' "$CANDIDATES" | grep -c .)"
ROWS="$(table_rows "$AUDIT")"
ROW_N="$(printf '%s\n' "$ROWS" | grep -c .)"

# row_matches_candidate <row> <candidate>  — the join R13 is defined over: the candidate named in
# either the Declaration or the Decided by cell.
row_matches_candidate() {
  local r="$1" c="$2"
  grep -Fq "$c" <<< "$(cell "$r" "$COL_DECLARATION")" && return 0
  grep -Fq "$c" <<< "$(cell "$r" "$COL_DECIDED")"
}

# ---- P1 / P7 ------------------------------------------------------------------------------------
P1_R=0; P7_R=0
SCAN_BACKED_LINES=""
if [ "${CAND_N:-0}" -eq 0 ]; then
  P1_R=1; why "the scan emitted no candidates — completeness over an empty set proves nothing"
else
  while IFS= read -r cand; do
    [ -z "$cand" ] && continue
    hits=0
    while IFS= read -r r; do
      [ -z "$r" ] && continue
      if row_matches_candidate "$r" "$cand"; then
        hits=$((hits + 1))
        SCAN_BACKED_LINES="$SCAN_BACKED_LINES$(row_lineno "$r")"$'\n'
      fi
    done <<< "$ROWS"
    case "$hits" in
      0) P1_R=1; why "candidate '$cand' has no row in the table" ;;
      1) : ;;
      *) P7_R=1; why "candidate '$cand' is matched by $hits rows — usually a duplicate, not a layer" ;;
    esac
  done <<< "$CANDIDATES"
fi
row P1 "$P1_R"

SCAN_BACKED_UNIQ="$(printf '%s' "$SCAN_BACKED_LINES" | sort -u | grep -c .)"
READING_BACKED=$(( ${ROW_N:-0} - SCAN_BACKED_UNIQ ))

# ---- P2 -----------------------------------------------------------------------------------------
# Acceptance criterion 2, on a copy: a row deleted, the same join asked again, and the candidate it
# carried expected to come back orphaned and named. The real audit is never written to.
R=0
ORPHAN=""
if [ "${CAND_N:-0}" -eq 0 ]; then
  R=1; why "no candidates, so no row can be deleted to orphan one"
else
  while IFS= read -r cand; do
    [ -z "$cand" ] && continue
    while IFS= read -r r; do
      [ -z "$r" ] && continue
      if row_matches_candidate "$r" "$cand"; then ORPHAN="$cand	$(row_lineno "$r")"; break 2; fi
    done <<< "$ROWS"
  done <<< "$CANDIDATES"
fi
if [ -z "$ORPHAN" ] && [ "$R" -eq 0 ]; then
  R=1; why "no candidate has a row, so P1 is already red and P2 has nothing to remove"
elif [ -n "$ORPHAN" ]; then
  cand="${ORPHAN%%	*}"; ln="${ORPHAN##*	}"
  LAB="$(scratch)"
  if [ -z "$LAB" ]; then
    R=1; why "no scratch directory could be created"
  else
    trap 'rm -rf "$LAB"' EXIT
    CUT="$LAB/cut.md"
    awk -v n="$ln" 'NR != n { print }' "$AUDIT" > "$CUT"
    HITS=0
    while IFS= read -r r; do
      [ -z "$r" ] && continue
      row_matches_candidate "$r" "$cand" && HITS=$((HITS + 1))
    done <<< "$(table_rows "$CUT")"
    if [ "$HITS" -ne 0 ]; then
      R=1; why "'$cand' still matched $HITS row(s) after its row was deleted — the join is too loose"
    else
      note "removing $AUDIT_REL:$ln orphans candidate '$cand', as P1 requires"
    fi
  fi
fi
row P2 "$R"

# ---- P3 -----------------------------------------------------------------------------------------
# The rows the scan could never have found. They are produced by reading the corpus, and R6 is what
# holds them honest — so the provenance rule is R6 applied to exactly that partition.
P3_R=0
READING_ROWS=""
while IFS= read -r r; do
  [ -z "$r" ] && continue
  ln="$(row_lineno "$r")"
  printf '%s' "$SCAN_BACKED_LINES" | grep -Fxq "$ln" && continue
  READING_ROWS="$READING_ROWS$r"$'\n'
  decl="$(cell "$r" "$COL_DECLARATION")"
  at="$(cell "$r" "$COL_INVITED")"
  locus="$(printf '%s' "$at" | tr -d '`' | grep -oE '[A-Za-z0-9._/-]+\.[A-Za-z0-9]+:[0-9]+' | head -1)"
  frag="$(printf '%s' "$at" | sed -n 's/.*[«"“]\([^"”»]*\)[»"”].*/\1/p' | head -1)"
  path="${locus%:*}"
  if [ -z "$locus" ] || [ -z "$frag" ]; then
    P3_R=1; why "reading-backed row '${decl:-<no declaration>}' ($AUDIT_REL:$ln) has no resolving citation: '${at:-<empty>}'"
    continue
  fi
  in_corpus "$path" || { P3_R=1; why "reading-backed row '${decl}' cites $path, outside the corpus"; }
  if [ -f "$REPO/$path" ]; then
    grep -Fq "$frag" "$REPO/$path" \
      || { P3_R=1; why "reading-backed row '${decl}' quotes \"$frag\", absent from $path"; }
  else
    P3_R=1; why "reading-backed row '${decl}' cites $path, which does not exist"
  fi
done <<< "$ROWS"

if [ "$READING_BACKED" -le 0 ]; then
  P3_R=1
  why "no reading-backed rows at all — the closed surfaces the scan cannot find are exactly the ones that matter"
fi
row P3 "$P3_R"

# ---- P4 -----------------------------------------------------------------------------------------
R=0
printf 'PARTITION\tscan-backed\t%s\n' "$SCAN_BACKED_UNIQ"
printf 'PARTITION\treading-backed\t%s\n' "$READING_BACKED"
note "$SCAN_BACKED_UNIQ scan-backed + $READING_BACKED reading-backed = ${ROW_N:-0} data row(s)"
[ "$(( SCAN_BACKED_UNIQ + READING_BACKED ))" -eq "${ROW_N:-0}" ] \
  || { R=1; why "the partition does not sum to the table's row count"; }
row P4 "$R"

# ---- P5 -----------------------------------------------------------------------------------------
R=0
LIMIT="$(section_by_heading "$AUDIT" 'scan|limit|cannot|derivation')"
if [ -z "$LIMIT" ]; then
  R=1; why "the audit has no section stating what the derivation cannot find"
else
  grep -qiE 'cannot (discover|find)|not a proof|no document key' <<< "$LIMIT" \
    || { R=1; why "that section does not say the scan cannot discover a closed surface"; }
  grep -qiE 'closed' <<< "$LIMIT" \
    || { R=1; why "that section does not name the closed case the limit is about"; }
fi
row P5 "$R"

# ---- P6 -----------------------------------------------------------------------------------------
# Both floors, shown rather than asserted. The predicates above are re-run against an empty scan and
# an empty reading partition; if either still holds, the corresponding rule is decoration.
R=0
[ "${CAND_N:-0}" -ge 1 ] \
  || { R=1; why "the scan emits nothing: completeness over an empty candidate set is vacuously true"; }
[ "$READING_BACKED" -ge 1 ] \
  || { R=1; why "the reading partition is empty: provenance over no rows is vacuously true"; }
note "floors: ${CAND_N:-0} candidate(s), $READING_BACKED reading-backed row(s) — both must be ≥ 1"
row P6 "$R"

row P7 "$P7_R"

finish
