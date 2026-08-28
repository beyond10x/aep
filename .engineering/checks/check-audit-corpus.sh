#!/usr/bin/env bash
# task:ova-audit-corpus — C1 … C9.  (R0–R2.)
#
# The unit every other check stands on. Two contracts it fixes for the audit's author:
#
#   * the corpus lives under a heading whose text contains **corpus**;
#   * each corpus path is its own line, as a markdown list item whose first backticked token is the
#     repository-relative path — so a list a check can compare, not a sentence it must parse.
#
# The comparison always re-derives the set from the globs. A check that read the audit's list and
# then checked the audit's list against itself would be green on a corpus that is entirely wrong.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

declare_row C1 "docs/guide/open-vocabulary.md exists and is non-empty"
declare_row C2 "the guide's Which guide table gains exactly one row linking to open-vocabulary.md"
declare_row C3 "the audit carries a corpus section listing paths, one per line"
declare_row C4 "that list equals the set the three globs produce, both directions, named individually"
declare_row C5 "every listed path exists"
declare_row C6 "the audit prints the corpus file count, and it equals both the list and the globs"
declare_row C7 "the audit names the three globs verbatim, so the check re-derives rather than trusts"
declare_row C8 "an unlisted file matching a glob is reported by name — the comparator discriminates"
declare_row C9 "this unit's writes stay inside docs/: only the audit and the guide README changed"

DERIVED="$(corpus_paths)"
DERIVED_N="$(printf '%s\n' "$DERIVED" | grep -c .)"

# ---- C1 -----------------------------------------------------------------------------------------
R=0
if ! audit_present; then
  R=1; why "no $AUDIT_REL"
elif [ ! -s "$AUDIT" ]; then
  R=1; why "$AUDIT_REL exists but is empty"
fi
row C1 "$R"

# ---- C2 -----------------------------------------------------------------------------------------
# The routing half of R0. Exactly one row: a guide listed twice is a table nobody trusts.
R=0
if [ ! -f "$GUIDE_README" ]; then
  R=1; why "no $GUIDE_README_REL"
else
  WHICH="$(section_by_heading "$GUIDE_README" 'which guide')"
  if [ -z "$WHICH" ]; then
    R=1; why "$GUIDE_README_REL has no 'Which guide' section"
  else
    HITS="$(grep -cF '(open-vocabulary.md)' <<< "$WHICH")"
    case "$HITS" in
      0) R=1; why "the Which guide table has no row linking to open-vocabulary.md" ;;
      1) : ;;
      *) R=1; why "the Which guide table links to open-vocabulary.md $HITS times; R0 wants one row" ;;
    esac
    [ -f "$REPO/docs/guide/open-vocabulary.md" ] \
      || { R=1; why "the link target docs/guide/open-vocabulary.md does not resolve"; }
  fi
fi
row C2 "$R"

# ---- C3 -----------------------------------------------------------------------------------------
R=0
LISTED=""
if ! audit_present; then
  R=1; why "no audit to read a corpus from"
else
  CORPUS_SECTION="$(section_by_heading "$AUDIT" 'corpus')"
  if [ -z "$CORPUS_SECTION" ]; then
    R=1; why "the audit has no section whose heading names the corpus"
  else
    # One path per line: a list item whose first backticked token is a path.
    LISTED="$(grep -oE '^[[:space:]]*[-*][[:space:]]+`[^`]+`' <<< "$CORPUS_SECTION" \
      | sed 's/.*`\(.*\)`/\1/' | sort -u)"
    LISTED_N="$(printf '%s\n' "$LISTED" | grep -c .)"
    [ "${LISTED_N:-0}" -ge 1 ] \
      || { R=1; why "the corpus section lists no paths as one-per-line list items"; }
  fi
fi
row C3 "$R"

LISTED_N="$(printf '%s\n' "$LISTED" | grep -c .)"

# ---- C4 -----------------------------------------------------------------------------------------
# Set equality in both directions, each difference named. R1's whole point: a guide added later
# makes the audit red rather than silently out of date.
R=0
if [ -z "$LISTED" ]; then
  R=1; why "no corpus list to compare against the ${DERIVED_N} path(s) the globs produce"
