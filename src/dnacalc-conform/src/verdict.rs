//! The verdict ladder — provenance first, always — and the value comparator
//! it ends in.
//!
//! ```text
//! 0. the workbook did not open                     -> NotEvaluated { open_failed }
//! 1. provenance of the published cell:
//!      none published                              -> NotEvaluated { not_published }
//!      FileCached (refined by the load report)     -> NotEvaluated { bind_degraded |
//!                                                     external_reference_pinned |
//!                                                     not_calc_modeled | manual_mode |
//!                                                     file_cached }
//!      Degraded                                    -> NotEvaluated { degraded }
//!      Stale                                       -> NotEvaluated { stale }
//! 2. Calculated but the formula calls NOW/TODAY/RAND/RANDBETWEEN/RANDARRAY,
//!    or the file marks the cell ca="1"             -> Excluded { nondeterministic }
//! 3. Calculated but the file has no scorable cache -> Excluded { no_file_cache |
//!                                                     oracle_unknown_error_code |
//!                                                     oracle_unsupported_payload }
//! 4. Calculated with a cache                        -> Match | Mismatch { class }
//! ```
//!
//! Only rung 4 ever produces `Match`. A `FileCached` value that equals
//! Excel's cache is the false green (`dnacalc-host-core/src/xlsx_false_green.rs`)
//! and lands on rung 1 by construction.
//!
//! Every class has a one-line definition in the crate README; `Unclassified`
//! must be zero and is never widened silently.

use oxcalc_core::workbook_settings::PublishedValueProvenance;
use oxfunc_core::value::{CalcValue, CoreValue, WorksheetErrorCode};

use crate::oracle::OracleValueGap;

/// Why a formula cell was never evaluated by the engine — so it can never
/// be a `Match`, whatever its published value says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotEvaluatedReason {
    /// `WorkbookSession::open_xlsx_bytes` returned an error; `kind` is the
    /// typed error's variant path (e.g. `oxcalc:GridEngine:InvalidDefinedName`).
    OpenFailed { kind: String },
    /// The oracle's sheet has no engine sheet of the same name.
    SheetUnmatched,
    /// The session refused a read the ladder needs (`sheets()` or
    /// `grid_view()` returned an error); `kind` is the typed error's variant
    /// path. An internal invariant of the session, surfaced as a row rather
    /// than a panic on a corpus run.
    EngineReadFailed { kind: String },
    /// The engine published nothing at the address (a cache-less
    /// `NotCalcModeled` or unbacked shared-region cell publishes nothing).
    NotPublished,
    /// `FileCached` and the load report's `bind_degradations` names the
    /// address: OxFml rejected the text, the engine pinned the file's cache.
    BindDegraded,
    /// `FileCached` and the load report's `external_reference_pins` names the
    /// address: the formula references another workbook and is pinned.
    ExternalReferencePinned,
    /// `FileCached` and the file's `FormulaTopology` record is `DataTable` or
    /// an unknown `t=`: OxCalc's ingest never binds it (`not_calc_modeled`).
    NotCalcModeled,
    /// `FileCached` on a `calcMode="manual"` workbook with no recalc forced:
    /// the load ran zero engine passes (the whole-file trap).
    ManualMode,
    /// `FileCached` with no more specific explanation from the load report.
    FileCached,
    /// `Degraded`: a rejected formula with no file cache; the engine
    /// published a substitute `#NAME?`.
    Degraded,
    /// `Stale`: calculated once, then invalidated by an undrained edit.
    Stale,
}

impl NotEvaluatedReason {
    /// The artifact key (`not_evaluated` map).
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::OpenFailed { kind } => format!("open_failed:{kind}"),
            Self::SheetUnmatched => "sheet_unmatched".to_string(),
            Self::EngineReadFailed { kind } => format!("engine_read_failed:{kind}"),
            Self::NotPublished => "not_published".to_string(),
            Self::BindDegraded => "bind_degraded".to_string(),
            Self::ExternalReferencePinned => "external_reference_pinned".to_string(),
            Self::NotCalcModeled => "not_calc_modeled".to_string(),
            Self::ManualMode => "manual_mode".to_string(),
            Self::FileCached => "file_cached".to_string(),
            Self::Degraded => "degraded".to_string(),
            Self::Stale => "stale".to_string(),
        }
    }
}

