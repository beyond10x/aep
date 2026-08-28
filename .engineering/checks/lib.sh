#!/usr/bin/env bash
# Shared vocabulary for the W4-2 verifiers.
#
# Sourced by every `check-*.sh`. It carries no assertion of its own — what it carries is the
# specification's invariants made mechanical, so no individual check can forget them:
#
#   * **A vacuous check is a failed check.** Every id a check declares must be reported. `finish`
#     fails the check if one was not, so a row that fell out of a branch is a red row and not an
#     absent one.
#   * **The table prints on every path, including failure.** Nothing here sets `-e`, and no helper
#     exits early.
#   * **One parser for the audit's table** (T8). Every sibling reads the table through
#     `table_rows`/`cell` rather than re-implementing the split, so no two checks can disagree about
#     what a row is.
#
# Deliberately *not* `set -e`: an assertion that aborts the script takes the report with it.
set -uo pipefail

CHECKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$CHECKS_DIR/../.." && pwd)"

AUDIT_REL="docs/guide/open-vocabulary.md"
AUDIT="$REPO/$AUDIT_REL"
GUIDE_README_REL="docs/guide/README.md"
GUIDE_README="$REPO/$GUIDE_README_REL"
SCAN="$CHECKS_DIR/scan-declarations.sh"
# The model this suite was written against left the repository on 2026-08-22: the agent-eval
# checks and their recorded transcripts moved to metaharness `evals/engineering-protocols/` with
# `epic:metaharness-migration`, and nothing under `integrations/claude-code/eval/` survives here.
# The variable stays, empty, so H4 reports *the model is in another repository* rather than *the
# model is gone*, which are different facts and only one of them is a defect.
MODEL_RUNNER_REL=""

# The `protocol` a check must use is the one this tree builds, never whatever a shell happens to
# have on PATH. On 2026-08-28 the PATH binary here was **0.28.0** against a 0.31.0 store, and H2
# read five stories as drifted that had not drifted — a stale install cannot be told from a current
# one by looking at it, which is the defect `version-check` exists for one layer up.
protocol_bin() {
  local built
  for built in "$REPO/target/debug/protocol" "$REPO/target/release/protocol"; do
    [ -x "$built" ] && { printf '%s' "$built"; return 0; }
  done
  command -v protocol >/dev/null 2>&1 || return 1
  printf '%s' "$(command -v protocol)"
}

# The workspace version, so a check can say which build answered it.
workspace_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' "$REPO/Cargo.toml" | head -1
}

# The seven columns, in the order the specification fixes. The checks parse by header, so this array
# is the contract between R3 and every column-reading sibling.
COLUMNS=(Declaration "Invited at" Verdict "Decided by" Guarantee "Reason for adopters at" Follow-up)
COL_DECLARATION=1
COL_INVITED=2
COL_VERDICT=3
COL_DECIDED=4
COL_GUARANTEE=5
COL_REASON=6
COL_FOLLOWUP=7

# R8's literal. An em dash, not a hyphen and not an empty cell.
EMDASH="—"

# R1's corpus rule, stated once, as the three globs the audit must name verbatim.
CORPUS_GLOBS=('docs/guide/*.md' 'website/docs/**/*.md' 'docs/plan/document-authoring-brief.md')

# ---- rows ---------------------------------------------------------------------------------------
# A check declares its ids and their statements up front, then reports each one exactly once.

declare -A STATEMENT=()
declare -A REPORTED=()
ROW_IDS=()
FAILED=0

# declare_row <id> <statement>
declare_row() {
  STATEMENT["$1"]="$2"
  ROW_IDS+=("$1")
}

# row <id> <exit-status>   — 0 is a pass, anything else is a failure.
row() {
  local id="$1" code="$2"
  if [ -n "${REPORTED[$id]:-}" ]; then
    printf 'FAIL  %-4s reported twice — the check is confused about its own rows\n' "$id"
    FAILED=$((FAILED + 1))
    return
  fi
  REPORTED["$id"]=1
  if [ "$code" -eq 0 ]; then
    printf 'PASS  %-4s %s\n' "$id" "${STATEMENT[$id]:-<undeclared row>}"
  else
    printf 'FAIL  %-4s %s\n' "$id" "${STATEMENT[$id]:-<undeclared row>}"
    FAILED=$((FAILED + 1))
  fi
}

# why <text…>  — the reason under the row it belongs to. Printed, never counted.
why() { printf '        ↳ %s\n' "$*"; }

# note <text…>  — a fact the reader needs that is not a verdict. Partition counts live here.
note() { printf '        · %s\n' "$*"; }

# red_all <reason>  — every not-yet-reported row goes red for one shared reason.
#
# This is what a missing deliverable looks like. It is emphatically not a skip: the rows are in the
# table, they are red, and the reason is under them.
red_all() {
  local reason="$1" id
  for id in "${ROW_IDS[@]}"; do
    [ -n "${REPORTED[$id]:-}" ] && continue
    row "$id" 1
    why "$reason"
  done
}

