#!/usr/bin/env bash
# The derivation R12 specifies: the declaration surfaces this tree declares *in documents*.
#
#   bash .engineering/checks/scan-declarations.sh            # the repository this file lives in
#   bash .engineering/checks/scan-declarations.sh <root>     # any tree with protocols/ and artifacts/
#
# The optional root exists so a check can show the scan reacting to a key it added — on a copy.
# Mutating the real `protocols/aep/1.yaml` to observe the derivation is exactly the thing the audit
# forbids, so the argument is part of the contract and not a convenience.
#
# Two properties everything downstream leans on:
#
#   * **stdout is candidates and nothing else.** Diagnostics go to stderr, so a caller can pipe this
#     straight into a comparison without filtering a banner out of it.
#   * **same tree, same bytes.** No clock, no network, no `find` ordering: the directories are
#     enumerated by a fixed list and the whole output goes through one `sort -u`.
#
# What it emits, one per line:
#
#   <key>                 a top-level vocabulary key of some protocols/*/*.yaml
#   artifacts/<family>    an adopter-writable document family under artifacts/
#
# A *bare* key and not `<file>:<key>`, because the same key declared by `aep/1` and extended by
# `adp/1` is one surface an adopter meets once, not two. The audit's row for it cites whichever
# document settles it.
#
# What it cannot emit is the whole point of the limit the audit states in its own words: a closed
# surface is precisely one with **no document key to find**, so nothing here will ever discover one.
# Those rows come from reading the corpus.
set -uo pipefail

ROOT="${1:-}"
if [ -z "$ROOT" ]; then
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi

if [ ! -d "$ROOT" ]; then
  printf 'scan-declarations: no such tree: %s\n' "$ROOT" >&2
  exit 1
fi

# Document metadata, not vocabulary. A key here declares what the document *is*; every other
# top-level key opening a block declares what may be written under it.
is_metadata() {
  case "$1" in
    id | version | title | summary | extends) return 0 ;;
    *) return 1 ;;
  esac
}

# The families an adopter may add a document to. A fixed list rather than a directory listing:
# `artifacts/README.md` is not a family, and enumerating would make the output depend on what
# somebody happened to drop in there.
FAMILIES=(kinds lifecycles relations templates)

{
  for doc in "$ROOT"/protocols/*/*.yaml; do
    [ -f "$doc" ] || continue
    # A top-level key whose value is a block: `capabilities:` and nothing after the colon. A key
    # with a scalar beside it (`version: 1`, `default_failure_policy: block`) declares a setting,
    # not a vocabulary, and an adopter cannot put a new value "in" it.
    while IFS= read -r key; do
      is_metadata "$key" && continue
      printf '%s\n' "$key"
    done < <(sed -n 's/^\([a-z_][a-z_]*\):[[:space:]]*$/\1/p' "$doc")
  done

  for family in "${FAMILIES[@]}"; do
    [ -d "$ROOT/artifacts/$family" ] && printf 'artifacts/%s\n' "$family"
  done
} | sort -u
