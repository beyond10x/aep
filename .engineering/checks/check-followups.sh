#!/usr/bin/env bash
# task:ova-followups — F1 … F9.  (R10.)
#
# The story's third bullet made decidable: *a closed vocabulary with no stated reason does not
# survive the audit unremarked*.
#
# Two things this check does not do, both because R18 forbids them:
#
#   * it does not recompute the settled/unsettled partition — it reads the `PARTITION` lines
#     `check-closed-cells.sh` prints (F8), so the two checks cannot disagree about which rows need a
#     follow-up;
#   * it does not open a planning file. Every fact about an artifact comes from `protocol artifact
#     list` and `protocol artifact graph`. F5 asks that the artifact quote the `Declaration` cell
#     "in its body"; read through the CLI that becomes its **title**, which is the part of the
#     artifact the store will show without anyone opening it. Recorded here rather than routed
#     around by grepping the store.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

STORY="story:open-vocabulary-audit"

declare_row F1 "every unsettled closed row's Follow-up is a story: or architecture-decision-record: id"
declare_row F2 "each such id appears in protocol artifact list --format json"
declare_row F3 "every settled closed row's Follow-up is exactly the em dash"
declare_row F4 "an id that is not in the store is reported by name — F2 discriminates"
declare_row F5 "each named artifact's title carries the Declaration cell of the row that produced it"
declare_row F6 "each named artifact is related to story:open-vocabulary-audit in the store's graph"
declare_row F7 "each named artifact is in its kind's initial status — none was moved"
declare_row F8 "the partition is read from check-closed-cells.sh, not recomputed here"
declare_row F9 "rows examined equals the number of closed rows in the table"

have protocol || { red_all "protocol is not on PATH; R18 allows no other route to the store"; finish; exit; }
audit_present || { red_all "no $AUDIT_REL"; finish; exit; }

# ---- F8 -----------------------------------------------------------------------------------------
# Reported first: everything below is read out of this, so a failure here explains the rest.
SIBLING="$CHECKS_DIR/check-closed-cells.sh"
PARTITION=""
R=0
if [ ! -f "$SIBLING" ]; then
  R=1; why "no check-closed-cells.sh — the partition has no source"
else
  PARTITION="$(bash "$SIBLING" 2>/dev/null | grep '^PARTITION')"
  [ -n "$PARTITION" ] \
    || { R=1; why "check-closed-cells.sh printed no PARTITION line; F8 forbids recomputing it here"; }
fi
# F8's other half: this file must not carry a settled-rule of its own. Matched as a *definition*
# rather than as a word, so the check does not trip over its own grep pattern.
grep -qE '^[a-z_]*settled[a-z_]*\(\)' "${BASH_SOURCE[0]}" \
  && { R=1; why "this check defines its own settled rule instead of reading the sibling's"; }
row F8 "$R"

CLOSED_N="$(rows_with_verdict "$AUDIT" closed | grep -c .)"
EXAMINED=0
F1_R=0; F2_R=0; F3_R=0; F5_R=0; F6_R=0; F7_R=0
IDS=()

if [ -n "$PARTITION" ]; then
  while IFS=$'\t' read -r _ state ln decl; do
    [ -z "$state" ] && continue
    EXAMINED=$((EXAMINED + 1))

    ROW="$(table_rows "$AUDIT" | awk -F'\t' -v n="$ln" '$1 == n { print }')"
    followup="$(cell "$ROW" "$COL_FOLLOWUP")"

    if [ "$state" = "settled" ]; then
      [ "$followup" = "$EMDASH" ] \
        || { F3_R=1; why "settled row '${decl}' ($AUDIT_REL:$ln) has Follow-up '${followup:-<empty>}', not the em dash"; }
      continue
    fi

    # Unsettled. F1: an id, and only the two kinds R10 allows.
    case "$followup" in
      story:*|architecture-decision-record:*)
        IDS+=("$followup")
        ;;
      *)
        F1_R=1
        why "unsettled row '${decl}' ($AUDIT_REL:$ln) has Follow-up '${followup:-<empty>}' — R10 wants a story: or architecture-decision-record: id"
        continue
        ;;
    esac

    if ! artifact_exists "$followup"; then
      F2_R=1; why "$followup (named by '${decl}') is not in the store"
      continue
    fi

    title="$(artifact_field "$followup" title)"
    if [ -z "$decl" ]; then
      F5_R=1; why "$followup's row has an empty Declaration, so nothing can be joined on"
    elif ! grep -Fq "$decl" <<< "$title"; then
      F5_R=1; why "$followup's title does not carry the Declaration '$decl' — the row and the artifact cannot be joined"
    fi

    artifact_relates "$followup" "$STORY" \
      || { F6_R=1; why "$followup carries no relation to $STORY in the store's graph"; }

    kind="$(artifact_field "$followup" kind)"
    status="$(artifact_field "$followup" status)"
    initial="$(kind_initial_status "$kind")"
    if [ -z "$initial" ]; then
      F7_R=1; why "no initial status could be read for kind '$kind'"
    elif [ "$status" != "$initial" ]; then
      F7_R=1; why "$followup is $status; a $kind starts at $initial and the work it names is out of scope here"
    fi
  done <<< "$PARTITION"
else
  F1_R=1; F2_R=1; F3_R=1; F5_R=1; F6_R=1; F7_R=1
  why "no partition to read, so no follow-up could be examined"
fi

row F1 "$F1_R"
row F2 "$F2_R"
row F3 "$F3_R"

# ---- F4 -----------------------------------------------------------------------------------------
# Acceptance criterion 4, reduced to the lookup that decides it: an id nobody created must not
# resolve. A store lookup that answered yes to everything would make F2 decorative.
R=0
artifact_exists "story:ova-probe-not-in-the-store" \
  && { R=1; why "the store lookup resolves an id that was never created"; }
artifact_exists "$STORY" \
  || { R=1; why "the store lookup does not resolve $STORY, which does exist"; }
if [ "${#IDS[@]}" -gt 0 ]; then
  note "${#IDS[@]} follow-up id(s) named: ${IDS[*]}"
fi
row F4 "$R"

row F5 "$F5_R"
row F6 "$F6_R"
row F7 "$F7_R"

# ---- F9 -----------------------------------------------------------------------------------------
R=0
[ "$EXAMINED" -eq "${CLOSED_N:-0}" ] \
  || { R=1; why "examined $EXAMINED row(s) of ${CLOSED_N:-0} closed row(s) in the table"; }
[ "${CLOSED_N:-0}" -ge 1 ] || { R=1; why "no closed rows at all, so F1-F7 assert nothing"; }
row F9 "$R"

finish