# finish  — the check's exit status, and the last enforcement of the no-silent-row rule.
finish() {
  local id missing=0
  for id in "${ROW_IDS[@]}"; do
    if [ -z "${REPORTED[$id]:-}" ]; then
      printf 'FAIL  %-4s never reported — a row that did not run is not a row that passed\n' "$id"
      missing=$((missing + 1))
    fi
  done
  [ "$((FAILED + missing))" -eq 0 ]
}

# ---- preconditions ------------------------------------------------------------------------------

have() { command -v "$1" >/dev/null 2>&1; }
audit_present() { [ -f "$AUDIT" ]; }
scan_present() { [ -f "$SCAN" ]; }

# ---- scratch ------------------------------------------------------------------------------------
# The forbidden base is assembled rather than written, because `check-surface-hygiene.sh` greps every
# file in this directory for it and a grep pattern written literally would match its own source.

FORBIDDEN_TMP="/$(printf 'tmp')"

scratch() {
  local base="${TMPDIR:-$HOME/.cache/claude-tmp}"
  mkdir -p "$base" || return 1
  mktemp -d "$base/ova-check.XXXXXX"
}

under_allowed_base() {
  local path="$1" base="${TMPDIR:-}" fallback="$HOME/.cache/claude-tmp"
  case "$path" in "$FORBIDDEN_TMP"/*) return 1 ;; esac
  [ -n "$base" ] && case "$path" in "$base"/*) return 0 ;; esac
  case "$path" in "$fallback"/*) return 0 ;; esac
  return 1
}

# ---- the corpus (R1) ----------------------------------------------------------------------------
# Re-derived from the globs on every call. Never read from the audit's own list — a check that
# trusted the list could not detect the list being wrong, which is the whole of R1.

corpus_paths() {
  (
    cd "$REPO" || return 1
    shopt -s nullglob
    for f in docs/guide/*.md; do printf '%s\n' "$f"; done
    find website/docs -type f -name '*.md' 2>/dev/null
    [ -f docs/plan/document-authoring-brief.md ] && printf '%s\n' docs/plan/document-authoring-brief.md
  ) | sort -u
}

in_corpus() {
  local want="$1"
  corpus_paths | grep -Fxq "$want"
}

# ---- the audit's table (R3, T8) -----------------------------------------------------------------
# One parser, used by every sibling. A "table" is a maximal run of consecutive lines beginning with
# `|`; `table_lines` numbers those runs so T1 can count them and every other check can insist on
# reading only the first.

# table_lines <file>  ->  block<TAB>lineno<TAB>content
table_lines() {
  awk '
    /^[[:space:]]*\|/ { if (!inb) { inb = 1; b++ } ; printf "%d\t%d\t%s\n", b, NR, $0; next }
    { inb = 0 }
  ' "$1" 2>/dev/null
}

table_block_count() { table_lines "$1" | cut -f1 | sort -un | grep -c . ; }

# table_block_starts <file>  -> the first line number of each table found
table_block_starts() { table_lines "$1" | awk -F'\t' '!seen[$1]++ { print $2 }'; }

_split_cells='
function cells(s,   arr, i, n, out) {
  sub(/^[[:space:]]*\|/, "", s)
  sub(/\|[[:space:]]*$/, "", s)
  n = split(s, arr, /\|/)
  for (i = 1; i <= n; i++) {
    gsub(/^[[:space:]]+/, "", arr[i]); gsub(/[[:space:]]+$/, "", arr[i])
    out = out (i > 1 ? "\t" : "") arr[i]
  }
  return out
}
function is_sep(s) { return s ~ /^[[:space:]]*\|[[:space:]:|-]+\|[[:space:]]*$/ }
'

# table_header <file>  ->  the first block's header cells, TAB separated
table_header() {
  table_lines "$1" | awk -F'\t' "$_split_cells"'
    $1 != 1 { next }
    { n++ }
    n == 1 { print cells($3); exit }
  '
}

# table_rows <file>  ->  lineno<TAB>cell1<TAB>…<TAB>cellN, header and separator dropped
table_rows() {
  table_lines "$1" | awk -F'\t' "$_split_cells"'
    $1 != 1 { next }
    { n++ }
    n == 1 { next }
    is_sep($3) { next }
    { print $2 "\t" cells($3) }
  '
}

table_row_count() { table_rows "$1" | grep -c . ; }

# cell <row> <n>  — the nth data cell of a `table_rows` line. Field 1 is the line number, so the
# nth column is field n+1. Prints nothing when the row is too short, which every caller treats as
# a failure rather than as an empty cell.
cell() {
  printf '%s' "$1" | awk -F'\t' -v n="$(( $2 + 1 ))" 'NF >= n { printf "%s", $n }'
}

row_lineno() { printf '%s' "$1" | cut -f1; }
row_width() { printf '%s' "$1" | awk -F'\t' '{ print NF - 1 }'; }

# rows_with_verdict <file> <open|closed>
rows_with_verdict() {
  table_rows "$1" | awk -F'\t' -v want="$2" -v c="$(( COL_VERDICT + 1 ))" '
    NF >= c && $c == want { print }
  '
}

# ---- the audit's sections -----------------------------------------------------------------------
# A section is its heading line plus everything up to the next heading of the same or shallower
# depth. Several checks want "the part of the audit under the heading that says X" and none of them
# should re-derive what that means.

# section_by_heading <file> <case-insensitive extended regex matched against the heading text>
section_by_heading() {
  awk -v want="$2" '
    {
      if ($0 ~ /^#+[[:space:]]/) {
        n = 0
        while (substr($0, n + 1, 1) == "#") n++
        text = tolower(substr($0, n + 1))
        if (inside && n <= level) inside = 0
        if (!inside && text ~ want) { inside = 1; level = n; print; next }
      }
      if (inside) print
    }
  ' "$1" 2>/dev/null
}

has_section() { [ -n "$(section_by_heading "$1" "$2")" ]; }

# ---- the store (R18) ----------------------------------------------------------------------------
# The **only** permitted route to planning state. No check reads an artifact file: a check that
# grepped a planning body would assert that a sentence is still written there, which is the failure
# mode this audit exists to remove.

artifact_ids() {
  protocol artifact list --format json 2>/dev/null \
    | sed -n 's/^[[:space:]]*"id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p'
}

artifact_exists() { artifact_ids | grep -Fxq "$1"; }

# artifact_field <id> <field>  — id, kind, status or title, out of the same JSON. A hand-rolled
# reader and not `jq`, because the map declares three programs on PATH and `jq` is not one of them.
artifact_field() {
  protocol artifact list --format json 2>/dev/null | awk -v want="$1" -v field="$2" '
    /"id"[[:space:]]*:/ {
      id = $0; sub(/.*"id"[[:space:]]*:[[:space:]]*"/, "", id); sub(/".*/, "", id)
      here = (id == want)
    }
    here {
      line = $0
      if (line ~ ("\"" field "\"[[:space:]]*:")) {
        sub(".*\"" field "\"[[:space:]]*:[[:space:]]*\"", "", line)
        sub(/".*/, "", line)
        print line
        exit
      }
    }
  '
}

