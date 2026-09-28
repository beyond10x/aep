# Direct-return producer fixtures

`direct-suite28.json` and `direct-suite29.json` are exact ESS CLI output from the adjacent
`direct-source.yaml` and `direct-scenario.yaml`. They were freshly produced by the ESS direct
library return extension over released 0.38.0; no version marker was relabeled.

From this directory, generate with the extension's ESS CLI:

```console
ess verify conform synthesize --path direct-source.yaml --scenarios direct-scenario.yaml --suite-format 4 --out direct-suite28.json
ess verify conform synthesize --path direct-source.yaml --scenarios direct-scenario.yaml --suite-format 5 --out direct-suite29.json
```

Both contain one generated and one authored scenario with zero refusals. The suite bytes and
model digests are producer output. Reports paired with them in tests are independently authored
reader fixtures, not claims of a real implementation run. The fixture deliberately uses only
the reader's supported scalar/newtype profile; broader ESS response types remain refused.
