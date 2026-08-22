#!/usr/bin/env bash
# task:ova-repeatability — Y1 … Y8.  (R14, R15.)
#
# What makes the next round a diff rather than a rewrite. The section under audit here is the one a
# future reader follows to reproduce the round: the corpus rule, the scan command, the commit it was
# taken at, and the reading pass that produced the rows the scan could never find.
#
# Y8 is the row that matters and the one nobody would notice: a method section that describes a step
# the suite does not perform is a promise, and a promise in a document nobody re-runs is exactly the
# failure the story was opened for.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

SCAN_REL="${SCAN#"$REPO"/}"

declare_row Y1 "the audit carries a section stating how it was produced, found by its heading"
declare_row Y2 "that section names the scan script by path, and the path exists"
declare_row Y3 "the command the section prints runs verbatim from the repository root and exits 0"
declare_row Y4 "it states the commit the round was taken at, and git cat-file -e resolves it"
declare_row Y5 "the corpus rule it states is the same three globs the corpus check re-derives from"
declare_row Y6 "the reading-pass row count it states equals the reading-backed count scan-loop prints"
declare_row Y7 "two consecutive suite runs leave the working tree identical — the audit is re-runnable"
declare_row Y8 "it describes no step the suite does not perform"

audit_present || { red_all "no $AUDIT_REL"; finish; exit; }

METHOD="$(section_by_heading "$AUDIT" 'produced|method|how it was made|reproduc')"

# ---- Y1 -----------------------------------------------------------------------------------------
R=0
[ -n "$METHOD" ] \
  || { R=1; why "the audit has no section whose heading says how the round was produced"; }
row Y1 "$R"

# ---- Y2 -----------------------------------------------------------------------------------------
R=0
if [ -z "$METHOD" ]; then
  R=1; why "no method section to read a path from"
else
  grep -qF "$SCAN_REL" <<< "$METHOD" || { R=1; why "the section does not name $SCAN_REL"; }
  [ -f "$SCAN" ] || { R=1; why "$SCAN_REL is named and does not exist"; }
fi
row Y2 "$R"