# artifact_relates <from-id> <to-id>  — is there an edge, in either direction, in the store's graph.
# `graph` and not the file, because R18 allows the CLI and forbids the body.
#
# Each stage lands in a variable rather than feeding the next through a pipe. Same predicate, but a
# `grep -q` closes its input the instant it matches, and under `pipefail` the SIGPIPE it sends
# upstream becomes the pipeline's exit status — so the piped form answered 141 for every pair that
# *does* relate, which is the one answer it must never give.
artifact_relates() {
  local edges from
  edges="$(protocol artifact graph 2>/dev/null | grep -- '->')"
  grep -Fq "\"$1\"" <<< "$edges" || return 1
  from="$(grep -F "\"$1\"" <<< "$edges")"
  grep -Fq "\"$2\"" <<< "$from"
}

# kind_initial_status <kind>  — `protocol artifact lifecycle <kind>` opens with "<kind> starts at X".
kind_initial_status() {
  protocol artifact lifecycle "$1" 2>/dev/null | sed -n 's/^.* starts at \([a-z_]*\).*/\1/p' | head -1
}

# ---- running the suite from inside a check ------------------------------------------------------
# Three units need to run the suite itself: `surface-hygiene` (H6, under stubs), `repeatability`
# (Y7, twice), and `mutation-proof` (M2-M8, on a copy). All three must therefore run it *without*
# themselves, or the suite has no fixed point. `checks-runner` stays in — it only ever runs a
# synthetic decomposition in a scratch directory, never this one.
#
# This is a bound on coverage, so it is printed rather than assumed: every caller `note`s what it
# left out. A silent exclusion reads as "the whole suite passed" when it did not.
INNER_UNITS=(
  checks-runner
  scan-declarations
  audit-corpus
  table-shape
  open-cells
  closed-cells
  followups
  citations
  layered-rows
  scan-loop
)
INNER_EXCLUDED="surface-hygiene, repeatability, mutation-proof"

# ---- the tree -----------------------------------------------------------------------------------

# The file's line count, `0` when it does not exist. `awk` and not `grep -c`, because `grep -c` on an
# empty file prints 0 *and* exits 1, and a `||` fallback behind it prints the zero twice.
file_lines() { awk 'END { print NR + 0 }' "$1" 2>/dev/null || printf '0\n'; }
git_status() { git -C "$REPO" status --porcelain; }
