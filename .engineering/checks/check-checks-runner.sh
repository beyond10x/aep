#!/usr/bin/env bash
# task:ova-checks-runner — N1 … N8.
#
# The one check whose subject is the harness. It never invokes the real suite: N2 through N5 and N8
# are shown against a **synthetic decomposition** in a scratch directory — a copy of `run.sh`, a
# `units.tsv` naming units that exist nowhere else, and small checks written to order. That is why
# `units.tsv` is a file the runner reads rather than an array inside it: a list a test can
# substitute is a list a test can prove things about, without recursion.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

RUNNER="$CHECKS_DIR/run.sh"
UNITS="$CHECKS_DIR/units.tsv"

declare_row N1 "the real run prints one row per declared unit, each naming the task: id that owns it"
declare_row N2 "exit 0 when every check passes, non-zero when one fails — both shown"
declare_row N3 "a unit whose check-<unit>.sh is absent is a FAIL row naming it, never a SKIP"
declare_row N4 "the table prints even when a check exits non-zero, writes to stderr, or kills itself"
declare_row N5 "a selection runs only the named units; an unknown name is a failed row"
declare_row N6 "run.sh names no network program: curl, wget, nc, ssh, git fetch/clone/pull"
declare_row N7 "scratch goes under \$TMPDIR or the documented fallback, and is removed on exit"
declare_row N8 "the red baseline is reproducible: no check present means every row FAIL, exit non-zero"
declare_row N9 "the table prints when the runner's own scratch base cannot be created, not only when a check dies"

if [ ! -f "$RUNNER" ] || [ ! -f "$UNITS" ]; then
  red_all "run.sh or units.tsv is missing from ${CHECKS_DIR#"$REPO"/}"
  finish
  exit
fi

UNIT_NAMES=()
UNIT_OWNERS=()
while IFS=$'\t' read -r u o _; do
  case "$u" in ''|'#'*) continue ;; esac
  UNIT_NAMES+=("$u")
  UNIT_OWNERS+=("$o")
done < "$UNITS"

# ---- the synthetic decomposition ----------------------------------------------------------------

LAB="$(scratch)" || { red_all "no scratch directory could be created"; finish; exit; }
trap 'rm -rf "$LAB"' EXIT

lab_reset() {
  rm -rf "$LAB"/*
  cp "$RUNNER" "$LAB/run.sh"
}

lab_units() {
  : > "$LAB/units.tsv"
  local u
  for u in "$@"; do printf '%s\tunit:%s\tsynthetic unit %s\n' "$u" "$u" "$u" >> "$LAB/units.tsv"; done
}

lab_check() { # <unit> <body…>
  local u="$1"; shift
  { printf '#!/usr/bin/env bash\n'; printf '%s\n' "$@"; } > "$LAB/check-$u.sh"
}

lab_run() { ( cd "$LAB" && bash ./run.sh "$@" 2>&1 ); }

GREEN='printf "PASS  A1   synthetic green\n"'
RED='printf "FAIL  A1   synthetic red\n"; exit 1'

# ---- N1 -----------------------------------------------------------------------------------------
# The **real** unit list, run in the lab against green stubs.
#
# Not `bash run.sh` in the repository, and the reason is not shyness about cost: this check is one of
# the units the runner runs, and so is `mutation-proof`, which runs the suite on a copy. A check that
# invoked the real suite would re-enter itself without a fixed point. Substituting the checks and
# keeping the list is what makes the row-per-unit property observable at all.
R=0
lab_reset
cp "$UNITS" "$LAB/units.tsv"
for u in "${UNIT_NAMES[@]}"; do lab_check "$u" "$GREEN"; done
REAL="$(lab_run)"
if [ -z "$REAL" ]; then
  R=1; why "the runner printed nothing for the real unit list"
else
  i=0
  while [ "$i" -lt "${#UNIT_NAMES[@]}" ]; do
    u="${UNIT_NAMES[$i]}"; o="${UNIT_OWNERS[$i]}"
    grep -qE "^(PASS|FAIL)[[:space:]]+$u[[:space:]]" <<< "$REAL" \
      || { R=1; why "no table row for unit $u"; }
    grep -qF "$o" <<< "$REAL" || { R=1; why "unit $u's row does not name $o"; }
    i=$((i + 1))
  done
fi
# The two directions the list and the directory can disagree in, asserted statically.
for u in "${UNIT_NAMES[@]}"; do
  [ -f "$CHECKS_DIR/check-$u.sh" ] || { R=1; why "unit $u has no check-$u.sh"; }
done
for script in "$CHECKS_DIR"/check-*.sh; do
  base="${script##*/check-}"; unit="${base%.sh}"
  printf '%s\n' "${UNIT_NAMES[@]}" | grep -Fxq "$unit" \
    || { R=1; why "check-$unit.sh decides no declared unit"; }
