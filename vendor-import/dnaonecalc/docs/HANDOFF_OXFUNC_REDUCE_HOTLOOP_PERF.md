*Posted by Codex agent on behalf of @govert*

# OxFunc Handoff: REDUCE / lambda-helper hot-loop allocator pressure

Status: **partial — lazy iteration landed (W095); small-row pool +
numeric-scalar specialisation still open**.
Direction: DnaOneCalc → OxFunc
Source repo / workset: DnaOneCalc / Performance investigation
Filed date: 2026-05-05
Landing-progress check: 2026-05-06

## Landing-progress check (2026-05-06)

OxFunc W095 + companion OxFml `invoke_many` shipped:
- `CallableInvocationBatch` / `CallableBatchMode` / `CallableInvoker::invoke_many`
  now exist on `oxfunc_core/src/functions/callable_helpers.rs`. The
  `materialize_iterable -> Vec<PreparedArgValue>` path is replaced by
  `PreparedIterableSource` walking the storage in place, and
  `eval_reduce_prepared` / `eval_scan_prepared` / `eval_byrow_prepared` /
  `eval_bycol_prepared` / `eval_map_prepared` route through
  `invoker.invoke_many(...)`. ➟ ASK A (lazy iteration) **DONE**.
- `OxFmlCallableInvoker::invoke_many` in
  `oxfml_core/src/eval/mod.rs` resolves the binding once per batch
  and reuses the closure / parameter slots / resolver across
  iterations, only falling back to per-call resolution for built-in
  callables. ➟ companion OxFml work **DONE**.

Re-measured Mandelbrot probe under
`cargo test --release -p dnaonecalc-host --test mandelbrot_perf_probe
-- --ignored --nocapture` on the same hardware as the original
filing:

| rows × cols × maxIter | inner iters | elapsed | per-inner-iter |
| --- | --- | --- | --- |
|   5 ×  5 ×  5 |     125 |    40.6 ms |  324.98 µs |
|  10 × 10 × 10 |   1 000 |   143.7 ms |  143.66 µs |
|  10 × 10 × 30 |   3 000 |   180.2 ms |   60.05 µs |
|  20 × 20 × 30 |  12 000 |   714.5 ms |   59.54 µs |
|  40 × 40 × 30 |  48 000 |    2.82 s |   58.77 µs |
|  50 × 30 × 30 |  45 000 |    2.65 s |   58.82 µs |
| 100 × 60 × 30 | 180 000 |   10.80 s |   59.99 µs |

Reading: per-iter cost asymptotes at ~60 µs. The lazy-iteration
landing flattened the small-N tail (5×5×5 dominated by setup ➟
wall clock, not per-iter), but the steady-state regime is still in
the 26–35 → 60 µs band. The original filing measured 26–35 µs per
inner iter; the current numbers are ~2× that. Two plausible
explanations:

1. Baseline machine variance (filing was on different hardware).
2. The remaining cost is dominated by the per-iter `EvalArray`
   1×3 row allocations and `INDEX(state, 1, k)` dispatch, neither
   of which W095 directly addresses.

Either way ASKS B and C (numeric-scalar specialisation; small-row
`EvalArray` inline-storage pool) remain open and are the next
likely wins. Filed status updated to "partial" on this basis.

## Landing-progress check (2026-05-07)

OxFunc W096 landed an architectural seam for compiled semantic
kernel dispatch (`FunctionCallTarget`, `FunctionExecutionContextBundle`,
`FunctionCallScratch`); OxFml W075 consumed the same seam for
compiled formula target planning. Together they replace the
string-based broad dispatch on the hot path with resolved
function-call target handles + reusable scratch buffers.

Re-measured Mandelbrot probe with the new upstream + the
host-side trace-mode opt-in:

| rows × cols × maxIter | per-inner-iter | wall |
| --- | --- | --- |
|   5 ×  5 ×  5 |  400.50 µs |   50 ms |
|  10 × 10 × 10 |  143.84 µs |  144 ms |
|  10 × 10 × 30 |   63.70 µs |  191 ms |
|  20 × 20 × 30 |   51.84 µs |  622 ms |
|  40 × 40 × 30 |   52.32 µs |  2.51 s |
|  50 × 30 × 30 |   48.54 µs |  2.18 s |
| 100 × 60 × 30 |   48.85 µs |  8.79 s |