/// Why an engine-`Calculated` cell is not scored against the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcludedReason {
    /// The formula calls a function whose result is not a function of the
    /// workbook, or the file marks the cell `ca="1"` (Excel's own
    /// always-calculate flag, which it sets on every cell in a volatile
    /// closure — `INDIRECT`/`OFFSET` users and their dependents included).
    Nondeterministic {
        function: Option<&'static str>,
        ca_marker: bool,
    },
    /// The file carries no `<v>` for the cell (also how OxDoc reports an
    /// empty-string result).
    NoFileCache,
    /// The cache is an error literal OxDoc could not decode (byte 255).
    OracleUnknownErrorCode { byte: u8 },
    /// The cache payload is not a value OxDoc can hand over as one.
    OracleUnsupportedPayload { detail: String },
}

impl ExcludedReason {
    /// The artifact key (`excluded` map). A nondeterministic exclusion names
    /// its trigger — the function, or `ca=1` when only the file's flag
    /// fired — so the two sources stay distinguishable in the totals.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Nondeterministic {
                function: Some(function),
                ..
            } => format!("nondeterministic:{function}"),
            Self::Nondeterministic { function: None, .. } => "nondeterministic:ca=1".to_string(),
            Self::NoFileCache => "no_file_cache".to_string(),
            Self::OracleUnknownErrorCode { .. } => "oracle_unknown_error_code".to_string(),
            Self::OracleUnsupportedPayload { detail } => {
                format!("oracle_unsupported_payload:{detail}")
            }
        }
    }
}

impl From<OracleValueGap> for ExcludedReason {
    fn from(gap: OracleValueGap) -> Self {
        match gap {
            OracleValueGap::UnknownErrorByte(byte) => Self::OracleUnknownErrorCode { byte },
            OracleValueGap::SharedStringOutOfRange(index) => Self::OracleUnsupportedPayload {
                detail: format!("shared_string_out_of_range:{index}"),
            },
            OracleValueGap::UnsupportedPayload(detail) => Self::OracleUnsupportedPayload {
                detail: detail.to_string(),
            },
        }
    }
}

/// How an engine-`Calculated` value differs from Excel's cache. Definitions
/// (one line each, mirrored in the README):
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MismatchClass {
    /// Both numbers, not equal, within 4 ULP (`distance` is the ULP count).
    Ulp { distance: u64 },
    /// Both numbers, more than 4 ULP apart, equal at 15 significant digits.
    SignificantDigits15,
    /// Both numbers, differing beyond 15 significant digits (or an engine
    /// non-finite value).
    Number,
    /// Both text, differing in at least one UTF-16 code unit.
    Text,
    /// Both booleans, differing.
    Logical,
    /// Both errors, differing in code.
    ErrorCode {
        engine: WorksheetErrorCode,
        excel: WorksheetErrorCode,
    },
    /// The engine produced an error where Excel cached a non-error value.
    EngineError { code: WorksheetErrorCode },
    /// Excel cached an error where the engine produced a non-error value.
    OracleError { code: WorksheetErrorCode },
    /// Different scalar kinds, neither an error (e.g. engine `Empty`, Excel
    /// `Number`).
    Type { engine: String, excel: String },
    /// An engine value shape the classifier does not model (array,
    /// reference, missing). Must be zero; widening requires a named bead.
    Unclassified { engine: String },
}

