# Conformance

Language-neutral fixtures, scenarios and expected results, so a backend in any language can prove it
implements the contract rather than merely compiling against it.

| Directory | Contents |
|---|---|
| `fixtures/` | input documents and entity graphs |
| `scenarios/` | ordered command/query steps, including the §104 end-to-end scenario |
| `expected/` | the observable results a conforming backend must produce |
| `trace/` | the shipped `trace-spec/1` documents — what a run of the plugin, a driven step and a denied step must have looked like |
| `eval/` | the eval-case corpus: a task statement, a `trace-spec/1` document and a committed transcript per case, replayed in the gate. See [`eval/README.md`](./eval/README.md) |

Scenarios must not depend on sleeps: ordering is established with consistency tokens.

The last two rows are a second and third tenant rather than a widening of the first. `fixtures/`,
`scenarios/` and `expected/` judge a **backend** against the command and query contract; `trace/` and
`eval/` judge an **agent run** against an authored expectation document. What the three share is the
thing this directory is for: the material a claim is decided against, kept apart from the code that
decides it.
