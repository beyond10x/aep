---
title: Artifacts, kinds and relations
sidebar_label: Artifacts and kinds
sidebar_position: 2
description: What an artifact is, the kinds AEP knows and the ones you add, the relation vocabulary, blockers, and which fields are yours to edit.
---

# Artifacts, kinds and relations

An **artifact** is one item of the plan: a vision, an epic, a story, a task, a design, a review, a
blocker. It is one Markdown file with YAML front matter, and its id says where the file lives:

```text
story:pay-by-card   →   .engineering/planning/story/pay-by-card.md
```

## Kinds

The kind is the part of the id before the colon. `aep plan artifact kinds` lists what can be created
in this project:

```shell-session
$ aep plan artifact kinds
vision                           planning
product-requirements             planning
initiative                       planning
epic                             planning
story                            planning
task                             planning
specification                    output
…
review-result                    output
…
executable-system-specification  output
blocker                          planning  (this store's lifecycles declare it)
obligation                       output    (this store's lifecycles declare it)
outbound-claim                   output    (this store's lifecycles declare it)
<type>-blocker                   planning  (open family: credential-blocker, decision-blocker, …)
```

*Planning* kinds describe work to be done. *Output* kinds are what work produces. The list merges
three sources: the kinds compiled into AEP, every kind your document tree declares a lifecycle for,
and the open `<type>-blocker` family.

A kind is open to authors but not to typos. Any kebab-case name is a valid kind, and a kind with no
lifecycle of its own takes the lifecycle of the kind it specialises. The parent is named by the last
hyphen segment, so `weekly-digest` is a `digest`. A **status** is not open in the same way. Only
the statuses the kind's lifecycle declares are accepted, and `validate` reports any other.

Some kinds have aliases: `new adr …` creates an `architecture-decision-record`.

## Relations

Edges between artifacts come from a fixed vocabulary. `aep plan artifact relations` prints it:

```shell-session
$ aep plan artifact relations
informed_by   Shaped by, without being derived from — read and taken into account.
derived_from  Produced from a higher-level artifact, whose intent it carries down.
decomposes    Breaks a larger artifact into smaller work.
specifies     States the required behaviour of something.
designs       Proposes how to satisfy something.
implements    Realises something in the system.
decides       Records a decision taken within something.
reviews       Assesses something.
verifies      Establishes that something holds.
blocks        Prevents progress on something.
depends_on    Needs something else first.
supersedes    Replaces something, which becomes superseded.
delivers      Produces the outcome something asked for.
serves        Moves an objective the collection has set — a `vision` artifact — and says which.
```

An edge is written on the artifact it starts at. You can add one at creation
(`new … --relate decomposes:epic:guest-checkout`), later (`relate story:x depends_on story:y`), and
take it back (`unrelate`). `relate` refuses an edge whose target the plan does not hold, and
`validate` reports a dangling edge, a cycle and a duplicate id.

Four relations are read by other commands:

| Relation | Read by | Effect |
|---|---|---|
| `blocks` | `list`, `board`, `blocked` | the target is marked `blocked: <type>` until the blocker reaches the end of its own lifecycle |
| `depends_on` | `waves` | a story is never placed in the same wave as something it depends on, or in an earlier one |
| `reviews` | `findings`, `review-value`, `evidence --kind review_outcome` | joins a `review-result` to the artifact it reviews |
| `specifies` | `aep observe specification evidence` | finds the approved specification that applies to a task's work |

## Blockers

A blocker is an artifact of kind `<type>-blocker`: `decision-blocker`, `credential-blocker`, or a
type of your own such as `procurement-blocker`. The type names what would clear it. Its lifecycle is
`open → cleared`:

```shell-session
$ aep plan artifact new decision-blocker card-vault-provider \
    --title "Which card vault do we use?" --relate blocks:story:save-card
created decision-blocker:card-vault-provider (open) at …/planning/decision-blocker/card-vault-provider.md
$ aep plan artifact blocked
decision-blocker:card-vault-provider  decision  open  Which card vault do we use?
  blocks story:save-card  draft  Save a card for next time
$ aep plan artifact list --kind story
story:guest-receipt  story  draft        Email a receipt to a guest
story:pay-by-card    story  implemented  Pay by card as a guest
story:refund-guest   story  draft        Refund a guest order
story:save-card      story  draft        Save a card for next time   blocked: decision
```

To unblock the story, move the blocker to `cleared`. That is a move, not an edit, so the plan keeps
the record that the story was ever stuck. `--withholds <evidence-kind>` on a blocker records which
evidence it is stopping somebody from producing. `explain` shows that on the blocked artifact.

## Who writes which field

| Field | Written by | Notes |
|---|---|---|
| `format`, `id`, `kind` | the CLI at `new` | identity, fixed for the artifact's life |
| `status`, `transitions` | `move` only | a move appends one transition; `set --status` is refused by name |
| `revision` | every CLI write | one more per write to the file |
| `title`, `summary`, `owner`, `tags`, `refs` | `new`, `set` | or a hand edit |
| `relations` | `new --relate`, `relate`, `unrelate` | or a hand edit; `validate` checks the result |
| `scope` | `scope` | stories only; read by `waves` |
| `withholds` | `new --withholds` | on a blocker |
| `model_digest` | `set --model-digest` | executable system specifications only |
| body | `new --from`, `body` | or a hand edit, except on immutable kinds such as `review-result` |

The full format, with an annotated example, is in [the artifact file](../reference/artifact-file.md).

## External references

`--ref <provider>:<key>` records the same work in another system, such as `jira:DEV-630`.
`list --ref jira:DEV-630` finds it again. When `project.yaml` sets a URL pattern under `providers:`
(`jira: https://tracker.example/browse/{key}`), `show` prints the link beside the reference.
