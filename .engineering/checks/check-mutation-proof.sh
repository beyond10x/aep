#!/usr/bin/env bash
# task:ova-mutation-proof — M1 … M14.  (Acceptance criteria 2 through 5, and five more.)
#
# The unit that decides whether the rest of the suite discriminates. Everything else here says *the
# audit is consistent today*; this one says *and it would have noticed if it were not*.
#
# Method: a copy of the tree (tracked and untracked files, no `.git`, no build output), one mutation
# applied to a fresh copy at a time, and the suite run there. Three verdicts per mutation:
#
#   the named check went red      the mutation is caught          (M3-M6, M10-M14)
#   another check went red too    over-reach, reported with both names   (M7)
#   nothing went red              the mutation is invisible — the case this unit exists for   (M8)
#
# Mutations 1-4 are the specification's acceptance criteria 2 through 5. Mutations 5-9 were added by
# the adversarial pass, and every one of them ran green against the suite before its check existed —
# which is the only reason to trust the first four at all.
#
# The real repository is never written to. M1 is the row that proves it.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

RUNNER="$CHECKS_DIR/run.sh"

declare_row M1 "the real working tree is byte-identical before and after this check runs"
declare_row M2 "with no mutation applied, the copy runs the suite to exit 0"
declare_row M3 "mutation 1 — a deleted candidate row — reddens scan-loop, naming the candidate"
declare_row M4 "mutation 2 — a guarantee downgraded to none — reddens followups, naming the row"
declare_row M5 "mutation 3 — a follow-up pointing at nothing — reddens followups, naming the id"
declare_row M6 "mutation 4 — a deleted quoted fragment — reddens citations, naming the row"
declare_row M7 "each mutation is applied alone to a fresh copy; a check reddened beyond the named one is reported"
declare_row M8 "a mutation that reddens no check at all is a failed row naming the mutation"
declare_row M9 "the nine mutations are described in the audit's method section"
declare_row M10 "mutation 5 — a line inserted above a citation — reddens citations, naming the stale row"
declare_row M11 "mutation 6 — the heading an anchor names, renamed — reddens closed-cells"
declare_row M12 "mutation 7 — a reason repointed at this audit — reddens closed-cells and followups"
declare_row M13 "mutation 8 — a verdict repointed at a use site — reddens citations, naming the line"
declare_row M14 "mutation 9 — an open verdict repointed at the enum head — reddens citations"

BEFORE="$(git_status)"

LAB="$(scratch)" || { red_all "no scratch directory could be created"; finish; exit; }
trap 'rm -rf "$LAB"; :' EXIT

# ---- the copy -----------------------------------------------------------------------------------
# `git ls-files -co --exclude-standard` is the tree as the repository sees it: tracked plus
# untracked, minus everything ignored. No `.git` goes across, so nothing in the copy can reach the
# real repository's history — which is half of M1's guarantee and the cheaper half to enforce.

make_copy() { # <destination>
  local dest="$1"
  mkdir -p "$dest" || return 1
  ( cd "$REPO" && git ls-files -co --exclude-standard -z ) \
    | ( cd "$REPO" && tar --null -T - -cf - ) \
    | ( cd "$dest" && tar -xf - ) || return 1
  [ -f "$dest/$AUDIT_REL" ]
}

# verdicts <root>  — one `unit=PASS|FAIL` line per unit, read out of the runner's own table.
verdicts() {
  ( cd "$1" && bash "$1/.engineering/checks/run.sh" "${INNER_UNITS[@]}" 2>&1 ) \
    | awk '$1 == "PASS" || $1 == "FAIL" { if (seen[$2]++ == 0 && $2 ~ /^[a-z-]+$/) print $2 "=" $1 }'
}

verdict_of() { printf '%s\n' "$1" | sed -n "s/^$2=//p" | head -1; }

# ---- M2 -----------------------------------------------------------------------------------------
# The baseline. Without it M3-M6 are unattributable: a check that was already red proves nothing
# about the mutation that ran beside it.
R=0
BASE="$LAB/base"
BASE_VERDICTS=""
if ! make_copy "$BASE"; then
  R=1; why "the tree could not be copied, or the copy carries no $AUDIT_REL"
