#!/usr/bin/env bash
# task:ova-scan-declarations — S1 … S9.
#
# The derivation, and its determinism. Two contracts this check fixes, because S7 and S3 cannot be
# shown without them:
#
#   * `bash scan-declarations.sh [<root>]` — the optional argument is the tree to read, defaulting to
#     the repository root. S7 needs a *copy* of a protocol document to add a key to, and mutating the
#     real one to observe the scan is exactly the thing the audit forbids.
#   * stdout is candidates and nothing else. Diagnostics go to stderr, so S1 can insist on it.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

AEP="$REPO/protocols/aep/1.yaml"
ADP="$REPO/protocols/adp/1.yaml"

declare_row S1 "the scan exits 0 and prints candidates, one per line, with no blanks on stdout"
declare_row S2 "its output equals its own sort -u — sorted, no duplicates"
declare_row S3 "two consecutive runs against an unchanged tree are byte-identical"
declare_row S4 "every top-level vocabulary key of protocols/aep/1.yaml appears, asserted by name"
declare_row S5 "every vocabulary key protocols/adp/1.yaml extends appears, derived from that file"
declare_row S6 "the four artifacts/ families appear: kinds, lifecycles, relations, templates"
declare_row S7 "a key added to a copy appears as a candidate; removing it makes it disappear"
declare_row S8 "the scan writes nothing into the tree: git status is identical before and after"
declare_row S9 "it runs with curl, wget, nc, jq, yq, python and node shadowed by stubs exiting 127"

scan_present || { red_all "no ${SCAN#"$REPO"/}"; finish; exit; }

LAB="$(scratch)" || { red_all "no scratch directory could be created"; finish; exit; }
trap 'rm -rf "$LAB"' EXIT

# ---- S1 -----------------------------------------------------------------------------------------
R=0
BEFORE_STATUS="$(git_status)"
OUT1="$LAB/run1.txt"
( cd "$REPO" && bash "$SCAN" ) > "$OUT1" 2> "$LAB/run1.err"
[ $? -eq 0 ] || { R=1; why "the scan exited non-zero: $(head -3 "$LAB/run1.err")"; }
LINES="$(grep -c . "$OUT1")"
[ "${LINES:-0}" -ge 1 ] || { R=1; why "the scan emitted no candidates"; }
BLANKS="$(grep -c '^[[:space:]]*$' "$OUT1")"
[ "${BLANKS:-0}" -eq 0 ] || { R=1; why "$BLANKS blank line(s) on stdout"; }
row S1 "$R"

# ---- S2 -----------------------------------------------------------------------------------------
R=0
if ! sort -u "$OUT1" | cmp -s - "$OUT1"; then
  R=1
  why "output is not its own sort -u; first difference:"
  why "$(sort -u "$OUT1" | diff - "$OUT1" | head -4 | tr '\n' ' ')"
fi
row S2 "$R"

# ---- S3 -----------------------------------------------------------------------------------------
R=0
OUT2="$LAB/run2.txt"
( cd "$REPO" && bash "$SCAN" ) > "$OUT2" 2>/dev/null
cmp -s "$OUT1" "$OUT2" || { R=1; why "two runs against an unchanged tree differ"; }
row S3 "$R"

# ---- S4 -----------------------------------------------------------------------------------------
# By name, as the task states them. These seven are the specification's own list from Context.
R=0
if [ ! -f "$AEP" ]; then
  R=1; why "protocols/aep/1.yaml does not exist"
else
  for key in capabilities evidence_kinds verifiers artifact_kinds phases observables scales; do
    grep -qE "^$key:" "$AEP" || { R=1; why "$key is not a top-level key of protocols/aep/1.yaml"; }
    grep -qF "$key" "$OUT1" || { R=1; why "the scan does not emit a candidate for $key"; }
  done
fi
row S4 "$R"

# ---- S5 -----------------------------------------------------------------------------------------
# Derived from the file, not typed here: a list in this check would go stale in exactly the case the
# check exists to catch.
R=0
if [ ! -f "$ADP" ]; then
  R=1; why "protocols/adp/1.yaml does not exist"
