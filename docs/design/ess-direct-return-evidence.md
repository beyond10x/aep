# ESS direct-return evidence

The optional `aep-ess-evidence` reader extends the existing standalone report/2 contract with
ordinary `ess-conformance/28` and inventory `ess-conformance/29`. These versions carry the
`expect_direct_response` step produced from `ess/17` and authored `ess-scenario/4` documents.
Previously supported suites/1–5 retain their admission and parent-comparison behavior. Versions
6–27 are not reinterpreted as direct-return documents; their features remain outside this reader.

The supported response types are `String`, `Boolean`, `Integer`, `Optional<String>`,
`Optional<List<String>>` and transparent nominal `newtype` declarations ending in one of those
types. Integers must have exact mathematical values within `i64::MIN..=u64::MAX`. Integral decimal
and exponent spellings such as `1.0` and `100e-2` are checked by decimal shifting, never by
conversion to binary64. Exponents must fit an `i64`. Original numeric spellings remain intact. Strings
may carry lossless JSON documents; their contents remain strings, not a weaker substitute for an
unsupported ESS `Json` declaration. Optional strings admit null or a string; optional string lists
admit null or an ordered array of strings, retaining duplicates. Decimal, Binary64, Json, other
collection/optional types, structs, enums and unions are explicitly refused. This is the profile needed by Entity Runtime's
scalar observations and document carriers, not a claim of general ESS type support.

The new step has closed `command`, `outcome`, `fields`, `declarations` and `expected` members.
It must refer to the preceding command invocation. A non-null outcome must belong to that
command. Fields are unique; every nominal declaration must be reachable, resolved and acyclic.
Expected literals may name a subset of declared fields, and each named literal must match its
type. The reader checks declaration authority, not a runtime return that only the ESS runner can
observe. Passing reports are descriptive evidence, not proof that their producer was honest.

Resource bounds are explicit: nonempty response fields with at most 256 entries, at most 4096
declarations, at most 128 nominal aliases on a resolution path, and at most 1 MiB of original
JSON for each complete response authority. String lists have at most 65,536 entries. The shared parser continues to reject duplicate keys
and nesting beyond 128. The byte bound counts original whitespace and escapes as well as values;
it is intentionally narrower than ESS's general typed value budget.

Inventory suite/29 uses the same report counts, outcome partition, exact model digest, complete
inventory and parent-chain checks as suite/5. SHA256 refers to the exact original UTF-8 suite
bytes, including whitespace. Modern parent comparisons preserve JSON scalar lexemes, decoded
string values, array order and all object fields. They do not normalize omitted fields or number
spellings: a filtered child must preserve its parent's complete admitted scenario definition.
Suite/5 keeps its existing ESS default normalization. A complete passing ordinary suite/28
remains count evidence with unknown coverage; it cannot satisfy complete-inventory requirements.

`aep plan artifact evidence --from REPORT --suite SUITE` wraps an unfiltered suite/29 without
rewriting it. Filtered suites require `--suite-input` and the exact original parent chain. Both
routes invoke the coverage reader before recording descriptive planning history.

Evidence source archives retain the existing `report_json` and `suite_input_json` strings; no new
archive kind or schema is introduced. AEP 0.63.1 can deserialize those source strings and read
descriptive planning history. Its old reader cannot re-admit suite/29, and its checked task suite
references reject suite/29. Admission, engine replay and typed expectations therefore require a
build containing this reader extension. Persisted admission is never trusted: deserialization
clears it, as before.

The retained producer fixtures include their source documents and exact emitted suites. Their
test reports are independently authored reader inputs and are not execution evidence. Tests cover
wrong suite/model identities, contradictory counts, missing and extra selected scenarios,
malformed inventory, unknown members, unresolved declarations, resource limits, legacy refusal
of the new step, and exact parent comparison above binary64's exact-integer range.