else
  note "inner runs cover ${#INNER_UNITS[@]} unit(s); excluded: $INNER_EXCLUDED (they run the suite themselves)"
  ( cd "$BASE" && bash "$BASE/.engineering/checks/run.sh" "${INNER_UNITS[@]}" ) > "$LAB/base.log" 2>&1
  BASE_STATUS=$?
  BASE_VERDICTS="$(verdicts "$BASE")"
  if [ "$BASE_STATUS" -ne 0 ]; then
    R=1
    why "the unmutated copy exited $BASE_STATUS; M3-M6 cannot be attributed to a mutation"
    while IFS= read -r l; do why "$l"; done < <(grep '^FAIL' "$LAB/base.log" | head -6)
  fi
fi
row M2 "$R"

# ---- the mutations ------------------------------------------------------------------------------
# Each is a function of the copy's root. They return 0 when the mutation was applied and 1 when the
# audit gave it nothing to bite on — which is itself a failure, reported as such.

mutate_1() { # delete the table row that carries a scan candidate
  local root="$1" audit="$1/$AUDIT_REL" cand ln
  while IFS= read -r cand; do
    [ -z "$cand" ] && continue
    ln="$(table_rows "$audit" | awk -F'\t' -v c="$cand" -v d=2 -v e=5 '
      index($2, c) || index($5, c) { print $1; exit }')"
    if [ -n "$ln" ]; then
      awk -v n="$ln" 'NR != n { print }' "$audit" > "$audit.new" && mv "$audit.new" "$audit"
      printf '%s' "candidate '$cand' (row $ln)"
      return 0
    fi
  done < <( cd "$root" && bash "$root/.engineering/checks/scan-declarations.sh" 2>/dev/null )
  return 1
}

mutate_2() { # a settled closed row's Guarantee downgraded to `none`, Follow-up left as the em dash
  local root="$1" audit="$1/$AUDIT_REL" line ln g s decl
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    g="$(cell "$line" "$COL_GUARANTEE")"; s="$(cell "$line" "$COL_REASON")"
    [ -z "$g" ] && continue
    [ "$g" = "none" ] && continue
    [ "$(cell "$line" "$COL_FOLLOWUP")" = "$EMDASH" ] || continue
    ln="$(row_lineno "$line")"; decl="$(cell "$line" "$COL_DECLARATION")"
    awk -v n="$ln" -v g="$g" 'NR == n { i = index($0, g); if (i) $0 = substr($0, 1, i - 1) "none" substr($0, i + length(g)) } { print }' \
      "$audit" > "$audit.new" && mv "$audit.new" "$audit"
    printf '%s' "row '$decl' guarantee -> none"
    return 0
  done <<< "$(rows_with_verdict "$audit" closed)"
  return 1
}

mutate_3() { # a Follow-up cell pointed at an id that is not in the store
  #
  # It used to take an existing unsettled row and repoint the id it already carried. That worked
  # only while the table had one, and the round that settled the last two closed rows left this
  # mutation with nothing to find — a mutation that cannot be applied is a proof that stopped
  # running, and it reports as one. So the state is now **constructed**: a settled row is downgraded
  # to need a follow-up and given one that resolves nowhere. F2's store lookup is the assertion
  # either way; only the setup moved from found to made.
  local root="$1" audit="$1/$AUDIT_REL" line ln g decl
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    g="$(cell "$line" "$COL_GUARANTEE")"
    [ -n "$g" ] && [ "$g" != "none" ] || continue
    [ "$(cell "$line" "$COL_FOLLOWUP")" = "$EMDASH" ] || continue
    ln="$(row_lineno "$line")"; decl="$(cell "$line" "$COL_DECLARATION")"
    awk -v n="$ln" -v g="$g" -v em="| $EMDASH |" -v repl="| story:ova-nowhere |" '
      NR == n {
        i = index($0, g); if (i) $0 = substr($0, 1, i - 1) "none" substr($0, i + length(g))
        j = index($0, em); if (j) $0 = substr($0, 1, j - 1) repl substr($0, j + length(em))
      }
      { print }' "$audit" > "$audit.new" && mv "$audit.new" "$audit"
    printf '%s' "row '$decl' follow-up -> story:ova-nowhere, on a row downgraded to need one"
    return 0
  done <<< "$(rows_with_verdict "$audit" closed)"
  return 1
}