impl MismatchClass {
    /// The artifact key (`mismatch` map): the class name, with the codes /
    /// kinds that make the row actionable.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Ulp { .. } => "Ulp".to_string(),
            Self::SignificantDigits15 => "SignificantDigits15".to_string(),
            Self::Number => "Number".to_string(),
            Self::Text => "Text".to_string(),
            Self::Logical => "Logical".to_string(),
            Self::ErrorCode { engine, excel } => format!("ErrorCode:{engine:?}->{excel:?}"),
            Self::EngineError { code } => format!("EngineError:{code:?}"),
            Self::OracleError { code } => format!("OracleError:{code:?}"),
            Self::Type { engine, excel } => format!("Type:{engine}->{excel}"),
            Self::Unclassified { engine } => format!("Unclassified:{engine}"),
        }
    }

    #[must_use]
    pub fn is_unclassified(&self) -> bool {
        matches!(self, Self::Unclassified { .. })
    }
}

/// The verdict for one formula cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    NotEvaluated(NotEvaluatedReason),
    Excluded(ExcludedReason),
    Match,
    Mismatch(MismatchClass),
}

impl Verdict {
    /// The artifact's per-cell verdict string: `not_evaluated:<reason>`,
    /// `excluded:<reason>`, `match`, `mismatch:<class>`.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::NotEvaluated(reason) => format!("not_evaluated:{}", reason.key()),
            Self::Excluded(reason) => format!("excluded:{}", reason.key()),
            Self::Match => "match".to_string(),
            Self::Mismatch(class) => format!("mismatch:{}", class.key()),
        }
    }
}

/// Maximum ULP distance still classified `Ulp` (beyond it the pair is
/// `SignificantDigits15` or `Number`).
pub const ULP_TOLERANCE: u64 = 4;

/// Rung 4: compare an engine-`Calculated` value with Excel's cached value.
/// Both are typed `CalcValue`s; only `core` is compared (a rich payload on
/// the engine side does not change what the cell's value is).
#[must_use]
pub fn compare(engine: &CalcValue, excel: &CalcValue) -> Verdict {
    match (engine.core(), excel.core()) {
        // Shapes the classifier does not model come first, so they are
        // never folded into an error/type class and stay loud.
        (CoreValue::Array(_) | CoreValue::Reference(_) | CoreValue::Missing, _) => {
            Verdict::Mismatch(MismatchClass::Unclassified {
                engine: kind_name(engine.core()).to_string(),
            })
        }
        (CoreValue::Error(e), CoreValue::Error(x)) => {
            if e == x {
                Verdict::Match
            } else {
                Verdict::Mismatch(MismatchClass::ErrorCode {
                    engine: *e,
                    excel: *x,
                })
            }
        }
        (CoreValue::Error(code), _) => {
            Verdict::Mismatch(MismatchClass::EngineError { code: *code })
        }
        (_, CoreValue::Error(code)) => {
            Verdict::Mismatch(MismatchClass::OracleError { code: *code })
        }
        (CoreValue::Number(a), CoreValue::Number(b)) => compare_numbers(*a, *b),
        (CoreValue::Text(a), CoreValue::Text(b)) => {
            if a == b {
                Verdict::Match
            } else {
                Verdict::Mismatch(MismatchClass::Text)
            }
        }
        (CoreValue::Logical(a), CoreValue::Logical(b)) => {
            if a == b {
                Verdict::Match
            } else {
                Verdict::Mismatch(MismatchClass::Logical)
            }
        }
        (engine_core, excel_core) => Verdict::Mismatch(MismatchClass::Type {
            engine: kind_name(engine_core).to_string(),
            excel: kind_name(excel_core).to_string(),
        }),
    }
}

/// The numeric rules: exact bits (or both zero — Excel has no negative
/// zero) is a `Match`; then 4 ULP; then 15 significant digits; then
/// `Number`.
fn compare_numbers(engine: f64, excel: f64) -> Verdict {
    if engine.to_bits() == excel.to_bits() || (engine == 0.0 && excel == 0.0) {
        return Verdict::Match;
    }
    if !engine.is_finite() || !excel.is_finite() {
        return Verdict::Mismatch(MismatchClass::Number);
    }
    let distance = ulp_distance(engine, excel);
    if distance <= ULP_TOLERANCE {
        return Verdict::Mismatch(MismatchClass::Ulp { distance });
    }
    if significant_15(engine) == significant_15(excel) {
        return Verdict::Mismatch(MismatchClass::SignificantDigits15);
    }
    Verdict::Mismatch(MismatchClass::Number)
}