done
row N1 "$R"

# ---- N2 -----------------------------------------------------------------------------------------
R=0
lab_reset; lab_units alpha beta
lab_check alpha "$GREEN"; lab_check beta "$GREEN"
lab_run > /dev/null 2>&1 || { R=1; why "a synthetic suite of two green checks did not exit 0"; }

lab_check beta "$RED"
if lab_run > /dev/null 2>&1; then
  R=1; why "a synthetic suite with one red check exited 0"
fi
row N2 "$R"

# ---- N3 -----------------------------------------------------------------------------------------
R=0
lab_reset; lab_units alpha beta
lab_check alpha "$GREEN"          # beta's script is deliberately never written
OUT3="$(lab_run)"
STATUS3=$?
grep -qE '^FAIL' <<< "$OUT3" || { R=1; why "a missing check produced no FAIL row"; }
grep -q 'beta' <<< "$OUT3" || { R=1; why "the missing unit beta is not named in the output"; }
grep -qiE '\bSKIP\b' <<< "$OUT3" && { R=1; why "the runner emitted a SKIP; R17 allows only a failed row"; }
[ "$STATUS3" -eq 0 ] && { R=1; why "the run exited 0 with a check script missing"; }
row N3 "$R"

# ---- N4 -----------------------------------------------------------------------------------------
# Three ways for a check to die, one requirement: the table is still there afterwards.
R=0
for mode in exit1 stderr suicide; do
  lab_reset; lab_units alpha
  case "$mode" in
    exit1)   lab_check alpha "$RED" ;;
    stderr)  lab_check alpha 'printf "PASS  A1   ok\n"' 'printf "noise\n" >&2' 'exit 1' ;;
    suicide) lab_check alpha 'printf "PASS  A1   ok\n"' 'kill -9 $$' ;;
  esac
  OUT4="$(lab_run)"
  grep -q 'W4-2 verifiers' <<< "$OUT4" \
    || { R=1; why "no summary table after a check died by $mode"; }
  grep -qE '^(PASS|FAIL)[[:space:]]+alpha[[:space:]]' <<< "$OUT4" \
    || { R=1; why "no table row for alpha after $mode"; }
  lab_run > /dev/null 2>&1 && { R=1; why "the run exited 0 after a check died by $mode"; }
done
row N4 "$R"

# ---- N5 -----------------------------------------------------------------------------------------
R=0
lab_reset; lab_units alpha beta gamma
lab_check alpha "$GREEN"; lab_check beta "$GREEN"; lab_check gamma "$GREEN"
OUT5="$(lab_run alpha gamma)"
grep -qE '^PASS[[:space:]]+alpha[[:space:]]' <<< "$OUT5" || { R=1; why "alpha was selected but has no row"; }
grep -qE '^PASS[[:space:]]+gamma[[:space:]]' <<< "$OUT5" || { R=1; why "gamma was selected but has no row"; }
grep -qE '^(PASS|FAIL)[[:space:]]+beta[[:space:]]' <<< "$OUT5" \
  && { R=1; why "beta was not selected but appears in the table"; }

OUT5B="$(lab_run alpha nosuchunit)"
grep -q 'nosuchunit' <<< "$OUT5B" || { R=1; why "an unknown unit name produced no row at all"; }
lab_run alpha nosuchunit > /dev/null 2>&1 && { R=1; why "an unknown unit name exited 0"; }
row N5 "$R"

# ---- N6 -----------------------------------------------------------------------------------------
# The static half. The dynamic half — the whole suite under stubs that exit 127 — is H6, and lives
# with the unit that owns the constraint.
R=0
for prog in curl wget nc ssh; do
  grep -qE "(^|[^-[:alnum:]_])$prog[[:space:]]" "$RUNNER" \
    && { R=1; why "run.sh invokes $prog"; }
