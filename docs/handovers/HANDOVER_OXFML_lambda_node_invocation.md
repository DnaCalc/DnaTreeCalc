# HANDOVER_OXFML_lambda_node_invocation

Status: Open
Target: OxFml
Ask: Support **invoking a lambda-valued node through a host-reference-resolved carrier plus call syntax** — `=Doubler(5)`, `=My.brother.node(1, 2, "A")` — Excel-aligned with how a defined name that holds a `LAMBDA` is invoked. Confirm the bind layer accepts a call applied to an opaque host-reference carrier supplied through an OxCalc host formula context whose active capability profile is TreeCalc.
Context: It is a core Excel premise that a defined name can store an executable lambda from a `LAMBDA` formula and be invoked by name. TreeCalc generalizes this to nodes: any reference that resolves to a lambda-valued node should be callable with argument syntax (CORE_MODEL_SPEC §3.8). LAMBDA invocation semantics are OxFml's; the novel surface is the *tree-path-as-callable* form.
Evidence: CORE_MODEL_SPEC §3.8 (node-as-function), §3 reference grammar, §4 capability profile, §6 values (a node value can be `Lambda`).

## What TreeCalc needs

1. **Generic host context.** OxFml should not grow a built-in TreeCalc parser mode. It should accept an OxCalc-supplied host formula context with a dialect/profile id, reference-expression hook, host namespace resolver, caller context, and function-registry view.
2. **Grammar.** A call tail on a resolved host path: `Path(args)` where `Path` binds through the host reference/name hook, then the resolved value is invoked. Confirm where this sits relative to the structured-ref `[...]` tail and the `@`-accessor family.
3. **Bind.** Accept a call on a host-reference carrier (single-reference resolutions only); a call on a set-producing reference is an error unless explicitly designed.
4. **Name/call resolution.** OxFml-owned special forms and lexical bindings keep their own precedence, but exact shadowing across built-in functions, registered UDFs, workbook/sheet defined names, and defined-name `LAMBDA` invocation must match observed Excel behavior before product semantics are frozen. TreeCalc host names/lambda-valued nodes should map onto the closest Excel-defined-name lane or be documented as an explicit TreeCalc extension. Non-call bare names and explicit host paths must produce replay-visible resolution-layer diagnostics.
5. **Dependency.** The caller depends on the lambda node's value (a change to the `LAMBDA` re-binds/re-evaluates callers).
6. **Profile gating.** The path-callable surface is admitted only when the active OxCalc host formula context exposes the TreeCalc capability profile; under `strict-excel` only Excel's own defined-name LAMBDA invocation applies.
7. **Errors.** Result kind when the resolved node is not a lambda, or arity/type mismatch — Excel-aligned (`#VALUE!` / `#CALC!` as appropriate, owned by OxFunc).

## Expected disposition

Part **confirm** (LAMBDA invocation already exists in OxFml; the question is accepting an opaque host-reference carrier as the callee), part **coordinate** (generic host-context interface, grammar placement of the call tail, Excel-matched function/UDF/defined-name-lambda shadowing, and the single-vs-set rule).