Steady-state per-iter dropped to ~49 µs (from ~60 µs at the W094/
W095 floor). Asks B and C remain open and remain the most likely
next wins — the small-row `EvalArray` allocator pressure
inside `HSTACK` and the `prepared_from_array_cell` clone for
numeric `SEQUENCE`-shape iterables are still the dominant
allocation events on the trace.
Related:
  `OxFunc/crates/oxfunc_core/src/functions/callable_helpers.rs::eval_reduce_prepared`,
  `OxFunc/crates/oxfunc_core/src/functions/callable_helpers.rs::materialize_iterable`,
  `OxFml/crates/oxfml_core/src/eval/mod.rs::OxFmlCallableInvoker`,
  sibling: `docs/HANDOFF_OXFML_LAMBDA_INVOCATION_PERF.md`.

## Symptom

A power-user Mandelbrot formula evaluating a 100×60 grid with 30
iterations per cell runs in **6.4 s on a native release build** of
the host crate, ~13–25 s in wasm. The host bridge re-runs the full
formula on every editor event (keystroke, arrow, click), so any
non-trivial use of `REDUCE` / `MAP` / `SCAN` / `BYROW` / `BYCOL`
puts the editor into a multi-second hang.

The hot loop is small enough to characterise: REDUCE over a
`SEQUENCE(maxIter)`, accumulator is a 1×3 row carrying `(x, y, n)`,
body destructures via three `INDEX` calls and re-packs via
`HSTACK`. End-to-end **~26–35 µs per inner iteration** (release).
For comparison a tight Rust loop running the same arithmetic is
~10 ns per iter — **2 600× faster**. The arithmetic is trivial; the
overhead is the function-dispatch and small-array allocation
machinery around it.

## Root causes inside OxFunc

The Mandelbrot probe lives in
`DnaOneCalc/src/dnaonecalc-host/tests/mandelbrot_perf_probe.rs`
(`#[ignore]`-gated; run with `cargo test --release -p
dnaonecalc-host --test mandelbrot_perf_probe -- --ignored
--nocapture`). The numbers below are reproducible on a current
native build.

### 1. `materialize_iterable` allocates the entire iterable upfront

```rust
fn materialize_iterable(prepared: &PreparedArgValue) -> Vec<PreparedArgValue> {
    match prepared {
        PreparedArgValue::Eval(EvalValue::Array(array)) => array
            .iter_row_major()
            .map(prepared_from_array_cell)
            .collect(),
        other => vec![other.clone()],
    }
}
```

`eval_reduce_prepared`, `eval_scan_prepared`, `eval_byrow_prepared`,
`eval_bycol_prepared`, `eval_map_prepared` all consume this `Vec`.
For the Mandelbrot case the iterable is `SEQUENCE(30)` per cell,
6 000 cells → 180 000 `prepared_from_array_cell` clones, each one
heap-allocating a `PreparedArgValue::Eval(EvalValue::Number(..))`
that lives just long enough for one lambda invocation.

### 2. `prepared_from_array_cell` clones every cell

```rust
fn prepared_from_array_cell(cell: &ArrayCellValue) -> PreparedArgValue {
    match cell {
        ArrayCellValue::Number(n) => PreparedArgValue::Eval(EvalValue::Number(*n)),
        ArrayCellValue::Text(t)   => PreparedArgValue::Eval(EvalValue::Text(t.clone())),
        ArrayCellValue::Logical(b)=> PreparedArgValue::Eval(EvalValue::Logical(*b)),
        ArrayCellValue::Error(c)  => PreparedArgValue::Eval(EvalValue::Error(*c)),
        ArrayCellValue::EmptyCell => PreparedArgValue::EmptyCell,
    }
}
```

For numeric iterables (the common case for `REDUCE` over
`SEQUENCE`), this is a direct copy of an `f64` wrapped in a 24-byte
enum value — fine for one cell, painful when materialised 180 000
times into a `Vec` that is then iterated and discarded.

### 3. The accumulator round-trips a 1×3 array per iter

The host-side workload uses `HSTACK(x, y, n)` as a 3-tuple. Each
REDUCE iteration:

1. Receives `accumulator: PreparedArgValue::Eval(EvalValue::Array(1×3))`.
2. Lambda body destructures via three separate `INDEX(state, 1, n)`
   function dispatches.
3. Re-packs via `HSTACK(...)` → freshly allocated 1×3 `EvalArray`
   (`EvalArray::from_rows(vec![row.to_vec()])`).