else
  N=0
  while IFS= read -r key; do
    case "$key" in id|version|title|summary|extends) continue ;; esac
    N=$((N + 1))
    grep -qF "$key" "$OUT1" || { R=1; why "adp/1 extends $key and the scan does not emit it"; }
  done < <(sed -n 's/^\([a-z_][a-z_]*\):.*/\1/p' "$ADP")
  [ "$N" -ge 1 ] || { R=1; why "no vocabulary key was derived from protocols/adp/1.yaml at all"; }
fi
row S5 "$R"

# ---- S6 -----------------------------------------------------------------------------------------
R=0
for family in kinds lifecycles relations templates; do
  [ -d "$REPO/artifacts/$family" ] || { R=1; why "artifacts/$family does not exist"; }
  grep -qF "$family" "$OUT1" || { R=1; why "the scan does not emit a candidate for artifacts/$family"; }
done
row S6 "$R"

# ---- S7 -----------------------------------------------------------------------------------------
# Both directions, against a copy. The real tree is never written to — S8 is the row that proves it.
R=0
COPY="$LAB/tree"
mkdir -p "$COPY"
if ! cp -R "$REPO/protocols" "$REPO/artifacts" "$COPY/" 2>/dev/null; then
  R=1; why "the protocols/ and artifacts/ trees could not be copied for the mutation"
else
  BASE="$LAB/copy-base.txt"
  ( cd "$COPY" && bash "$SCAN" "$COPY" ) > "$BASE" 2>/dev/null \
    || { R=1; why "the scan does not accept a root argument — S7 cannot be shown without mutating the tree"; }

  printf '\nova_probe_vocabulary:\n  - id: probe\n' >> "$COPY/protocols/aep/1.yaml"
  ADDED="$LAB/copy-added.txt"
  ( cd "$COPY" && bash "$SCAN" "$COPY" ) > "$ADDED" 2>/dev/null
  grep -qF 'ova_probe_vocabulary' "$ADDED" \
    || { R=1; why "a top-level key added to the copy produced no new candidate"; }

  # …and away again. Removing the key must remove the candidate, or the scan is accumulating rather
  # than deriving.
  cp "$REPO/protocols/aep/1.yaml" "$COPY/protocols/aep/1.yaml"
  REMOVED="$LAB/copy-removed.txt"
  ( cd "$COPY" && bash "$SCAN" "$COPY" ) > "$REMOVED" 2>/dev/null
  grep -qF 'ova_probe_vocabulary' "$REMOVED" \
    && { R=1; why "the probe candidate survived the key being removed"; }
  cmp -s "$BASE" "$REMOVED" || { R=1; why "the scan did not return to its pre-mutation output"; }
fi
row S7 "$R"

# ---- S8 -----------------------------------------------------------------------------------------
R=0
AFTER_STATUS="$(git_status)"
if [ "$BEFORE_STATUS" != "$AFTER_STATUS" ]; then
  R=1
  why "the working tree changed across the scan:"
  why "$(diff <(printf '%s\n' "$BEFORE_STATUS") <(printf '%s\n' "$AFTER_STATUS") | head -5 | tr '\n' ' ')"
fi
row S8 "$R"

# ---- S9 -----------------------------------------------------------------------------------------
# Hermeticity by shadowing, not by reading the source. A dependency the scan actually reaches for
# exits 127 under the stub and the run fails, which is the only honest way to ask this question.
R=0
STUBS="$LAB/stubs"
mkdir -p "$STUBS"
for prog in curl wget nc jq yq python python3 node; do
  printf '#!/usr/bin/env bash\nexit 127\n' > "$STUBS/$prog"
  chmod +x "$STUBS/$prog"
done
STUBBED="$LAB/stubbed.txt"
( cd "$REPO" && PATH="$STUBS:$PATH" bash "$SCAN" ) > "$STUBBED" 2> "$LAB/stubbed.err"
if [ $? -ne 0 ]; then
  R=1; why "the scan failed under stubs: $(head -3 "$LAB/stubbed.err" | tr '\n' ' ')"
elif ! cmp -s "$OUT1" "$STUBBED"; then
  R=1; why "the scan's output changed under stubs — it is reading one of them"
fi
row S9 "$R"

finish