done
grep -qE 'git[[:space:]]+(fetch|clone|pull)' "$RUNNER" && { R=1; why "run.sh reaches a remote via git"; }
row N6 "$R"

# ---- N7 -----------------------------------------------------------------------------------------
R=0
grep -qF "$FORBIDDEN_TMP/" "$RUNNER" && { R=1; why "run.sh contains a literal ${FORBIDDEN_TMP} path"; }
grep -qF 'TMPDIR' "$RUNNER" || { R=1; why "run.sh does not derive its scratch path from TMPDIR"; }

# Pointed at an empty directory, it must use it — and leave it empty again.
PROBE="$LAB/probe"; mkdir -p "$PROBE"
lab_reset; lab_units alpha; lab_check alpha "$GREEN"
( cd "$LAB" && TMPDIR="$PROBE" bash ./run.sh ) > /dev/null 2>&1
LEFT="$(find "$PROBE" -mindepth 1 2>/dev/null | grep -c .)"
[ "$LEFT" -eq 0 ] || { R=1; why "$LEFT temporary file(s) left behind in TMPDIR after a green run"; }
lab_check alpha "$RED"
( cd "$LAB" && TMPDIR="$PROBE" bash ./run.sh ) > /dev/null 2>&1
LEFT="$(find "$PROBE" -mindepth 1 2>/dev/null | grep -c .)"
[ "$LEFT" -eq 0 ] || { R=1; why "$LEFT temporary file(s) left behind in TMPDIR after a red run"; }
row N7 "$R"

# ---- N8 -----------------------------------------------------------------------------------------
# The red baseline this suite is measured from, made reproducible. It is not an observation about a
# past run — it is the mechanism that produced it: units declared, no checks present, every row red.
R=0
lab_reset; lab_units alpha beta gamma      # no check scripts written at all
OUT8="$(lab_run)"
STATUS8=$?
[ "$STATUS8" -eq 0 ] && { R=1; why "a decomposition with no checks at all exited 0"; }
for u in alpha beta gamma; do
  grep -qE "^FAIL[[:space:]]+$u[[:space:]]" <<< "$OUT8" \
    || { R=1; why "$u is not a FAIL row when no check exists for it"; }
done
grep -qE '^PASS' <<< "$OUT8" && { R=1; why "a run with no checks present reported a passing row"; }
row N8 "$R"

# ---- N9 -----------------------------------------------------------------------------------------
# N4 covers a check dying. It does not cover the runner failing before it ever gets to one, and the
# specification's invariant has no exception for that: *the runner prints its table on every path*.
# The failure it was found by is ordinary — a `TMPDIR` that cannot be made — and the observed result
# was a bare `mkdir:` line, exit 1, and no table at all. A report that did not print is
# indistinguishable from a suite with nothing to say, which is this runner's own stated reason for
# never using `set -e`.
#
# `TMPDIR` is pointed at a path *under a regular file*, so `mkdir -p` fails with ENOTDIR on any
# filesystem — no `/proc`, no permissions games, nothing this machine could make succeed.
R=0
lab_reset; lab_units alpha beta
lab_check alpha "$GREEN"; lab_check beta "$GREEN"
NOTADIR="$LAB/not-a-directory"
: > "$NOTADIR"
OUT9="$( cd "$LAB" && TMPDIR="$NOTADIR/scratch" bash ./run.sh 2>&1 )"
STATUS9=$?
grep -q 'W4-2 verifiers' <<< "$OUT9" \
  || { R=1; why "no summary table when the scratch base could not be created"; }
for u in alpha beta; do
  grep -qE "^FAIL[[:space:]]+$u[[:space:]]" <<< "$OUT9" \
    || { R=1; why "$u has no FAIL row when the runner could not allocate scratch"; }
done
grep -qE '^PASS' <<< "$OUT9" && { R=1; why "a run with no scratch file reported a passing row"; }
[ "$STATUS9" -eq 0 ] && { R=1; why "the run exited 0 with no scratch file"; }
if [ "$R" -ne 0 ]; then
  why "the runner printed: '$(head -1 <<< "$OUT9")' (exit $STATUS9)"
fi
row N9 "$R"

finish