else
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    grep -Fxq "$p" <<< "$LISTED" || { R=1; why "the globs produce $p and the audit does not list it"; }
  done <<< "$DERIVED"
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    grep -Fxq "$p" <<< "$DERIVED" || { R=1; why "the audit lists $p and no glob produces it"; }
  done <<< "$LISTED"
fi
row C4 "$R"

# ---- C5 -----------------------------------------------------------------------------------------
# R2: a corpus entry that does not resolve is a failed check, not a stale line.
R=0
if [ -z "$LISTED" ]; then
  R=1; why "no corpus list to resolve"
else
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    [ -f "$REPO/$p" ] || { R=1; why "$p is listed as corpus and does not exist"; }
  done <<< "$LISTED"
fi
row C5 "$R"

# ---- C6 -----------------------------------------------------------------------------------------
# Three numbers that must agree. The stated one is the one that rots.
R=0
if ! audit_present; then
  R=1; why "no audit to read a count from"
else
  STATED="$(section_by_heading "$AUDIT" 'corpus' \
    | grep -oE '[0-9]+[[:space:]]+files?' | head -1 | grep -oE '[0-9]+')"
  if [ -z "$STATED" ]; then
    R=1; why "the corpus section states no file count (expected a phrase like '33 files')"
  else
    [ "$STATED" -eq "${LISTED_N:-0}" ] \
      || { R=1; why "the audit states $STATED file(s) and lists ${LISTED_N:-0}"; }
    [ "$STATED" -eq "${DERIVED_N:-0}" ] \
      || { R=1; why "the audit states $STATED file(s) and the globs produce ${DERIVED_N:-0}"; }
  fi
fi
row C6 "$R"

# ---- C7 -----------------------------------------------------------------------------------------
R=0
if ! audit_present; then
  R=1; why "no audit to read the globs from"
else
  for g in "${CORPUS_GLOBS[@]}"; do
    grep -qF "$g" "$AUDIT" || { R=1; why "the audit does not name the glob $g verbatim"; }
  done
fi
row C7 "$R"

# ---- C8 -----------------------------------------------------------------------------------------
# The comparator, shown to discriminate — against a synthetic derived set rather than by writing a
# file into docs/guide/. Creating a real file to prove a point would leave the tree dirty for
# check-surface-hygiene.sh, and a check that reddens a sibling is not evidence.
R=0
if [ -z "$LISTED" ]; then
  R=1; why "no corpus list, so the comparator has nothing to discriminate on"
else
  PROBE='docs/guide/ova-probe-unlisted.md'
  FOUND=0
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    if ! grep -Fxq "$p" <<< "$LISTED"; then
      [ "$p" = "$PROBE" ] && FOUND=1
    fi
  done < <(printf '%s\n%s\n' "$DERIVED" "$PROBE")
  [ "$FOUND" -eq 1 ] \
    || { R=1; why "an unlisted glob member was not detected — C4's comparison is not discriminating"; }
fi
row C8 "$R"

# ---- C9 -----------------------------------------------------------------------------------------
# This unit's declared write surface. The whole-tree version is H3; this one is narrower: the audit
# task touches three files under docs/ and nothing else under docs/.
#
# **It was two.** Settling a closed row needs a reason an adopter reads, and R9 refuses the audit's
# own cell as that place (K10) — so a round that settles one has to write prose into a corpus file
# that is neither this page nor the README that routes to it. Inside H3's surface the only corpus
# files are `docs/guide/*.md` and the authoring brief, and `adopting.md` is the page that already
# tells an adopter what they may declare. The surface grew by exactly the one file that made the
# relation and predicate-operator rows settleable; every other path under docs/ still reddens this
# row.
R=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  path="${line:3}"
  case "$path" in
    docs/guide/open-vocabulary.md|docs/guide/README.md|docs/guide/adopting.md) ;;
    *) R=1; why "this unit changed $path, which is not one of its two declared files" ;;
  esac
done < <(git -C "$REPO" status --porcelain -- docs/)
# …and the floor. A unit that wrote nothing has not stayed inside its surface, it has stayed outside
# the work: with no audit on disk this row would be green for having nothing to be wrong about.
audit_present || { R=1; why "no $AUDIT_REL, so this unit has written nothing to stay inside docs/"; }
row C9 "$R"

finish
