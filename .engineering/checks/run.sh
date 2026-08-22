#!/usr/bin/env bash
# The verifiers for W4-2 — one check per decomposed unit, one table, an honest exit code.
#
# This is the file `drivers/development/checks.yaml` names as the map's verifier:
#
#   bash .engineering/checks/run.sh
#
# Written in the `establish_verifiers` state, **before** `docs/guide/open-vocabulary.md`,
# `scan-declarations.sh` or the follow-up artifacts exist. It is therefore red, and being red is the
# state's product: a check that passes before the thing it checks exists is a check of nothing.
#
# The model is `integrations/claude-code/eval/checks/run-checks.sh`, read and not edited.
#
# ## Two names this run moved, and why
#
# The specification's deliverable line is *one check per decomposed unit, named for the unit it
# decides*, and that is the rule followed here: thirteen units in `units.tsv`, thirteen
# `check-<unit>.sh`. Three sibling task bodies name check scripts written before that rule was
# settled; they map onto this suite as:
#
#   check-corpus.sh                        -> check-audit-corpus.sh
#   check-completeness.sh, check-provenance.sh -> check-scan-loop.sh   (two rows, one unit, one file)
#
# Recorded here rather than silently renamed, because a task acceptance row that names a file
# nobody wrote is the failure this suite exists to catch.
#
#   bash .engineering/checks/run.sh                          every unit
#   bash .engineering/checks/run.sh table-shape open-cells   only those
#
# Never `set -e`: a runner that aborts mid-suite takes its report with it, and a report that did not
# print is indistinguishable from a suite with nothing to say.
set -uo pipefail

CHECKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNITS_FILE="${OVA_UNITS_FILE:-$CHECKS_DIR/units.tsv}"

# ---- the decomposition --------------------------------------------------------------------------

ALL=()
declare -A OWNER=()
declare -A DECIDES=()

if [ ! -f "$UNITS_FILE" ]; then
  printf 'FAIL  ----  no unit list at %s — the runner does not know what it is deciding\n' "$UNITS_FILE"
  printf '\n== W4-2 verifiers: 0 pass, 1 fail, 0 broken check(s) ==\n'
  exit 1
fi

while IFS=$'\t' read -r unit owner decides; do
  case "$unit" in ''|'#'*) continue ;; esac
  ALL+=("$unit")
  OWNER["$unit"]="$owner"
  DECIDES["$unit"]="$decides"
done < "$UNITS_FILE"

SELECTED=("$@")
if [ "${#SELECTED[@]}" -eq 0 ]; then
  SELECTED=("${ALL[@]}")
  FULL_RUN=1
else
  FULL_RUN=0
fi

# ---- scratch ------------------------------------------------------------------------------------
# Never a literal temp path: this machine's tmpfs drops writes under pressure. Same fallback the
# model runner uses.
#
# Failing here used to `exit 1` on the spot, which printed a bare `mkdir:` line and **no table** —
# the one thing R16 has no exception for, found by the adversarial pass with a `TMPDIR` that could
# not be made. It is now a harness failure like any other: every selected unit goes red with the
# reason under it, and the table prints. A runner that cannot allocate scratch has decided nothing,
# and *decided nothing* has to be readable in the same place as every other verdict.

SCRATCH_BASE="${TMPDIR:-$HOME/.cache/claude-tmp}"
OUT=""
if mkdir -p "$SCRATCH_BASE" 2>/dev/null; then
  OUT="$(mktemp "$SCRATCH_BASE/ova-checks.XXXXXX" 2>/dev/null)" || OUT=""
fi
if [ -n "$OUT" ]; then
  trap 'rm -f "$OUT"' EXIT
else
  printf 'FAIL  ----  no scratch file under %s — no check can be run from here\n' "$SCRATCH_BASE"
fi

# ---- the run ------------------------------------------------------------------------------------

TOTAL_PASS=0
TOTAL_FAIL=0
BROKEN=0
VERDICT_UNITS=()
declare -A VERDICT=()

verdict_of() {
  local unit="$1" v="$2"
  VERDICT_UNITS+=("$unit")
  VERDICT["$unit"]="$v"
}

