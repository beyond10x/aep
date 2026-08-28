#!/usr/bin/env bash
# task:ova-surface-hygiene — H1 … H9.  (Acceptance 6 and 7, R18, and the Constraints.)
#
# The unit that keeps the run inside its declared lines, and the checks honest about where they get
# their facts.
#
# H5 is the one with a trap in it. A check that greps for the literal path `.engineering/pl…ing`
# would contain that literal path, and H9 says H5 covers every file in this directory including this
# one. So the forbidden prefixes are **assembled at run time** from parts that are not themselves
# paths. That is not cleverness for its own sake: the alternative is a check that exempts itself,
# and an exemption is exactly how a rule stops being one.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

declare_row H1 "protocol validate --root . exits 0, with its output relayed"
declare_row H2 "protocol artifact validate exits 0, with its output relayed"
declare_row H3 "git status lists changed paths only under docs/ and .engineering/"
declare_row H4 "the model runner is unchanged: it is read as a model, never edited"
declare_row H5 "no check names a planning path — protocol artifact list is the only route to the store"
declare_row H6 "the suite runs to exit 0 with jq, yq, curl, wget, nc, python, node and cargo stubbed"
declare_row H7 "no script contains a literal temporary-directory path; scratch derives from TMPDIR"
declare_row H8 "a run with TMPDIR pointed at an empty directory leaves it empty, red path and green"
declare_row H9 "H5, H6 and H7 enumerate this directory rather than a list, so a later check is covered"

# The directory, enumerated once. H9's subject.
SCRIPTS=()
for f in "$CHECKS_DIR"/run.sh "$CHECKS_DIR"/lib.sh "$CHECKS_DIR"/check-*.sh "$CHECKS_DIR"/scan-declarations.sh; do
  [ -f "$f" ] && SCRIPTS+=("$f")
done

# ---- H1 -----------------------------------------------------------------------------------------
R=0
if ! protocol_ready; then
  R=1; why "$(protocol_absence)"
else
  OUT="$( ( cd "$REPO" && "$PROTOCOL" validate --root . ) 2>&1 )"
  if [ $? -ne 0 ]; then
    R=1
    while IFS= read -r l; do [ -n "$l" ] && why "$l"; done <<< "$OUT"
  else
    note "$(head -1 <<< "$OUT")"
  fi
fi
row H1 "$R"

# ---- H2 -----------------------------------------------------------------------------------------
R=0
if ! protocol_ready; then
  R=1; why "$(protocol_absence)"
else
  OUT="$( ( cd "$REPO" && "$PROTOCOL" artifact validate ) 2>&1 )"
  if [ $? -ne 0 ]; then
    R=1
    while IFS= read -r l; do [ -n "$l" ] && why "$l"; done <<< "$OUT"
  else
    note "$(head -1 <<< "$OUT")"
    note "read by $PROTOCOL ($(workspace_version))"
  fi
fi
row H2 "$R"