/// The number rounded to 15 significant digits, as text (`{:.14e}` is 15
/// significant digits, correctly rounded).
#[must_use]
pub fn significant_15(value: f64) -> String {
    format!("{value:.14e}")
}

/// Number of representable doubles between `a` and `b` (both finite), via
/// the sign-magnitude to two's-complement mapping that makes the IEEE bit
/// patterns totally ordered.
#[must_use]
pub fn ulp_distance(a: f64, b: f64) -> u64 {
    fn ordered(x: f64) -> i64 {
        let bits = x.to_bits() as i64;
        if bits < 0 { i64::MIN - bits } else { bits }
    }
    (i128::from(ordered(a)) - i128::from(ordered(b))).unsigned_abs() as u64
}

/// The kind name used in `Type` / `Unclassified` keys.
#[must_use]
pub fn kind_name(core: &CoreValue) -> &'static str {
    match core {
        CoreValue::Number(_) => "Number",
        CoreValue::Text(_) => "Text",
        CoreValue::Logical(_) => "Logical",
        CoreValue::Error(_) => "Error",
        CoreValue::Empty => "Empty",
        CoreValue::Missing => "Missing",
        CoreValue::Array(_) => "Array",
        CoreValue::Reference(_) => "Reference",
    }
}

/// The `not_evaluated` reason for a `FileCached` / `Degraded` / `Stale`
/// provenance, refined by what the load report knows about the address.
#[must_use]
pub fn not_evaluated_reason(
    provenance: PublishedValueProvenance,
    facts: &PinFacts,
) -> Option<NotEvaluatedReason> {
    match provenance {
        PublishedValueProvenance::Calculated { .. } => None,
        PublishedValueProvenance::Stale { .. } => Some(NotEvaluatedReason::Stale),
        PublishedValueProvenance::Degraded => Some(NotEvaluatedReason::Degraded),
        PublishedValueProvenance::FileCached => Some(if facts.bind_degraded {
            NotEvaluatedReason::BindDegraded
        } else if facts.external_reference_pinned {
            NotEvaluatedReason::ExternalReferencePinned
        } else if facts.not_calc_modeled_kind {
            NotEvaluatedReason::NotCalcModeled
        } else if facts.manual_load_undrained {
            NotEvaluatedReason::ManualMode
        } else {
            NotEvaluatedReason::FileCached
        }),
    }
}

