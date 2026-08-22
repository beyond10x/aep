#!/usr/bin/env bash
# task:ova-citations — I1 … I10.  (R6, R7.)
#
# The two columns that make the audit falsifiable. Everything else in the table is a claim about the
# repository; these are the claims that resolve against it, and the quoted fragment is the drift
# detector — a guide that stops inviting a declaration turns its row red, which is the signal the
# next round needs.
#
# Two cell grammars this unit fixes, because R6 and R7 name the parts without naming the form:
#
#   Invited at   `<corpus path>:<line>` … "<verbatim fragment>"     (ASCII or typographic quotes)
#   Decided by   `<path>:<key>`  or  `<path>:<line>`
#
# Backticks are markdown and are stripped before anything is resolved.
#
# ## Two rows the adversarial pass added, and the mutations that would have passed without them
#
# I3 and I6 resolve the *path* halves of both grammars and leave the numbers decorative — a fragment
# found anywhere in the file satisfies I3, and any line within the file's length satisfies I6. Two
# mutations went straight through the suite because of it:
#
#   a line inserted at the top of a cited corpus file   every number for that file off by one, green
#   a Decided by repointed from the enum to a use site  a verdict cited to code that does not settle it
#
# I11 and I12 are those two holes closed. Both are drift the next round is *supposed* to catch: a
# citation a reader cannot follow to the sentence it names has stopped being a citation, and the
# audit's own promise is that re-running the suite says which citations no longer resolve.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

declare_row I1  "every Invited at cell parses as a path, a line number and a quoted fragment"
declare_row I2  "each such path is a member of the corpus, re-derived from the globs"
declare_row I3  "each quoted fragment occurs verbatim in the file it cites"
declare_row I4  "each cited line number is within that file's length"
declare_row I5  "every Decided by cell is a path in one of the two permitted forms"
declare_row I6  "for the file:line form, the file exists and has at least that many lines"
declare_row I7  "for the file:key form, the key occurs as a declaration, not as prose"
declare_row I8  "no Decided by cell is prose: an unresolvable cell is red and named"
declare_row I9  "deleting a cited fragment on a copy turns I3 red — the fragment really is a detector"
declare_row I10 "cells checked equals the table's row count: no row is skipped for being unparseable"
declare_row I11 "each quoted fragment occurs at the line the cell cites, not merely somewhere in the file"
declare_row I12 "a closed row's crates/ file:line is an item declaration, never a use site"
declare_row I13 "an open row's crates/ file:line is the variant that admits a free value, not the enum head"

audit_present || { red_all "no $AUDIT_REL"; finish; exit; }

strip_md() { printf '%s' "$1" | tr -d '`'; }

# The fragment, between the first pair of quotes of either flavour.
fragment_of() {
  printf '%s' "$1" | sed -n 's/.*[«"“]\([^"”»]*\)[»"”].*/\1/p' | head -1
}

# The `path:line` token, backticks removed.
locus_of() {
  strip_md "$1" | grep -oE '[A-Za-z0-9._/-]+\.[A-Za-z0-9]+:[0-9]+' | head -1
}

ROWS="$(table_rows "$AUDIT")"
ROW_N="$(printf '%s\n' "$ROWS" | grep -c .)"

I1_R=0; I2_R=0; I3_R=0; I4_R=0; I5_R=0; I6_R=0; I7_R=0; I8_R=0; I11_R=0; I12_R=0; I13_R=0
I11_SEEN=0; I12_SEEN=0; I13_SEEN=0
CHECKED=0
SAMPLE_FILE=""; SAMPLE_FRAG=""; SAMPLE_DECL=""

if [ "${ROW_N:-0}" -eq 0 ]; then
  red_all "the audit's table carries no data rows, so no citation was resolved"
  finish
  exit
fi