# ---- H3 -----------------------------------------------------------------------------------------
# Acceptance criterion 7. Each offending path named individually — a count tells nobody which
# constraint was crossed.
R=0
CHANGED=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  path="${line:3}"
  path="${path%% -> *}"
  CHANGED=$((CHANGED + 1))
  case "$path" in
    docs/*|.engineering/*) ;;
    *) R=1; why "changed outside the declared surface: $path" ;;
  esac
done < <(git_status)
note "$CHANGED changed path(s) in the working tree"
row H3 "$R"

# ---- H4 -----------------------------------------------------------------------------------------
# The model left this repository on 2026-08-22 with `epic:metaharness-migration` — it is metaharness
# `evals/engineering-protocols/checks/run-checks.sh` now, and no check here can read a file in
# another repository. So the row asserts what is still assertable: that this suite has not grown its
# own copy of the model under the deleted path, which is how a migrated model comes back as a fork.
R=0
if [ -n "$MODEL_RUNNER_REL" ]; then
  if [ ! -f "$REPO/$MODEL_RUNNER_REL" ]; then
    R=1; why "$MODEL_RUNNER_REL does not exist — the model this suite was written against is gone"
  elif ! git -C "$REPO" diff --quiet -- "$MODEL_RUNNER_REL" 2>/dev/null; then
    R=1; why "$MODEL_RUNNER_REL has been modified; it is read as a model and never edited"
  fi
elif [ -e "$REPO/integrations/claude-code/eval" ]; then
  R=1
  why "integrations/claude-code/eval/ is back — the model moved to metaharness"
  why "evals/engineering-protocols/ on 2026-08-22 and a second copy here is a fork of it"
else
  note "the model is metaharness evals/engineering-protocols/checks/run-checks.sh, read there"
fi
row H4 "$R"

# ---- H5 -----------------------------------------------------------------------------------------
# R18. Assembled, never written: see this file's header for why.
R=0
E=".engineering"
FORBIDDEN=("$E/$(printf 'plan')$(printf 'ning')" "$E/$(printf 'task-w4-2.yaml')")
if [ "${#SCRIPTS[@]}" -eq 0 ]; then
  R=1; why "no scripts found to inspect"
else
  for f in "${SCRIPTS[@]}"; do
    for bad in "${FORBIDDEN[@]}"; do
      if grep -qF "$bad" "$f"; then
        R=1; why "${f##*/} names $bad — R18 allows only \`protocol artifact list\` for store state"
      fi
    done
  done
  note "${#SCRIPTS[@]} script(s) inspected for a planning path"
fi
row H5 "$R"

# ---- H6 -----------------------------------------------------------------------------------------
# Hermeticity by shadowing. `bash`, `git` and `protocol` are the three programs the map declares; a
# fourth dependency that crept in exits 127 under its stub, and the suite goes red rather than
# quietly requiring a tool the driver never promised.
R=0
LAB="$(scratch)"
if [ -z "$LAB" ] || [ ! -f "$CHECKS_DIR/run.sh" ]; then
  R=1; why "no scratch directory, or no run.sh to run under stubs"
else
  trap 'rm -rf "$LAB"' EXIT
  STUBS="$LAB/stubs"; mkdir -p "$STUBS"
  for prog in jq yq curl wget nc python python3 node cargo; do
    printf '#!/usr/bin/env bash\nexit 127\n' > "$STUBS/$prog"
    chmod +x "$STUBS/$prog"
  done
  note "inner run covers ${#INNER_UNITS[@]} unit(s); excluded: $INNER_EXCLUDED (they run the suite themselves)"
  OUT="$( cd "$REPO" && PATH="$STUBS:$PATH" bash "$CHECKS_DIR/run.sh" "${INNER_UNITS[@]}" 2>&1 )"
  if [ $? -ne 0 ]; then
    R=1
    why "the suite did not exit 0 under stubs"
    while IFS= read -r l; do why "$l"; done < <(grep '^FAIL' <<< "$OUT" | head -6)
  fi
fi
row H6 "$R"

# ---- H7 -----------------------------------------------------------------------------------------
R=0
if [ "${#SCRIPTS[@]}" -eq 0 ]; then
  R=1; why "no scripts found to inspect"
else
  for f in "${SCRIPTS[@]}"; do
    grep -qF "$FORBIDDEN_TMP/" "$f" \
      && { R=1; why "${f##*/} contains a literal $FORBIDDEN_TMP path"; }
  done
  grep -qF 'TMPDIR' "$CHECKS_DIR/lib.sh" \
    || { R=1; why "lib.sh does not derive its scratch base from TMPDIR"; }
fi
row H7 "$R"

# ---- H8 -----------------------------------------------------------------------------------------
# Both paths. A suite that tidies up only when it passes leaves its evidence behind exactly when
# somebody is looking for it.
R=0
if [ -z "${LAB:-}" ] || [ ! -f "$CHECKS_DIR/run.sh" ]; then
  R=1; why "no scratch directory, or no run.sh"
else
  PROBE="$LAB/probe"; mkdir -p "$PROBE"
  for unit in checks-runner audit-corpus; do
    rm -rf "${PROBE:?}"/*
    ( cd "$REPO" && TMPDIR="$PROBE" bash "$CHECKS_DIR/run.sh" "$unit" ) > /dev/null 2>&1
    verdict=$?
    LEFT="$(find "$PROBE" -mindepth 1 2>/dev/null | grep -c .)"
    [ "$LEFT" -eq 0 ] \
      || { R=1; why "$LEFT path(s) left in TMPDIR after running $unit (exit $verdict)"; }
    note "$unit exited $verdict, TMPDIR left with $LEFT entr(ies)"
  done
fi
row H8 "$R"

# ---- H9 -----------------------------------------------------------------------------------------
# The rule that keeps H5, H6 and H7 true of a check written next month.
R=0
ON_DISK="$(ls "$CHECKS_DIR"/check-*.sh 2>/dev/null | grep -c .)"
INSPECTED_CHECKS=0
for f in "${SCRIPTS[@]}"; do
  case "${f##*/}" in check-*.sh) INSPECTED_CHECKS=$((INSPECTED_CHECKS + 1)) ;; esac
done
[ "$INSPECTED_CHECKS" -eq "${ON_DISK:-0}" ] \
  || { R=1; why "inspected $INSPECTED_CHECKS check script(s) of ${ON_DISK:-0} on disk"; }
grep -qF 'check-*.sh' "${BASH_SOURCE[0]}" \
  || { R=1; why "this check does not enumerate the directory, so a later check would escape H5-H7"; }
row H9 "$R"

finish