/// What the load report and the file say about one pinned address — the
/// inputs to the `FileCached` refinement, in precedence order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PinFacts {
    /// `WorkbookLoadReport::bind_degradations` names the address.
    pub bind_degraded: bool,
    /// `WorkbookLoadReport::external_reference_pins` names the address.
    pub external_reference_pinned: bool,
    /// The file's topology record for the cell is `DataTable` / `Unknown`.
    pub not_calc_modeled_kind: bool,
    /// The load ran the `Manual` path and no forced recalc drained anything
    /// afterwards (none was forced, it failed, or it had nothing to drain).
    pub manual_load_undrained: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::ExcelText;

    fn text(value: &str) -> CalcValue {
        CalcValue::text(ExcelText::from_interop_assignment(value))
    }

    #[test]
    fn numbers_match_on_exact_bits_and_on_signed_zero() {
        assert_eq!(
            compare(&CalcValue::number(21.0), &CalcValue::number(21.0)),
            Verdict::Match
        );
        assert_eq!(
            compare(&CalcValue::number(-0.0), &CalcValue::number(0.0)),
            Verdict::Match
        );
    }

    #[test]
    fn numbers_within_four_ulp_are_ulp_mismatches_with_the_distance() {
        let one = 1.0_f64;
        let one_plus = f64::from_bits(one.to_bits() + 3);
        assert_eq!(ulp_distance(one, one_plus), 3);
        assert_eq!(
            compare(&CalcValue::number(one_plus), &CalcValue::number(one)),
            Verdict::Mismatch(MismatchClass::Ulp { distance: 3 })
        );
        let one_plus_4 = f64::from_bits(one.to_bits() + 4);
        assert_eq!(
            compare(&CalcValue::number(one), &CalcValue::number(one_plus_4)),
            Verdict::Mismatch(MismatchClass::Ulp { distance: 4 })
        );
        // Across zero the ordered mapping still counts representable doubles.
        assert_eq!(
            ulp_distance(-f64::MIN_POSITIVE, f64::MIN_POSITIVE),
            2 * (1u64 << 52)
        );
    }

    #[test]
    fn numbers_agreeing_at_fifteen_digits_but_beyond_four_ulp_are_significant_digits_15() {
        // 0.1 + 0.2 vs 0.3: 1 ULP apart -> Ulp, not SignificantDigits15.
        assert_eq!(
            compare(&CalcValue::number(0.1 + 0.2), &CalcValue::number(0.3)),
            Verdict::Mismatch(MismatchClass::Ulp { distance: 1 })
        );
        // Excel's 15-digit cache of 1/3 vs the full double: many ULP apart,
        // equal at 15 significant digits.
        let full = 1.0_f64 / 3.0;
        let cached_15: f64 = "0.333333333333333".parse().unwrap();
        assert!(ulp_distance(full, cached_15) > ULP_TOLERANCE);
        assert_eq!(significant_15(full), significant_15(cached_15));
        assert_eq!(
            compare(&CalcValue::number(full), &CalcValue::number(cached_15)),
            Verdict::Mismatch(MismatchClass::SignificantDigits15)
        );
        // A real numeric divergence.
        assert_eq!(
            compare(&CalcValue::number(1.0), &CalcValue::number(2.0)),
            Verdict::Mismatch(MismatchClass::Number)
        );
        assert_eq!(
            compare(&CalcValue::number(f64::INFINITY), &CalcValue::number(1e308)),
            Verdict::Mismatch(MismatchClass::Number)
        );
    }

    #[test]
    fn errors_compare_by_code_and_asymmetric_errors_name_their_side() {
        let name = CalcValue::error(WorksheetErrorCode::Name);
        let value = CalcValue::error(WorksheetErrorCode::Value);
        assert_eq!(compare(&name, &name), Verdict::Match);
        assert_eq!(
            compare(&value, &name),
            Verdict::Mismatch(MismatchClass::ErrorCode {
                engine: WorksheetErrorCode::Value,
                excel: WorksheetErrorCode::Name,
            })
        );
        assert_eq!(
            compare(&value, &CalcValue::number(1.0)),
            Verdict::Mismatch(MismatchClass::EngineError {
                code: WorksheetErrorCode::Value
            })
        );
        assert_eq!(
            compare(
                &CalcValue::number(1.0),
                &CalcValue::error(WorksheetErrorCode::Div0)
            ),
            Verdict::Mismatch(MismatchClass::OracleError {
                code: WorksheetErrorCode::Div0
            })
        );
        assert_eq!(
            MismatchClass::EngineError {
                code: WorksheetErrorCode::Value
            }
            .key(),
            "EngineError:Value"
        );
    }

    #[test]
    fn text_and_logical_compare_exactly_and_kinds_are_type_mismatches() {
        assert_eq!(compare(&text("abc"), &text("abc")), Verdict::Match);
        assert_eq!(
            compare(&text("abc"), &text("ABC")),
            Verdict::Mismatch(MismatchClass::Text)
        );
        assert_eq!(
            compare(&CalcValue::logical(true), &CalcValue::logical(true)),
            Verdict::Match
        );
        assert_eq!(
            compare(&CalcValue::logical(true), &CalcValue::logical(false)),
            Verdict::Mismatch(MismatchClass::Logical)
        );
        assert_eq!(
            compare(&CalcValue::empty(), &CalcValue::number(0.0)),
            Verdict::Mismatch(MismatchClass::Type {
                engine: "Empty".to_string(),
                excel: "Number".to_string(),
            })
        );
        assert_eq!(
            compare(&CalcValue::logical(true), &CalcValue::number(1.0)),
            Verdict::Mismatch(MismatchClass::Type {
                engine: "Logical".to_string(),
                excel: "Number".to_string(),
            })
        );
        assert_eq!(
            compare(&CalcValue::number(1.0), &text("1")),
            Verdict::Mismatch(MismatchClass::Type {
                engine: "Number".to_string(),
                excel: "Text".to_string(),
            })
        );
    }

    #[test]
    fn engine_shapes_the_classifier_does_not_model_are_unclassified() {
        let verdict = compare(&CalcValue::missing(), &CalcValue::number(1.0));
        assert_eq!(
            verdict,
            Verdict::Mismatch(MismatchClass::Unclassified {
                engine: "Missing".to_string()
            })
        );
        assert_eq!(verdict.key(), "mismatch:Unclassified:Missing");
        let Verdict::Mismatch(class) = verdict else {
            unreachable!()
        };
        assert!(class.is_unclassified());
        // An unmodeled engine shape stays Unclassified even against an Excel
        // error: it is never folded into OracleError.
        assert_eq!(
            compare(
                &CalcValue::missing(),
                &CalcValue::error(WorksheetErrorCode::Value)
            ),
            Verdict::Mismatch(MismatchClass::Unclassified {
                engine: "Missing".to_string()
            })
        );
    }

    #[test]
    fn file_cached_refinement_follows_the_precedence_order() {
        let facts = PinFacts {
            bind_degraded: true,
            external_reference_pinned: true,
            not_calc_modeled_kind: true,
            manual_load_undrained: true,
        };
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::FileCached, &facts),
            Some(NotEvaluatedReason::BindDegraded)
        );
        let facts = PinFacts {
            bind_degraded: false,
            ..facts
        };
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::FileCached, &facts),
            Some(NotEvaluatedReason::ExternalReferencePinned)
        );
        let facts = PinFacts {
            external_reference_pinned: false,
            ..facts
        };
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::FileCached, &facts),
            Some(NotEvaluatedReason::NotCalcModeled)
        );
        let facts = PinFacts {
            not_calc_modeled_kind: false,
            ..facts
        };
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::FileCached, &facts),
            Some(NotEvaluatedReason::ManualMode)
        );
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::FileCached, &PinFacts::default()),
            Some(NotEvaluatedReason::FileCached)
        );
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::Degraded, &facts),
            Some(NotEvaluatedReason::Degraded)
        );
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::Stale { since_tick_id: 3 }, &facts),
            Some(NotEvaluatedReason::Stale)
        );
        assert_eq!(
            not_evaluated_reason(PublishedValueProvenance::Calculated { tick_id: 3 }, &facts),
            None
        );
    }

    #[test]
    fn keys_are_stable_strings() {
        assert_eq!(
            Verdict::NotEvaluated(NotEvaluatedReason::OpenFailed {
                kind: "oxcalc:GridEngine:InvalidDefinedName".to_string()
            })
            .key(),
            "not_evaluated:open_failed:oxcalc:GridEngine:InvalidDefinedName"
        );
        assert_eq!(
            Verdict::Excluded(ExcludedReason::Nondeterministic {
                function: Some("NOW"),
                ca_marker: false
            })
            .key(),
            "excluded:nondeterministic:NOW"
        );
        assert_eq!(
            Verdict::Excluded(ExcludedReason::Nondeterministic {
                function: None,
                ca_marker: true
            })
            .key(),
            "excluded:nondeterministic:ca=1"
        );
        assert_eq!(Verdict::Match.key(), "match");
        assert_eq!(
            Verdict::Mismatch(MismatchClass::Ulp { distance: 2 }).key(),
            "mismatch:Ulp"
        );
        assert_eq!(
            ExcludedReason::from(OracleValueGap::UnknownErrorByte(255)).key(),
            "oracle_unknown_error_code"
        );
    }
}
