#!/usr/bin/env bash
# task:ova-closed-cells — K1 … K8.  (R9.)
#
# A closed verdict is not a defect. This check never fails a row for being closed; what it decides is
# whether the closure says what it buys and where an adopter reads why.
#
# It also **publishes the partition**, because `check-followups.sh` must not recompute it (F8). One
# line per closed row, on stdout, tab separated:
#
#   PARTITION<TAB>settled|unsettled<TAB><line number><TAB><Declaration>
#
# `run.sh` counts only `PASS `/`FAIL ` rows, so these lines are data for a sibling and noise to
# nobody. Two checks that each decided "settled" for themselves could disagree, and the row that
# needs a follow-up would be the one they disagreed about.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

declare_row K1 "every closed row's Guarantee is the literal none or a non-empty statement"
declare_row K2 "every Reason for adopters at is none, or a corpus path (with optional anchor) that exists"
declare_row K3 "a Reason for adopters at that exists but is outside the corpus is red"
declare_row K4 "every closed row is printed as settled or unsettled, with a count of each"
declare_row K5 "a guarantee of none flips a settled row to unsettled — the partition rule discriminates"
declare_row K6 "at least one closed row exists, so K1-K4 are not vacuous"
declare_row K7 "rows partitioned equals the number of closed verdicts the table carries"
declare_row K8 "no row here fails for being closed: an all-settled table passes"
declare_row K9 "an anchor in a Reason for adopters at resolves to a heading in the file it names"
declare_row K10 "no closed row points its reason at this audit — a cell is not somewhere else"

# ---- what the adversarial pass found here -------------------------------------------------------
#
# R9 asks for the reason to be *somewhere an adopter reads*, "not only in the audit's own cell", and
# the check below resolved only the file half of the path. Two mutations went through green:
#
#   the heading an anchor names, renamed   the link lands at the top of the page, reason unfound
#   the reason repointed at this audit     every unsettled row launders itself settled, no follow-up
#
# The second is the story's third acceptance bullet defeated in one cell edit: `docs/guide/*.md` is
# corpus, this audit is a `docs/guide/*.md`, so the audit citing itself resolved. `reason_resolves`
# now refuses it and K10 reports it, because a partition that can be talked into `settled` decides
# which closures escape a follow-up.

# R9's rule, defined once and reported on by K5. A row is settled when the guarantee is stated **and**
# the reason resolves to a corpus file that is not this document.
reason_resolves() {
  local value="${1%%#*}"
  [ -n "$value" ] || return 1
  [ "$value" = "none" ] && return 1
  [ "$value" = "$AUDIT_REL" ] && return 1
  in_corpus "$value" && [ -f "$REPO/$value" ]
}

# anchor_resolves <file> <anchor>  — the GitHub slug rule, applied to every heading in the file:
# lower-cased, punctuation dropped, spaces to hyphens. A link nobody can follow is not a citation.
anchor_resolves() {
  awk -v want="$2" '
    /^#+[[:space:]]/ {
      h = $0
      sub(/^#+[[:space:]]*/, "", h)
      s = tolower(h)
      gsub(/[^a-z0-9 _-]/, "", s)
      gsub(/[ _]+/, "-", s)
      if (s == want) found = 1
    }
    END { exit(found ? 0 : 1) }
  ' "$1" 2>/dev/null
}

is_settled() { # <guarantee> <reason>
  [ -n "$1" ] && [ "$1" != "none" ] && [ "$1" != "$EMDASH" ] && reason_resolves "$2"
}

audit_present || { red_all "no $AUDIT_REL"; finish; exit; }

CLOSED_ROWS="$(rows_with_verdict "$AUDIT" closed)"
CLOSED_N="$(printf '%s\n' "$CLOSED_ROWS" | grep -c .)"

K1_R=0; K2_R=0; K3_R=0; K9_R=0; K10_R=0
ANCHORS_SEEN=0
PARTITIONED=0
SETTLED=0
UNSETTLED=0