# ---- Y3 -----------------------------------------------------------------------------------------
# Run verbatim, because a command nobody has run is documentation of an intention.
R=0
CMD="$(grep -oE "bash[[:space:]]+$SCAN_REL[^\`]*" <<< "$METHOD" | head -1 | sed 's/[[:space:]]*$//')"
if [ -z "$CMD" ]; then
  R=1; why "the section prints no runnable \`bash $SCAN_REL\` invocation"
elif [ ! -f "$SCAN" ]; then
  R=1; why "$SCAN_REL does not exist, so the printed command cannot be run"
else
  ( cd "$REPO" && eval "$CMD" ) > /dev/null 2>&1 \
    || { R=1; why "the command the section prints exited non-zero: $CMD"; }
  note "ran verbatim: $CMD"
fi
row Y3 "$R"

# ---- Y4 -----------------------------------------------------------------------------------------
R=0
if [ -z "$METHOD" ]; then
  R=1; why "no method section to read a commit from"
else
  SHA="$(grep -oE '\b[0-9a-f]{7,40}\b' <<< "$METHOD" | head -1)"
  if [ -z "$SHA" ]; then
    R=1; why "the section states no commit the round was taken at"
  elif ! git -C "$REPO" cat-file -e "$SHA" 2>/dev/null; then
    R=1; why "the section names commit $SHA, which this repository does not have"
  else
    note "round taken at $SHA ($(git -C "$REPO" log -1 --format=%s "$SHA" 2>/dev/null))"
  fi
fi
row Y4 "$R"

# ---- Y5 -----------------------------------------------------------------------------------------
# Compared, not eyeballed: the globs in the section are the globs `lib.sh` derives the corpus from,
# which is what stops the method describing a different round than the one the checks decide.
R=0
if [ -z "$METHOD" ]; then
  R=1; why "no method section to read the corpus rule from"
else
  for g in "${CORPUS_GLOBS[@]}"; do
    grep -qF "$g" <<< "$METHOD" || { R=1; why "the method section does not state the glob $g"; }
  done
fi
row Y5 "$R"

# ---- Y6 -----------------------------------------------------------------------------------------
# The number in the prose against the number a check computes. A stated count is the one part of a
# method section that is silently wrong the moment the table grows.
R=0
SIBLING="$CHECKS_DIR/check-scan-loop.sh"
if [ -z "$METHOD" ]; then
  R=1; why "no method section to read a reading-pass count from"
elif [ ! -f "$SIBLING" ]; then
  R=1; why "no check-scan-loop.sh, so the reading-backed count has no source"
else
  ACTUAL="$(bash "$SIBLING" 2>/dev/null | awk -F'\t' '$1 == "PARTITION" && $2 == "reading-backed" { print $3 }')"
  STATED="$(grep -oiE '[0-9]+[[:space:]]+(rows?|reading-backed)' <<< "$METHOD" | head -1 | grep -oE '[0-9]+')"
  if [ -z "$ACTUAL" ]; then
    R=1; why "check-scan-loop.sh printed no reading-backed partition line"
  elif [ -z "$STATED" ]; then
    R=1; why "the method section states no count for the reading pass"
  elif [ "$STATED" -ne "$ACTUAL" ]; then
    R=1; why "the section states $STATED reading-pass row(s); scan-loop counts $ACTUAL"
  else
    note "reading pass: $ACTUAL row(s), stated and computed agree"
  fi
fi
row Y6 "$R"

# ---- Y7 -----------------------------------------------------------------------------------------
# R15: nothing in the audit requires rewriting to run it again. Shown by running the suite twice and
# comparing the working tree — a check that wrote its own answer into the tree would show up here.
R=0
RUNNER="$CHECKS_DIR/run.sh"
if [ ! -f "$RUNNER" ]; then
  R=1; why "no run.sh to run twice"
else
  note "inner runs cover ${#INNER_UNITS[@]} unit(s); excluded: $INNER_EXCLUDED (they run the suite themselves)"
  BEFORE="$(git_status)"
  ( cd "$REPO" && bash "$RUNNER" "${INNER_UNITS[@]}" ) > /dev/null 2>&1
  FIRST=$?
  MIDDLE="$(git_status)"
  ( cd "$REPO" && bash "$RUNNER" "${INNER_UNITS[@]}" ) > /dev/null 2>&1
  SECOND=$?
  AFTER="$(git_status)"
  [ "$BEFORE" = "$MIDDLE" ] || { R=1; why "the first run changed the working tree"; }
  [ "$MIDDLE" = "$AFTER" ] || { R=1; why "the second run changed the working tree"; }
  [ "$FIRST" -eq 0 ] || { R=1; why "the first run exited $FIRST"; }
  [ "$SECOND" -eq 0 ] || { R=1; why "the second run exited $SECOND"; }
fi
row Y7 "$R"

# ---- Y8 -----------------------------------------------------------------------------------------
# Every command the section prints must be one a check actually runs. The direction matters: a
# method may say less than the suite does, never more.
R=0
if [ -z "$METHOD" ]; then
  R=1; why "no method section, so nothing could be compared against the suite"
else
  SUITE_TEXT="$(cat "$CHECKS_DIR"/run.sh "$CHECKS_DIR"/lib.sh "$CHECKS_DIR"/check-*.sh 2>/dev/null)"
  MENTIONED=0
  while IFS= read -r cmdline; do
    [ -z "$cmdline" ] && continue
    MENTIONED=$((MENTIONED + 1))
    verb="$(printf '%s' "$cmdline" | awk '{ print $1 (NF > 1 ? " " $2 : "") }')"
    case "$verb" in
      "bash $SCAN_REL"|"bash .engineering/checks/run.sh") continue ;;
    esac
    head="$(printf '%s' "$cmdline" | awk '{ print $1 }')"
    grep -qF "$head" <<< "$SUITE_TEXT" \
      || { R=1; why "the method names \`$cmdline\`, which no check in this suite runs"; }
  done < <(grep -oE '`[a-z][a-z0-9_-]+ [^`]*`' <<< "$METHOD" | tr -d '`')
  [ "$MENTIONED" -ge 1 ] \
    || { R=1; why "the method section names no command at all — there is nothing to reproduce"; }
fi
row Y8 "$R"

finish