mutate_4() { # the quoted fragment of an Invited at cell deleted from the corpus file it cites
  local root="$1" audit="$1/$AUDIT_REL" line at locus frag path decl
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    at="$(cell "$line" "$COL_INVITED")"
    locus="$(printf '%s' "$at" | tr -d '`' | grep -oE '[A-Za-z0-9._/-]+\.[A-Za-z0-9]+:[0-9]+' | head -1)"
    frag="$(printf '%s' "$at" | sed -n 's/.*[«"“]\([^"”»]*\)[»"”].*/\1/p' | head -1)"
    path="${locus%:*}"
    [ -n "$frag" ] && [ -n "$path" ] && [ -f "$root/$path" ] || continue
    grep -Fq "$frag" "$root/$path" || continue
    decl="$(cell "$line" "$COL_DECLARATION")"
    grep -vF "$frag" "$root/$path" > "$root/$path.new" && mv "$root/$path.new" "$root/$path"
    printf '%s' "fragment of row '$decl' removed from $path"
    return 0
  done <<< "$(table_rows "$audit")"
  return 1
}

# ---- the five the adversarial pass added ---------------------------------------------------------
# Each of these was applied to a copy by hand first and the suite ran to **exit 0** under it. That is
# the whole argument for them being here: mutations 1-4 showed the suite discriminates on the things
# it was written to discriminate on, and said nothing about the things nobody had thought to ask.

mutate_5() { # a line inserted above a cited fragment — every line number for that file goes stale
  local root="$1" audit="$1/$AUDIT_REL" line at locus path decl
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    at="$(cell "$line" "$COL_INVITED")"
    locus="$(printf '%s' "$at" | tr -d '`' | grep -oE '[A-Za-z0-9._/-]+\.[A-Za-z0-9]+:[0-9]+' | head -1)"
    path="${locus%:*}"
    [ -n "$path" ] && [ -f "$root/$path" ] || continue
    decl="$(cell "$line" "$COL_DECLARATION")"
    { printf '<!-- a line somebody added above the citation -->\n'; cat "$root/$path"; } > "$root/$path.new" \
      && mv "$root/$path.new" "$root/$path"
    printf '%s' "a line inserted at the top of $path, cited by '$decl'"
    return 0
  done <<< "$(table_rows "$audit")"
  return 1
}

mutate_6() { # the heading a Reason for adopters at anchor names, renamed
  local root="$1" audit="$1/$AUDIT_REL" line reason base anchor decl heading
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    reason="$(cell "$line" "$COL_REASON")"
    case "$reason" in *\#*) ;; *) continue ;; esac
    base="${reason%%#*}"; anchor="${reason#*#}"
    [ -n "$anchor" ] && [ -f "$root/$base" ] || continue
    heading="$(awk -v want="$anchor" '
      /^#+[[:space:]]/ {
        h = $0; sub(/^#+[[:space:]]*/, "", h)
        s = tolower(h); gsub(/[^a-z0-9 _-]/, "", s); gsub(/[ _]+/, "-", s)
        if (s == want) { print NR; exit }
      }' "$root/$base")"
    [ -n "$heading" ] || continue
    decl="$(cell "$line" "$COL_DECLARATION")"
    awk -v n="$heading" 'NR == n { print $0 " under another name" ; next } { print }' "$root/$base" \
      > "$root/$base.new" && mv "$root/$base.new" "$root/$base"
    printf '%s' "$base:$heading renamed, orphaning the #$anchor named by row '$decl'"
    return 0
  done <<< "$(rows_with_verdict "$audit" closed)"
  return 1
}

mutate_7() { # a closed row's reason repointed at this audit — the cell citing its own page
  #
  # It used to pick an already-unsettled row, so that the partition did not move and `followups`
  # stayed out of it. There is no such row once every closure is explained, and the honest version
  # is louder anyway: repointing a **settled** row's reason at this page takes its reason away, and
  # a row with no reason and no follow-up is exactly the state R10 refuses. So this one is expected
  # to redden two checks, and both are named in EXPECTED rather than tolerated by M7.
  local root="$1" audit="$1/$AUDIT_REL" line ln reason decl
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    reason="$(cell "$line" "$COL_REASON")"
    [ -n "$reason" ] && [ "$reason" != "none" ] || continue
    case "$reason" in "$AUDIT_REL"*) continue ;; esac
    ln="$(row_lineno "$line")"; decl="$(cell "$line" "$COL_DECLARATION")"
    awk -v n="$ln" -v old="$reason" -v new="$AUDIT_REL" '
      NR == n { i = index($0, old); if (i) $0 = substr($0, 1, i - 1) new substr($0, i + length(old)) }
      { print }' "$audit" > "$audit.new" && mv "$audit.new" "$audit"
    printf '%s' "row '$decl' reason -> $AUDIT_REL"
    return 0
  done <<< "$(rows_with_verdict "$audit" closed)"
  return 1
}