if [ "${CLOSED_N:-0}" -gt 0 ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    ln="$(row_lineno "$line")"
    decl="$(cell "$line" "$COL_DECLARATION")"
    guarantee="$(cell "$line" "$COL_GUARANTEE")"
    reason="$(cell "$line" "$COL_REASON")"

    # K1 — a statement or the literal `none`. Blank and the em dash are both refusals to answer.
    case "$guarantee" in
      "" | "$EMDASH")
        K1_R=1; why "closed row '${decl:-<no declaration>}' ($AUDIT_REL:$ln) has an empty Guarantee" ;;
      *[!\ ]*) : ;;
      *)
        K1_R=1; why "closed row '${decl:-<no declaration>}' ($AUDIT_REL:$ln) has a whitespace Guarantee" ;;
    esac

    # K2/K3 — `none`, or a corpus path that resolves. The anchor is stripped before resolving.
    if [ "$reason" != "none" ]; then
      base="${reason%%#*}"
      if [ -z "$base" ] || [ "$base" = "$EMDASH" ]; then
        K2_R=1; why "closed row '${decl:-<no declaration>}' has Reason for adopters at '${reason:-<empty>}'"
      elif ! in_corpus "$base"; then
        K3_R=1
        if [ -f "$REPO/$base" ]; then
          why "'${decl:-<no declaration>}' points its reason at $base, which exists but is not corpus"
        else
          why "'${decl:-<no declaration>}' points its reason at $base, which is not in the corpus"
        fi
      elif [ ! -f "$REPO/$base" ]; then
        K2_R=1; why "'${decl:-<no declaration>}' points its reason at $base, which does not exist"
      fi

      # K10 — the audit is corpus, so without this the cheapest way to settle every unsettled row is
      # to cite this page. R9's "not only in the audit's own cell" is the sentence being enforced.
      if [ "$base" = "$AUDIT_REL" ]; then
        K10_R=1
        why "closed row '${decl:-<no declaration>}' points its reason at $AUDIT_REL — the audit's own cell is not a reason written for adopters"
      fi

      # K9 — the anchor half of R9's "(optionally with an anchor)". A heading rename leaves the file
      # resolving and the link landing at the top of the page, which is where the reason is not.
      case "$reason" in
        *\#*)
          anchor="${reason#*#}"
          if [ -n "$anchor" ] && [ -f "$REPO/$base" ]; then
            ANCHORS_SEEN=$((ANCHORS_SEEN + 1))
            anchor_resolves "$REPO/$base" "$anchor" || {
              K9_R=1
              why "closed row '${decl:-<no declaration>}' points its reason at $base#$anchor; no heading there slugs to '$anchor'"
            }
          elif [ -z "$anchor" ]; then
            K9_R=1; why "closed row '${decl:-<no declaration>}' has an empty anchor in '$reason'"
          fi
          ;;
      esac
    fi

    if is_settled "$guarantee" "$reason"; then
      printf 'PARTITION\tsettled\t%s\t%s\n' "$ln" "$decl"
      SETTLED=$((SETTLED + 1))
    else
      printf 'PARTITION\tunsettled\t%s\t%s\n' "$ln" "$decl"
      UNSETTLED=$((UNSETTLED + 1))
    fi
    PARTITIONED=$((PARTITIONED + 1))
  done <<< "$CLOSED_ROWS"
fi

row K1 "$K1_R"
row K2 "$K2_R"
row K3 "$K3_R"

# ---- K4 -----------------------------------------------------------------------------------------
R=0
note "$SETTLED settled, $UNSETTLED unsettled, of ${CLOSED_N:-0} closed row(s)"
[ "$PARTITIONED" -eq "${CLOSED_N:-0}" ] \
  || { R=1; why "printed $PARTITIONED partition line(s) for ${CLOSED_N:-0} closed row(s)"; }
row K4 "$R"