while IFS= read -r line; do
  [ -z "$line" ] && continue
  CHECKED=$((CHECKED + 1))
  ln="$(row_lineno "$line")"
  decl="$(cell "$line" "$COL_DECLARATION")"
  at="$(cell "$line" "$COL_INVITED")"
  by="$(cell "$line" "$COL_DECIDED")"
  verdict="$(cell "$line" "$COL_VERDICT")"
  tag="'${decl:-<no declaration>}' ($AUDIT_REL:$ln)"

  # ---- Invited at ------------------------------------------------------------------------------
  locus="$(locus_of "$at")"
  frag="$(fragment_of "$at")"
  path="${locus%:*}"
  lineno="${locus##*:}"

  if [ -z "$locus" ] || [ -z "$frag" ]; then
    I1_R=1
    [ -z "$locus" ] && why "$tag Invited at carries no path:line — got '${at:-<empty>}'"
    [ -z "$frag" ] && why "$tag Invited at carries no quoted fragment — got '${at:-<empty>}'"
  else
    if ! in_corpus "$path"; then
      I2_R=1; why "$tag cites $path, which is not in the R1 corpus"
    fi
    if [ ! -f "$REPO/$path" ]; then
      I3_R=1; why "$tag cites $path, which does not exist"
      I4_R=1
    else
      grep -Fq "$frag" "$REPO/$path" \
        || { I3_R=1; why "$tag quotes \"$frag\", which no longer occurs in $path"; }
      total="$(file_lines "$REPO/$path")"
      if [ "$lineno" -ge 1 ] && [ "$lineno" -le "$total" ]; then
        # I11 — the number, resolved. `awk` prints the one line, and the fragment has to be in it.
        # A fragment that is in the file but not at the cited line is a citation a reader follows to
        # the wrong place, and it is what a line inserted above it leaves behind.
        I11_SEEN=$((I11_SEEN + 1))
        at_line="$(awk -v n="$lineno" 'NR == n { print; exit }' "$REPO/$path")"
        if ! grep -Fq "$frag" <<< "$at_line"; then
          I11_R=1
          why "$tag cites $path:$lineno, which reads '${at_line:-<empty>}' — the fragment is not there"
          actual="$(grep -nF "$frag" "$REPO/$path" | head -1 | cut -d: -f1)"
          [ -n "$actual" ] && why "the fragment is at $path:$actual — the citation is stale by $((actual - lineno)) line(s)"
        fi
      else
        I4_R=1; why "$tag cites $path:$lineno; the file has $total line(s)"
      fi
      if [ -z "$SAMPLE_FILE" ] && grep -Fq "$frag" "$REPO/$path"; then
        SAMPLE_FILE="$path"; SAMPLE_FRAG="$frag"; SAMPLE_DECL="$decl"
      fi
    fi
  fi

  # ---- Decided by ------------------------------------------------------------------------------
  # R7's invariant: no cell here may be prose. A verdict that cannot be attached to a file in this
  # tree is one that was never entered.
  token="$(strip_md "$by" | grep -oE '[A-Za-z0-9._/-]+:[A-Za-z0-9_.-]+' | head -1)"
  if [ -z "$token" ]; then
    I5_R=1; I8_R=1
    why "$tag Decided by is not a path in either form — got '${by:-<empty>}'"
  else
    dpath="${token%:*}"
    dsuffix="${token##*:}"
    if [ ! -f "$REPO/$dpath" ]; then
      I5_R=1; I8_R=1
      why "$tag Decided by names $dpath, which does not exist in this tree"
    elif printf '%s' "$dsuffix" | grep -qE '^[0-9]+$'; then
      total="$(file_lines "$REPO/$dpath")"
      if [ "$dsuffix" -ge 1 ] && [ "$dsuffix" -le "$total" ]; then
        # I12 — R7 asks for a path that **settles** the verdict, and its own example is an `enum`
        # declaration. A line inside `crates/` that merely *uses* the type settles nothing: a reader
        # who follows `RelationKind::parse(…)` still has to go and find `RelationKind` to learn
        # whether it has an escape hatch. Only the declaration answers the question the row asks.
        case "$dpath" in
          crates/*)
            at_decl="$(awk -v n="$dsuffix" 'NR == n { print; exit }' "$REPO/$dpath")"
            if [ "$verdict" = "open" ]; then
              # I13 — the mirror of I12, and the one that caught a live row. An `open` verdict cited
              # to `pub enum TestSuite {` sends the reader to a ten-variant enum and lets them
              # conclude *closed*: the line that settles `open` is the variant carrying a free name,
              # `Named(String)` twenty-two lines further down. A citation that argues against its own
              # verdict is worse than none, because a reader who checks it stops trusting the table.
              I13_SEEN=$((I13_SEEN + 1))
              if ! grep -qE '^[[:space:]]*[A-Z][A-Za-z0-9_]*\(String[,)]' <<< "$at_decl"; then
                I13_R=1
                why "$tag is open and decides at $dpath:$dsuffix, which reads '${at_decl:-<empty>}' — that line does not admit a value of the adopter's own"
              fi
            else
              I12_SEEN=$((I12_SEEN + 1))
              if ! grep -qE '^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?(enum|struct|const|static|type|trait|fn|impl|mod)[[:space:]]' <<< "$at_decl"; then
                I12_R=1
                why "$tag decides at $dpath:$dsuffix, which reads '${at_decl:-<empty>}' — a use site, not a declaration"
              fi
            fi
            ;;
        esac
      else
        I6_R=1; why "$tag decides at $dpath:$dsuffix; the file has $total line(s)"
      fi
    else
      grep -qE "^[[:space:]]*(- )?$dsuffix:" "$REPO/$dpath" \
        || { I7_R=1; why "$tag names key '$dsuffix' in $dpath, where it is not declared"; }
    fi
  fi
done <<< "$ROWS"

row I1 "$I1_R"
row I2 "$I2_R"
row I3 "$I3_R"
row I4 "$I4_R"
row I5 "$I5_R"
row I6 "$I6_R"
row I7 "$I7_R"
row I8 "$I8_R"

# ---- I9 -----------------------------------------------------------------------------------------
# Acceptance criterion 5, on a copy. One real citation, its file copied, the fragment deleted, and
# the same predicate asked again. If it still says yes, I3 is checking nothing.
R=0
if [ -z "$SAMPLE_FILE" ]; then
  R=1; why "no resolving citation to mutate — I3 has nothing to discriminate on"
else
  LAB="$(scratch)"
  if [ -z "$LAB" ]; then
    R=1; why "no scratch directory could be created"
  else
    trap 'rm -rf "$LAB"' EXIT
    MUT="$LAB/mutated"
    cp "$REPO/$SAMPLE_FILE" "$MUT"
    grep -vF "$SAMPLE_FRAG" "$MUT" > "$MUT.new" && mv "$MUT.new" "$MUT"
    if grep -Fq "$SAMPLE_FRAG" "$MUT"; then
      R=1; why "the fragment survived being deleted from the copy — the mutation did not apply"
    fi
    note "mutated a copy of $SAMPLE_FILE (cited by '${SAMPLE_DECL}'), fragment removed"
  fi
fi
row I9 "$R"

# ---- I10 ----------------------------------------------------------------------------------------
R=0
[ "$CHECKED" -eq "${ROW_N:-0}" ] \
  || { R=1; why "checked $CHECKED row(s) of ${ROW_N:-0} — a row was skipped"; }
row I10 "$R"

# ---- I11 / I12 ----------------------------------------------------------------------------------
# Both carry their own floor. `I11` over zero resolvable citations and `I12` over zero `crates/`
# citations are each true of a table that cites nothing, which is the state they exist to refuse.
[ "$I11_SEEN" -ge 1 ] \
  || { I11_R=1; why "no row cites a line inside an existing corpus file — I11 asserted nothing"; }
note "$I11_SEEN fragment(s) at their cited line; $I12_SEEN closed and $I13_SEEN open crates/ citation(s)"
row I11 "$I11_R"

[ "$I12_SEEN" -ge 1 ] \
  || { I12_R=1; why "no closed row decides a verdict at a crates/ file:line — I12 asserted nothing"; }
row I12 "$I12_R"

# I13 has no floor of its own: a table where every open verdict is settled by a document key is the
# *better* table, and an empty set of open crates/ citations is that table, not a vacuous check.
note "$I13_SEEN open row(s) decide a verdict inside crates/ — each needs the escape hatch on the cited line"
row I13 "$I13_R"

finish