mutate_8() { # a Decided by repointed from the declaration to a line that merely uses it
  local root="$1" audit="$1/$AUDIT_REL" line by token dpath dline decl target
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    decl="$(cell "$line" "$COL_DECLARATION")"
    # Not a relation row: `layered-rows` resolves that one's line for itself, and a mutation that
    # reddens two units cannot be attributed to either. (It was the artifact-status row until that
    # pair opened at both layers and the layer check re-anchored onto relations.)
    case "$(printf '%s' "$decl" | tr '[:upper:]' '[:lower:]')" in *relation*) continue ;; esac
    by="$(cell "$line" "$COL_DECIDED")"
    token="$(printf '%s' "$by" | tr -d '`' | grep -oE 'crates/[A-Za-z0-9._/-]+:[0-9]+' | head -1)"
    [ -n "$token" ] || continue
    dpath="${token%:*}"; dline="${token##*:}"
    [ -f "$root/$dpath" ] || continue
    # The first line that is not an item declaration — a `use`, a doc comment, a statement.
    target="$(awk '
      NF && $0 !~ /^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?(enum|struct|const|static|type|trait|fn|impl|mod)[[:space:]]/ { print NR; exit }
    ' "$root/$dpath")"
    [ -n "$target" ] && [ "$target" != "$dline" ] || continue
    awk -v n="$(row_lineno "$line")" -v old="$dpath:$dline" -v new="$dpath:$target" '
      NR == n { i = index($0, old); if (i) $0 = substr($0, 1, i - 1) new substr($0, i + length(old)) }
      { print }' "$audit" > "$audit.new" && mv "$audit.new" "$audit"
    printf '%s' "row '$decl' decided by -> $dpath:$target"
    return 0
  done <<< "$(rows_with_verdict "$audit" closed)"
  return 1
}

mutate_9() { # an open row's crates/ citation moved off the escape hatch onto the enum head
  local root="$1" audit="$1/$AUDIT_REL" line by token dpath dline decl head
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    by="$(cell "$line" "$COL_DECIDED")"
    token="$(printf '%s' "$by" | tr -d '`' | grep -oE 'crates/[A-Za-z0-9._/-]+:[0-9]+' | head -1)"
    [ -n "$token" ] || continue
    dpath="${token%:*}"; dline="${token##*:}"
    [ -f "$root/$dpath" ] || continue
    # The enclosing `pub enum` — the line a reader would land on and read as closed.
    head="$(awk -v n="$dline" 'NR <= n && /^[[:space:]]*pub[[:space:]]+enum[[:space:]]/ { last = NR } END { print last }' "$root/$dpath")"
    [ -n "$head" ] && [ "$head" != "$dline" ] || continue
    decl="$(cell "$line" "$COL_DECLARATION")"
    awk -v n="$(row_lineno "$line")" -v old="$dpath:$dline" -v new="$dpath:$head" '
      NR == n { i = index($0, old); if (i) $0 = substr($0, 1, i - 1) new substr($0, i + length(old)) }
      { print }' "$audit" > "$audit.new" && mv "$audit.new" "$audit"
    printf '%s' "open row '$decl' decided by -> $dpath:$head, the enum head"
    return 0
  done <<< "$(rows_with_verdict "$audit" open)"
  return 1
}

MUTATIONS=(mutate_1 mutate_2 mutate_3 mutate_4 mutate_5 mutate_6 mutate_7 mutate_8 mutate_9)
# One entry per mutation, and an entry may name **more than one** unit: a mutation whose real
# consequence is two red checks is described that way rather than trimmed to one, because the
# alternative is M7 reporting the second as over-reach every run until somebody silences it.
EXPECTED=(scan-loop followups followups citations citations closed-cells "closed-cells followups" citations citations)
ROWS_FOR=(M3 M4 M5 M6 M10 M11 M12 M13 M14)
M7_R=0
M8_R=0
declare -A OUTCOME=()

