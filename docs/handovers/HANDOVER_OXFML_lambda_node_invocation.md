# HANDOVER_OXFML_lambda_node_invocation

Status: Open
Target: OxFml
Ask: Support **invoking a lambda-valued node through a tree-path-resolved reference plus call syntax** — `=Doubler(5)`, `=My.brother.node(1, 2, "A")` — Excel-aligned with how a defined name that holds a `LAMBDA` is invoked. Confirm the bind layer accepts a call applied to a tree-reference carrier under `treecalc-v1`.
Context: It is a core Excel premise that a defined name can store an executable lambda from a `LAMBDA` formula and be invoked by name. TreeCalc generalizes this to nodes: any reference that resolves to a lambda-valued node should be callable with argument syntax (CORE_MODEL_SPEC §3.8). LAMBDA invocation semantics are OxFml's; the novel surface is the *tree-path-as-callable* form.
Evidence: CORE_MODEL_SPEC §3.8 (node-as-function), §3 reference grammar, §4 capability profile, §6 values (a node value can be `Lambda`).

## What TreeCalc needs

1. **Grammar.** A call tail on a resolved path: `Path(args)` where `Path` binds to a node by the normal walk-up / anchored rules, then the resolved value is invoked. Confirm where this sits relative to the structured-ref `[...]` tail and the `@`-accessor family.
2. **Bind.** Accept a call on a tree-reference carrier (single-reference resolutions only); a call on a set-producing reference is an error unless explicitly designed.
3. **Dependency.** The caller depends on the lambda node's value (a change to the `LAMBDA` re-binds/re-evaluates callers).
4. **Profile gating.** The path-callable surface is `treecalc-v1`; under `strict-excel` only Excel's own defined-name LAMBDA invocation applies.
5. **Errors.** Result kind when the resolved node is not a lambda, or arity/type mismatch — Excel-aligned (`#VALUE!` / `#CALC!` as appropriate, owned by OxFunc).

## Expected disposition

Part **confirm** (LAMBDA invocation already exists in OxFml; the question is accepting a tree-reference carrier as the callee), part **coordinate** (grammar placement of the call tail and the single-vs-set rule).