# ---- K5 -----------------------------------------------------------------------------------------
# The rule, shown to discriminate, without editing the audit: a settled row's own cells put back
# through `is_settled` with the guarantee downgraded to `none` must come out unsettled. That is
# acceptance criterion 3, reduced to the predicate that decides it.
R=0
SAMPLE=""
if [ "${CLOSED_N:-0}" -gt 0 ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    g="$(cell "$line" "$COL_GUARANTEE")"; s="$(cell "$line" "$COL_REASON")"
    if is_settled "$g" "$s"; then SAMPLE="$line"; break; fi
  done <<< "$CLOSED_ROWS"
fi
if [ -z "$SAMPLE" ]; then
  R=1; why "no settled closed row to downgrade — K5 has nothing to discriminate on"
else
  s="$(cell "$SAMPLE" "$COL_REASON")"
  is_settled "none" "$s" && { R=1; why "a Guarantee of 'none' still reads as settled"; }
  g="$(cell "$SAMPLE" "$COL_GUARANTEE")"
  is_settled "$g" "none" && { R=1; why "a Reason for adopters at of 'none' still reads as settled"; }
  is_settled "$g" "docs/guide/no-such-file.md" \
    && { R=1; why "a Reason for adopters at that does not resolve still reads as settled"; }
fi
row K5 "$R"

# ---- K6 -----------------------------------------------------------------------------------------
R=0
[ "${CLOSED_N:-0}" -ge 1 ] || { R=1; why "the table carries no closed row, so K1-K4 assert nothing"; }
row K6 "$R"

# ---- K7 -----------------------------------------------------------------------------------------
R=0
[ "$PARTITIONED" -eq "${CLOSED_N:-0}" ] \
  || { R=1; why "$PARTITIONED row(s) partitioned of ${CLOSED_N:-0} closed verdict(s)"; }
row K7 "$R"

# ---- K8 -----------------------------------------------------------------------------------------
# The invariant, stated as an assertion about this check's own behaviour: a table whose closed rows
# are all settled produces no red row here. Run against the audit's own settled rows.
R=0
BAD=0
if [ "${CLOSED_N:-0}" -gt 0 ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    g="$(cell "$line" "$COL_GUARANTEE")"; s="$(cell "$line" "$COL_REASON")"
    is_settled "$g" "$s" || continue
    # A settled row must contribute nothing to K1, K2 or K3.
    { [ -n "$g" ] && [ "$g" != "$EMDASH" ] && reason_resolves "$s"; } \
      || { BAD=$((BAD + 1)); why "a row counted settled would still redden K1-K3: $(cell "$line" "$COL_DECLARATION")"; }
  done <<< "$CLOSED_ROWS"
fi
[ "$BAD" -eq 0 ] || R=1
[ "${SETTLED:-0}" -ge 1 ] || { R=1; why "no settled closed row exists, so 'closed is not failing' is untested"; }
row K8 "$R"

# ---- K9 -----------------------------------------------------------------------------------------
# The floor, and the predicate shown to discriminate. A slug rule that answered yes to everything
# would make the loop above decoration, so it is asked about a heading that is not there.
note "$ANCHORS_SEEN anchored reason(s) resolved against their file's headings"
[ "$ANCHORS_SEEN" -ge 1 ] \
  || { K9_R=1; why "no closed row carries an anchor, so K9 asserted nothing"; }
if [ -f "$AUDIT" ]; then
  anchor_resolves "$AUDIT" "ova-no-such-heading-anywhere" \
    && { K9_R=1; why "the slug rule resolves an anchor no heading produces"; }
  anchor_resolves "$AUDIT" "the-table" \
    || { K9_R=1; why "the slug rule does not resolve '## The table', a heading this audit has"; }
fi
row K9 "$K9_R"

# ---- K10 ----------------------------------------------------------------------------------------
# …and the partition rule that backs it: this audit must not resolve as somewhere an adopter reads
# the reason, or the settled/unsettled split can be argued into existence one cell at a time.
reason_resolves "$AUDIT_REL" \
  && { K10_R=1; why "the settled rule accepts $AUDIT_REL as the place the reason is written"; }
reason_resolves "$AUDIT_REL#the-table" \
  && { K10_R=1; why "the settled rule accepts $AUDIT_REL with an anchor on it"; }
row K10 "$K10_R"

finish
