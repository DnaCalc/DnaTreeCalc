# Handover: Sibling-Offset Tail Value Uses Stale/Unexpected Published Value

Target repo: `OxCalc`

## Observation

During the workspace-document persistence tranche, the broad DnaTreeCalc host test run exposed an
unrelated active corpus failure:

```text
cargo test -p dnatreecalc-host -j 1
```

Failing test:

```text
src/dnatreecalc-host/tests/active_sibling_offsets_corpus.rs
active_sibling_offset_corpus_executes_through_direct_oxcalc_context
```

Case:

```text
ref-prev-tail
caller: Accounts.2005.Q2
reference: @PREV.Net
expected target: Accounts.2005.Q1.Net
expected published value: 7
observed published value: 100
```

The DnaTreeCalc test mutates `Accounts.2005.Q1.Net` to constant `7` before creating the direct
OxCalc context. The dependency membership assertion passes before the value assertion, so the
reference appears to bind to the intended target (`Accounts.2005.Q1.Net`). The published value
returned through the host projection is nevertheless `100`, which is the value of
`Accounts.2005.Q2.Net` in the fixture.

## Request

Please check the OxCalc/OxFml host-reference runtime path for sibling-offset references with a
tail (`@PREV.Net`, `Base.@PREV.Net`). Specifically, verify that the runtime value provider
dereferences the resolved tail target rather than the caller/current sibling context after bind.

## DnaTreeCalc Status

This is not caused by the workspace-document persistence changes. The storage tranche targeted
tests pass, and the broad host run reaches this existing corpus semantic mismatch after successful
compilation under `-j 1`.