i=0
while [ "$i" -lt "${#MUTATIONS[@]}" ]; do
  fn="${MUTATIONS[$i]}"; want="${EXPECTED[$i]}"; rid="${ROWS_FOR[$i]}"
  n=$((i + 1))
  R=0
  COPY="$LAB/mut$n"
  # A mutation that could not be evaluated leaves M7 and M8 asserting nothing, and a vacuous check
  # is a failed check — so an unevaluable mutation reddens them too, not only its own row.
  if [ -z "$BASE_VERDICTS" ]; then
    R=1; M7_R=1; M8_R=1; why "no baseline to compare mutation $n against"
  elif ! make_copy "$COPY"; then
    R=1; M7_R=1; M8_R=1; why "the tree could not be copied for mutation $n"
  else
    DESC="$($fn "$COPY")"
    if [ -z "$DESC" ]; then
      R=1; M7_R=1; M8_R=1; why "mutation $n found nothing in the audit to mutate"
    else
      note "mutation $n: $DESC"
      AFTER_V="$(verdicts "$COPY")"
      for unit_wanted in $want; do
        got="$(verdict_of "$AFTER_V" "$unit_wanted")"
        if [ "$got" != "FAIL" ]; then
          R=1; M8_R=1
          why "mutation $n left $unit_wanted at '${got:-<no row>}' — the mutation is invisible to the suite"
        fi
      done
      # M7: over-reach. Any unit that was PASS in the baseline and FAIL here, other than the named
      # one, is reported with both names.
      while IFS= read -r entry; do
        [ -z "$entry" ] && continue
        unit="${entry%%=*}"; verdict="${entry##*=}"
        case " $want " in *" $unit "*) continue ;; esac
        [ "$verdict" = "FAIL" ] || continue
        [ "$(verdict_of "$BASE_VERDICTS" "$unit")" = "PASS" ] || continue
        M7_R=1
        why "mutation $n was expected to redden $want and also reddened $unit"
      done <<< "$AFTER_V"
    fi
  fi
  OUTCOME["$rid"]="$R"
  i=$((i + 1))
done

row M3 "${OUTCOME[M3]:-1}"
row M4 "${OUTCOME[M4]:-1}"
row M5 "${OUTCOME[M5]:-1}"
row M6 "${OUTCOME[M6]:-1}"
row M10 "${OUTCOME[M10]:-1}"
row M11 "${OUTCOME[M11]:-1}"
row M12 "${OUTCOME[M12]:-1}"
row M13 "${OUTCOME[M13]:-1}"
row M14 "${OUTCOME[M14]:-1}"
row M7 "$M7_R"
row M8 "$M8_R"

# ---- M1 -----------------------------------------------------------------------------------------
# Reported after the mutations, because it is a claim about what they did — not before, where it
# would be a claim about nothing.
R=0
AFTER="$(git_status)"
if [ "$BEFORE" != "$AFTER" ]; then
  R=1
  why "the real working tree changed while the mutations ran:"
  while IFS= read -r l; do
    [ -n "$l" ] && why "$l"
  done < <(diff <(printf '%s\n' "$BEFORE") <(printf '%s\n' "$AFTER") | head -6)
fi
row M1 "$R"

# ---- M9 -----------------------------------------------------------------------------------------
# So a future round can re-run them by hand. A mutation the audit does not describe is one nobody
# will repeat once this script is the only record of it.
R=0
if ! audit_present; then
  R=1; why "no $AUDIT_REL"
else
  METHOD="$(section_by_heading "$AUDIT" 'produced|method|mutation|how it was made')"
  if [ -z "$METHOD" ]; then
    R=1; why "the audit has no section describing the mutations"
  else
    for phrase in 'row' 'guarantee' 'follow-up' 'fragment' 'line number' 'anchor' 'this page' 'use site'; do
      grep -qi "$phrase" <<< "$METHOD" \
        || { R=1; why "the method section does not describe the mutation involving the $phrase"; }
    done
    COUNT="$(grep -ciE 'mutation' <<< "$METHOD")"
    [ "${COUNT:-0}" -ge 1 ] || { R=1; why "the method section never uses the word mutation"; }
    # The number in the prose against the number this check runs. A method that describes four
    # mutations while eight run is the same defect as a stale corpus count.
    STATED="$(grep -oiE '\b(one|two|three|four|five|six|seven|eight|nine|ten|[0-9]+)\b[[:space:]]+deliberate[[:space:]]+mutations?' <<< "$METHOD" \
      | head -1 | awk '{ print $1 }' | tr '[:upper:]' '[:lower:]')"
    WORDS=" one=1 two=2 three=3 four=4 five=5 six=6 seven=7 eight=8 nine=9 ten=10 "
    case "$STATED" in
      '') R=1; why "the method section states no count of deliberate mutations" ;;
      *[!0-9]*) NUM="${WORDS##* $STATED=}"; NUM="${NUM%% *}" ;;
      *) NUM="$STATED" ;;
    esac
    if [ -n "${NUM:-}" ]; then
      [ "$NUM" -eq "${#MUTATIONS[@]}" ] \
        || { R=1; why "the method section states $STATED deliberate mutation(s); ${#MUTATIONS[@]} run"; }
    fi
  fi
fi
row M9 "$R"

finish