`EvalArray` is heap-backed; 30 × 6 000 = 180 000 fresh-vec
allocations per Mandelbrot pass. None of this is OxFunc's fault per
se — the user wrote `HSTACK` in the inner loop — but the *shape*
of the allocation pattern is the cost driver.

### 4. No early-exit / fast-path for `REDUCE` on `SEQUENCE`

`REDUCE` always iterates the full materialised list, even when the
body is an `IF(escaped, state, …)` no-op. Excel's REDUCE has the
same semantics, so this is not strictly a bug — but it does mean
the engine pays the dispatch cost per iter for cells that escaped
early.

## Concrete asks

### A. Lazy iteration in the lambda helpers

Replace `materialize_iterable -> Vec<PreparedArgValue>` with an
iterator-shaped abstraction:

```rust
pub trait LambdaIterableSource {
    fn next_prepared(&mut self) -> Option<PreparedArgValue>;
    fn shape_hint(&self) -> Option<ArrayShape>;
}
```

with implementations for `EvalValue::Array` (zero-copy walking the
existing storage) and `EvalValue::Number / Text / Logical /
Error / Reference` (single-shot). Each helper
(`eval_reduce_prepared`, `eval_scan_prepared`, etc.) iterates this
source instead of holding a fully-materialised `Vec`. The
`Vec::with_capacity(values.len())` patterns in `eval_scan_prepared`
/ `eval_byrow_prepared` switch to `shape_hint().rows * cols` when
available.

For `REDUCE`, this drops 180 000 `PreparedArgValue` heap
allocations on the Mandelbrot probe.

### B. Specialise `eval_reduce_prepared` for numeric scalar iterables

Common case: `REDUCE(initial, SEQUENCE(N), LAMBDA(...))`. The
lambda is invoked with `accumulator` and a fresh `Number(k)` per
step. Cache the lambda binding once per call (so the OxFml-side
invoker doesn't re-resolve the callable token on every step — see
the sibling OxFml handoff) and, when the iterable's element kind
is statically known to be `ArrayCellValue::Number`, skip the
generic `prepared_from_array_cell` clone and pass `EvalValue::Number(value)`
directly.

### C. Pool the small-row `EvalArray` allocations

`HSTACK(scalar, scalar, scalar)` allocates a fresh
`Vec<Vec<ArrayCellValue>>` for the rows + a `Vec<ArrayCellValue>`
for the row cells. For the very common case of small-row
accumulators (1×N rows with N ≤ 8), introduce an inline-storage
variant:

```rust
enum EvalArrayStorage {
    Inline { rows: u8, cols: u8, cells: SmallVec<[ArrayCellValue; 8]> },
    Heap { rows: usize, cols: usize, cells: Vec<ArrayCellValue> },
}
```

The hot loop's 1×3 accumulator stays on the stack. Same shape; no
public-API change.

This is the single largest win: it removes the per-iter heap
allocation entirely for the Mandelbrot probe and any similar
"3-tuple-as-row" hot loops.

### D. Optional: `REDUCE` early-exit hint

Out of scope for Excel parity but helpful for power users: an
opt-in `REDUCE.STOP(value)` sentinel that lets the body
short-circuit. Not required for this perf issue; flag separately.

## Suggested test corpus

`OxFunc/tests/lambda_helper_perf_tests.rs` (new):

1. `reduce_over_sequence_does_not_materialise_full_iterable` —
   construct a `MockIterable` that panics if more than `chunk_size`
   items are realised concurrently; assert REDUCE consumes them
   one-at-a-time.
2. `reduce_with_numeric_scalar_iterable_skips_per_cell_clone` —
   use a counter-instrumented `prepared_from_array_cell` substitute
   to assert it is called at most `iterable.len() / cache_factor`
   times after the specialisation lands.
3. `eval_reduce_prepared_round_trips_1x3_accumulator_without_heap_growth`
   — run 1 000 iterations of a Mandelbrot-shape body, assert
   `EvalArray`'s heap allocator does not exceed a small bound.

## DnaOneCalc-side impact

Once (A) + (C) land the host probe should drop from ~6.4 s to
roughly 1–2 s on the same hardware (most of the remaining cost
moves to OxFml's `evaluate_expr_value` per-call overhead — see
sibling handoff). Then a host-side debounced-runtime split (already
scoped, not yet built) closes the user-visible gap on typing.

No host SEAM is needed today; the workload runs to completion, just
slowly.