for name in "${SELECTED[@]}"; do
  owner="${OWNER[$name]:-}"
  printf '\n== %s  (%s) ==\n' "$name" "${owner:-unowned}"

  # R17's first half, applied to the selection itself: a name nobody declared is a failed row and
  # never a silent no-op. Asking for a unit that does not exist is a question the runner answers.
  if [ -z "$owner" ]; then
    printf 'FAIL  ----  %s is not a unit in %s\n' "$name" "${UNITS_FILE##*/}"
    BROKEN=$((BROKEN + 1))
    verdict_of "$name" FAIL
    continue
  fi

  script="$CHECKS_DIR/check-$name.sh"

  # R17: a check whose script is missing is a **failed row**, never a skipped one.
  if [ ! -f "$script" ]; then
    printf 'FAIL  ----  no check exists for %s (expected check-%s.sh)\n' "$name" "$name"
    BROKEN=$((BROKEN + 1))
    verdict_of "$name" FAIL
    continue
  fi

  # No scratch, no run — but a red row and a reason, never a silent exit. Reported per unit so the
  # table below carries one line for every unit the selection asked about.
  if [ -z "$OUT" ]; then
    printf 'FAIL  ----  %s could not be run: no scratch file under %s\n' "$name" "$SCRATCH_BASE"
    BROKEN=$((BROKEN + 1))
    verdict_of "$name" FAIL
    continue
  fi

  # `bash "$script"`, never `"$script"`: a missing execute bit is a property of the checkout, not a
  # verdict about the unit. The subshell is what keeps N4 true — a check that calls `exit`, writes
  # to stderr or kills itself cannot take this loop or the table below with it.
  bash "$script" > "$OUT" 2>&1
  status=$?
  cat "$OUT"

  pass=$(grep -c '^PASS ' "$OUT")
  fail=$(grep -c '^FAIL ' "$OUT")
  TOTAL_PASS=$((TOTAL_PASS + pass))
  TOTAL_FAIL=$((TOTAL_FAIL + fail))

  broke=0

  # The specification's vacuity invariant, applied to the harness itself: a check that produced
  # **zero** rows fails. A table with nothing in it goes green while checking nothing, and that is
  # the one outcome no report may ever produce.
  if [ "$((pass + fail))" -eq 0 ]; then
    printf 'FAIL  ----  %s produced no rows (exit %s) — a check that asserts nothing is not green\n' \
      "$name" "$status"
    broke=1
  elif [ "$status" -eq 0 ] && [ "$fail" -gt 0 ]; then
    printf 'FAIL  ----  %s exited 0 with %s red row(s) — its exit code disagrees with its table\n' \
      "$name" "$fail"
    broke=1
  elif [ "$status" -ne 0 ] && [ "$fail" -eq 0 ]; then
    printf 'FAIL  ----  %s exited %s with no red row — a failure nobody can read is not a report\n' \
      "$name" "$status"
    broke=1
  fi

  BROKEN=$((BROKEN + broke))
  if [ "$broke" -eq 0 ] && [ "$status" -eq 0 ]; then
    verdict_of "$name" PASS
  else
    verdict_of "$name" FAIL
  fi
done

# A `check-*.sh` beside this file that no unit claims is a failed row too — the mirror of R17. One
# direction catches a unit nobody wrote a check for; this one catches a check nobody owns.
UNDECLARED=0
if [ "$FULL_RUN" -eq 1 ]; then
  for script in "$CHECKS_DIR"/check-*.sh; do
    [ -f "$script" ] || continue
    base="${script##*/check-}"
    unit="${base%.sh}"
    [ -n "${OWNER[$unit]:-}" ] && continue
    printf '\nFAIL  ----  %s decides no declared unit — every check names the unit it decides\n' \
      "${script##*/}"
    UNDECLARED=$((UNDECLARED + 1))
  done
fi

# ---- the table ----------------------------------------------------------------------------------
# R16: printed on **every** path, including failure. Nothing above exits early, so there is no path
# that reaches the end of the suite without reaching this.

printf '\n== W4-2 verifiers ==\n\n'
printf '%-6s  %-18s  %-28s  %s\n' RESULT UNIT OWNS DECIDES
printf '%-6s  %-18s  %-28s  %s\n' '------' '------------------' \
  '----------------------------' '----------------------------'
for unit in "${VERDICT_UNITS[@]}"; do
  printf '%-6s  %-18s  %-28s  %s\n' \
    "${VERDICT[$unit]}" "$unit" "${OWNER[$unit]:-unowned}" "${DECIDES[$unit]:-<undeclared>}"
done

printf '\n%s pass, %s fail, %s broken check(s), %s undeclared check(s)\n' \
  "$TOTAL_PASS" "$TOTAL_FAIL" "$BROKEN" "$UNDECLARED"
printf 'units: %s\n' "$UNITS_FILE"

[ "$((TOTAL_FAIL + BROKEN + UNDECLARED))" -eq 0 ]
