#!/usr/bin/env bash
# task:ova-open-cells — O1 … O7.  (R8.)
#
# The smallest unit in the decomposition, and the one whose absence is least visible: a blank cell in
# an open row reads as "not applicable" to one person and "not filled in yet" to the next, and both
# of them stop asking. The em dash is the only value that says *decided, and nothing goes here*.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

declare_row O1 "every open row's Guarantee is exactly the em dash"
declare_row O2 "every open row's Reason for adopters at is exactly the em dash"
declare_row O3 "every open row's Follow-up is exactly the em dash"
declare_row O4 "empty, whitespace, -, n/a, none and TBD are each red in an open row"
declare_row O5 "at least one open row exists, so O1-O3 are not vacuously true"
declare_row O6 "every violation names the row's Declaration and the offending column"
declare_row O7 "rows inspected equals the number of open verdicts the table carries"

# The predicate, defined once and reported on by O4. Exactly the em dash: not trimmed-to-empty, not
# a hyphen, not a word that happens to mean nothing.
is_emdash() { [ "$1" = "$EMDASH" ]; }

audit_present || { red_all "no $AUDIT_REL"; finish; exit; }

OPEN_ROWS="$(rows_with_verdict "$AUDIT" open)"
OPEN_N="$(printf '%s\n' "$OPEN_ROWS" | grep -c .)"
INSPECTED=0
VIOLATIONS=0

# One pass, three columns, and the reasons collected as they are found — O6 wants the row named, not
# a count, so the `why` lines below are the row's own output and not a summary.
declare -A COLUMN_BAD=([5]=0 [6]=0 [7]=0)
if [ "${OPEN_N:-0}" -gt 0 ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    INSPECTED=$((INSPECTED + 1))
    decl="$(cell "$line" "$COL_DECLARATION")"
    for col in "$COL_GUARANTEE" "$COL_REASON" "$COL_FOLLOWUP"; do
      value="$(cell "$line" "$col")"
      if ! is_emdash "$value"; then
        COLUMN_BAD[$col]=$(( ${COLUMN_BAD[$col]} + 1 ))
        VIOLATIONS=$((VIOLATIONS + 1))
        why "open row '${decl:-<no declaration>}' has ${COLUMNS[$((col - 1))]} = '${value:-<empty>}'"
      fi
    done
  done <<< "$OPEN_ROWS"
fi

row O1 "$([ "${COLUMN_BAD[5]}" -eq 0 ] && echo 0 || echo 1)"
row O2 "$([ "${COLUMN_BAD[6]}" -eq 0 ] && echo 0 || echo 1)"
row O3 "$([ "${COLUMN_BAD[7]}" -eq 0 ] && echo 0 || echo 1)"

# ---- O4 -----------------------------------------------------------------------------------------
# The six near-misses, each put through the predicate the rows above are decided by. A predicate that
# accepted any of them would make O1-O3 green on exactly the cells R8 exists to forbid.
R=0
for probe in "" " " "-" "n/a" "none" "TBD"; do
  if is_emdash "$probe"; then
    R=1; why "the em-dash predicate accepts '${probe:-<empty>}' in an open row"
  fi
done
is_emdash "$EMDASH" || { R=1; why "the em-dash predicate rejects the em dash itself"; }
row O4 "$R"

# ---- O5 -----------------------------------------------------------------------------------------
R=0
[ "${OPEN_N:-0}" -ge 1 ] || { R=1; why "the table carries no open row, so O1-O3 assert nothing"; }
row O5 "$R"

# ---- O6 -----------------------------------------------------------------------------------------
# The reporting requirement, checked against what this run actually printed: every violation counted
# above produced a named `why` line, because the two are the same loop.
R=0
if [ "$VIOLATIONS" -gt 0 ]; then
  note "$VIOLATIONS violation(s), each named above with its row and column"
fi
if [ "${OPEN_N:-0}" -gt 0 ]; then
  MISSING_DECL=0
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    [ -n "$(cell "$line" "$COL_DECLARATION")" ] || MISSING_DECL=$((MISSING_DECL + 1))
  done <<< "$OPEN_ROWS"
  [ "$MISSING_DECL" -eq 0 ] \
    || { R=1; why "$MISSING_DECL open row(s) have an empty Declaration — a violation there cannot be named"; }
else
  R=1; why "no open rows, so no violation could be named"
fi
row O6 "$R"

# ---- O7 -----------------------------------------------------------------------------------------
# Coverage: a row that fell out of the loop is a row that passed by not being looked at.
R=0
[ "$INSPECTED" -eq "${OPEN_N:-0}" ] \
  || { R=1; why "inspected $INSPECTED row(s) of ${OPEN_N:-0} open verdict(s)"; }
row O7 "$R"

finish
