//! `.dnafml` (and equivalent `.xml`) persistence — slice 1.
//!
//! Schema is the XML Spreadsheet 2003 + `dna:` extension lane defined
//! in [PERSISTENCE_FORMAT_PLAN.md §5](../../../../../docs/PERSISTENCE_FORMAT_PLAN.md):
//!
//!   * `<Worksheet>` carries the formula in cell A1 so Excel double-click
//!     opens it,
//!   * `<dna:Formula>` carries identity, entry, context, and UI prefs that
//!     Excel doesn't represent,
//!   * `<dna:CompareBundle>` siblings (slice 4) accumulate compare-with-Excel
//!     evidence as history.
//!
//! This slice (1) ships the in-memory `Scenario` shape, a hand-rolled XML
//! emitter, and a `roxmltree`-backed reader, with end-to-end round-trip
//! tests. Wiring `Save as…` / `Open…` actions to the breadcrumb dropdown
//! is slice 1b; the compare-bundle merge is slice 4.
//!
//! Internal architectural name: `scenario`. User-facing name: `formula`
//! (per [APP_UX_BRIEF.md §1A](../../../../../docs/APP_UX_BRIEF.md)).

use std::fmt;

use roxmltree::{Document, Node};

const SS_NAMESPACE: &str = "urn:schemas-microsoft-com:office:spreadsheet";
const DNA_NAMESPACE: &str = "urn:dnakode:dnaonecalc:formula:1";

const FORMULA_VERSION: &str = "1";

// ---------------------------------------------------------------------------
// In-memory shape
// ---------------------------------------------------------------------------

/// One persisted formula scenario. The on-disk XML round-trips this
/// struct verbatim — every field maps to either an Excel-native location
/// or a `dna:` extension element.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scenario {
    pub identity: Identity,
    pub entry: Entry,
    pub context: Context,
    pub ui_preferences: UiPreferences,
    /// Compare-with-Excel evidence bundles in chronological-ascending
    /// order by `compared_at`. Per `PERSISTENCE_FORMAT_PLAN §9` the
    /// `.dnafml` carries zero-or-more bundles (empty by default; the
    /// user adds them by running Compare with Excel and choosing
    /// Save bundle). `apply_bundle_retention_policy` enforces the
    /// §9.5 cap at save time.
    pub bundles: Vec<CompareBundle>,
    /// Forward-compat: raw outer-XML of any element under `<Workbook>`
    /// this build did not recognise. Populated on read for workbook-
    /// root elements outside the known set (Worksheet, ExcelWorkbook,
    /// Styles, dna:Formula, dna:CompareBundle); re-emitted verbatim
    /// at the workbook root after the known children on write. Lets
    /// an older OneCalc build open a file written by a newer one,
    /// edit the formula, and save without silently destroying data
    /// the older build doesn't understand.
    ///
    /// Limitations: only workbook-root unknowns are preserved.
    /// Unknown attributes / sub-elements nested inside known elements
    /// (e.g. a new attribute on `<dna:Identity>`) are still lost —
    /// per-element walking is a follow-up bead.
    pub unknown_root_xml: Vec<String>,
}

/// Stable identifying metadata. Timestamps are ISO-8601 UTC strings;
/// the persistence layer does not parse them, just round-trips.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
}

/// The formula text plus its entry mode (Formula / Value / Text / Empty).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entry {
    pub mode: EntryMode,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntryMode {
    Formula,
    Value,
    Text,
    #[default]
    Empty,
}

impl EntryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Formula => "Formula",
            Self::Value => "Value",
            Self::Text => "Text",
            Self::Empty => "Empty",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Formula" => Some(Self::Formula),
            "Value" => Some(Self::Value),
            "Text" => Some(Self::Text),
            "Empty" => Some(Self::Empty),
            _ => None,
        }
    }
}

/// Presentation + execution context that determines how the formula
/// renders and how it would be compared with Excel. Mirrors the host
/// state's scenario-context fields plus the publication-context plane
/// per `APP_UX_REALIZATION §5.1`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Context {
    pub host_profile: HostProfile,
    pub locale: Locale,
    pub publication_context: PublicationContext,
    pub scenario_policy: ScenarioPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostProfile {
    pub profile_id: String,
    pub requires_excel_observation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Locale {
    pub id: String,
    pub date1904: bool,
}

/// Authoritative formatting + style + CF context for the cell. Mirrors
/// the upstream `VerificationPublicationContext` shape, kept simple in
/// slice 1 — `style_hierarchy` and `cf_rules` are reserved schema slots
/// and are not yet round-tripped (empty on read; only written when the
/// in-memory state has them populated, which today never happens).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PublicationContext {
    pub format_profile: String,
    pub number_format_code: String,
    pub style_id: String,
    pub font_color: String,
    pub fill_color: String,
    pub style_hierarchy: Vec<String>,
    pub cf_rules: Vec<CfRule>,
}

/// One CF rule. Minimal shape for slice 1; richer fields land alongside
/// `OxFml::publication::VerificationConditionalFormattingRule` mapping
/// in slice 2.
/// One CF rule. Two flavours overlap on this single shape:
///
/// - **Worksheet-range rules** (the SpreadsheetML 2003
///   `<ConditionalFormatting>` lane) populate `range` plus `formula`
///   / `rule_kind`. These persist into the worksheet block.
/// - **Result-hero rules** (the host's
///   `FormulaConditionalFormattingRule`, attached to the formula's
///   single-cell display) leave `range` empty and populate the
///   `operator` / `thresholds` / `font_color` / `fill_color`
///   fields. These persist inside `<dna:CfRules>` only.
///
/// `scenario_projection.rs` decides which flavour each rule is and
/// projects accordingly. The persistence layer never tries to
/// disambiguate further than "range vs. no range".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CfRule {
    pub range: String,
    pub formula: Option<String>,
    pub rule_kind: Option<String>,
    pub operator: Option<String>,
    pub thresholds: Vec<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    /// Optional typed CF rule payload, mirroring OxFml W073's
    /// `ConditionalFormattingTypedRule`. Persisted as a JSON string
    /// inside a `<dna:TypedRule>` child element of `<dna:CfRule>` so
    /// the typed shape can grow without churning the XML grammar.
    /// `None` when the rule relies purely on the W072 bounded-string
    /// `thresholds` convention.
    pub typed_rule: Option<CfTypedRule>,
}

/// Persistence shape for the typed CF rule payload. Mirrors
/// `crate::state::FormulaConditionalFormattingTypedRule` field-for-
/// field; the projection layer (`scenario_projection.rs`) maps
/// between this and the in-memory shape, and the JSON ser/de helpers
/// at the bottom of this module round-trip it through the XML.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CfTypedRule {
    pub color_scale: Option<CfColorScaleRuleOptions>,
    pub data_bar: Option<CfDataBarRuleOptions>,
    pub icon_set: Option<CfIconSetRuleOptions>,
    pub rank: Option<CfRankRuleOptions>,
    pub average: Option<CfAverageRuleOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CfColorScaleRuleOptions {
    pub stops: Vec<CfColorScaleStop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfColorScaleStop {
    pub position: CfThreshold,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CfDataBarRuleOptions {
    pub minimum: Option<CfThreshold>,
    pub maximum: Option<CfThreshold>,
    pub bar_color: Option<String>,
    pub direction: Option<CfDataBarDirection>,
    pub show_bar_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfDataBarDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfIconSetRuleOptions {
    pub set_kind: String,
    pub thresholds: Vec<CfThreshold>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfRankRuleOptions {
    pub rank: CfRank,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CfRank {
    Count(usize),
    Percent(f64),
}

impl Eq for CfRank {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CfAverageRuleOptions {
    pub include_equal: bool,
    pub stddev_multiplier: Option<f64>,
}

impl Eq for CfAverageRuleOptions {}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum CfThreshold {
    #[default]
    Min,
    Mid,
    Max,
    Percent(f64),
    Percentile(f64),
    Number(f64),
}

impl Eq for CfThreshold {}

/// Per-formula recalc + seeding policy. Three states the user
/// picks via the formatting-panel segmented control:
///
/// - `Deterministic` — pin volatile-function seeds (`=NOW()`,
///   `=RAND()`, `=RANDARRAY()`) so the formula re-runs identically
///   on every keystroke. The runtime pass still runs on every text
///   event; the value is stable. Authoring-friendly default for
///   formulas the user wants to reproduce verbatim.
/// - `LiveRecalc` — fresh seeds per bridge round-trip. Runtime
///   runs on every text event. Matches Excel's default-on workbook
///   behaviour (`=NOW()` advances per keystroke).
/// - `ManualRecalc` — runtime evaluation skipped on text events;
///   the formula re-runs only on an explicit Calculate / F9
///   request. Seeds refresh at that moment. Right choice for
///   expensive formulas (REDUCE / MAKEARRAY / large LAMBDA) where
///   typing latency would otherwise be dominated by re-evaluation
///   cost. Until OxFml's lambda invoker hoists invariant work
///   (`HANDOFF_OXFML_LAMBDA_INVOCATION_PERF.md`) and OxFunc's
///   helpers grow lazy iteration
///   (`HANDOFF_OXFUNC_REDUCE_HOTLOOP_PERF.md`), this is the user's
///   primary lever for keeping the editor responsive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScenarioPolicy {
    Deterministic,
    /// Default: volatile functions (`=NOW()`, `=RAND()`,
    /// `=RANDARRAY()`) re-evaluate each bridge round-trip.
    /// Matches Excel's default-on workbook behaviour.
    #[default]
    LiveRecalc,
    /// Runtime evaluation is gated on Calculate / F9. Text edits
    /// run parse / bind / popup refresh only.
    ManualRecalc,
}

impl ScenarioPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "Deterministic",
            Self::LiveRecalc => "LiveRecalc",
            Self::ManualRecalc => "ManualRecalc",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Deterministic" => Some(Self::Deterministic),
            "LiveRecalc" => Some(Self::LiveRecalc),
            "ManualRecalc" => Some(Self::ManualRecalc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiPreferences {
    pub formula_drill_expanded: bool,
    pub result_drill_expanded: bool,
    pub expanded_editor: bool,
}

// ---------------------------------------------------------------------------
// Compare bundles (slice 4)
// ---------------------------------------------------------------------------

/// One compare-with-Excel run. Per `PERSISTENCE_FORMAT_PLAN §9` the
/// `.dnafml` carries zero-or-more bundles in chronological-ascending
/// order by `compared_at`. Each bundle carries the metadata
/// describing what was compared and the three top-level verdicts;
/// the detailed VerificationRequest / VerificationReport / OxFmlSummary
/// / ExcelObservationSummary / ReplayMismatch / ReplayExplain payloads
/// are reserved schema slots that land in a later slice once a real
/// Compare-with-Excel workflow produces them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareBundle {
    /// Stable id the UI targets for delete / replace / pin actions.
    /// Convention from §9: `cb-<timestamp>-<excel-host-id>`, but the
    /// reader does not parse the structure — any non-empty string
    /// works and round-trips verbatim.
    pub bundle_id: String,
    /// ISO-8601 UTC. Format is up to the caller; the persistence
    /// layer just round-trips.
    pub compared_at: String,
    /// e.g. `Excel365Win-16.0.18025`. Empty when unknown.
    pub excel_host_id: String,
    /// Digest of the formula state the bundle was generated against
    /// (formula text + relevant context). Lets the UI distinguish
    /// "live" bundles (digest matches the current scenario) from
    /// "history" bundles. Empty when no digest was computed yet.
    pub for_formula_state: String,
    pub value_verdict: BundleVerdict,
    pub display_verdict: BundleVerdict,
    pub replay_verdict: BundleVerdict,
    /// Optional human-readable summary the UI displays on the
    /// bundle row. Slice 4 carries this as a single text node;
    /// later slices add structured per-mismatch / per-explain
    /// elements alongside.
    pub summary: Option<String>,
}

/// Verdict for one of the three comparison families. `Unknown` is
/// the safe default; round-trips through any future renames upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BundleVerdict {
    Match,
    Mismatch,
    Equivalent,
    Blocked,
    #[default]
    Unknown,
}

impl BundleVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Mismatch => "mismatch",
            Self::Equivalent => "equivalent",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "match" => Self::Match,
            "mismatch" => Self::Mismatch,
            "equivalent" => Self::Equivalent,
            "blocked" => Self::Blocked,
            _ => Self::Unknown,
        }
    }
}

/// Default cap from `PERSISTENCE_FORMAT_PLAN §9.5`. The user can
/// override per-file or globally; the value here is the floor.
pub const DEFAULT_BUNDLE_RETENTION_CAP: usize = 10;

/// Apply the §9.5 retention policy to a chronologically-ascending
/// bundle list against the current formula state. Always keeps the
/// most-recent bundle for each `(for_formula_state, excel_host_id)`
/// pair; preserves all bundles whose `for_formula_state` matches
/// the current state ("live" bundles); when over the cap, drops the
/// oldest history-only bundles first; never drops a live bundle to
/// satisfy the cap.
///
/// `bundles` is mutated in place. The result preserves chronological
/// order. Pruning runs at save time only — see §9.5.
pub fn apply_bundle_retention_policy(
    bundles: &mut Vec<CompareBundle>,
    current_for_formula_state: &str,
    cap: usize,
) {
    if bundles.is_empty() {
        return;
    }

    // Sort defensively — bundles should already be ascending, but
    // re-sorting on save makes the contract robust to in-memory
    // mutation order.
    bundles.sort_by(|left, right| left.compared_at.cmp(&right.compared_at));

    // Step 1: dedup `(for_formula_state, excel_host_id)` keeping the
    // most-recent bundle per pair. Bundles with empty
    // `for_formula_state` AND empty `excel_host_id` are treated as
    // distinct from each other (we never coalesce into "everything
    // unknown").
    let mut latest_per_pair: std::collections::BTreeMap<(String, String), usize> =
        std::collections::BTreeMap::new();
    let mut keep = vec![true; bundles.len()];
    for (index, bundle) in bundles.iter().enumerate() {
        let key = (
            bundle.for_formula_state.clone(),
            bundle.excel_host_id.clone(),
        );
        if let Some(prior) = latest_per_pair.insert(key, index) {
            keep[prior] = false;
        }
    }

    // Step 2: enforce the cap, dropping oldest history-only entries
    // first. Live bundles (digest matches the current state) are
    // never dropped to make space.
    let mut alive_count = keep.iter().filter(|&&k| k).count();
    if alive_count > cap {
        for index in 0..bundles.len() {
            if alive_count <= cap {
                break;
            }
            if !keep[index] {
                continue;
            }
            // History-only bundle? Eligible for drop.
            if bundles[index].for_formula_state != current_for_formula_state {
                keep[index] = false;
                alive_count -= 1;
            }
        }
        // If even after dropping every history bundle we're still
        // over, cap the live-bundle list by oldest-first to avoid
        // unbounded growth. This shouldn't happen in normal use.
        if alive_count > cap {
            for index in 0..bundles.len() {
                if alive_count <= cap {
                    break;
                }
                if keep[index] {
                    keep[index] = false;
                    alive_count -= 1;
                }
            }
        }
    }

    // Step 3: rebuild the list preserving the kept entries' order.
    let mut iter = keep.into_iter();
    bundles.retain(|_| iter.next().unwrap_or(false));
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaFileError {
    /// XML is not well-formed.
    Parse(String),
    /// XML is well-formed but no recognisable `<dna:Formula>` extension
    /// AND no usable Excel-native fallback (e.g. missing `<Worksheet>` or
    /// no cell).
    NotADnaFormula(String),
    /// `<dna:Formula>` carries a `version` attribute we do not understand.
    /// The caller is expected to surface this honestly to the user rather
    /// than silently downgrade.
    UnsupportedVersion(String),
}

/// A successful load. Carries the deserialised `Scenario` plus a list
/// of diagnostics the caller can surface to the user — for example
/// when a file was loaded through the Excel-only fallback path
/// because `<dna:Formula>` wasn't present (the user opened a file
/// that had been saved by Excel after a round-trip; non-formula
/// fields filled in with defaults).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedFormula {
    pub scenario: Scenario,
    pub diagnostics: Vec<LoadDiagnostic>,
}

/// Per-file diagnostic surfaced from the reader. The host renders
/// these as a non-blocking warning chip in the status-foot until
/// the user explicitly saves (which re-establishes the canonical
/// `dna:` extension and clears the diagnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadDiagnostic {
    /// File was loaded through the Excel-only fallback path: the
    /// `<dna:Formula>` extension was absent, so identity / context
    /// / UI prefs / compare-bundles all defaulted. The cell formula
    /// (and any inline styling) was recovered from the worksheet.
    ImportedFromExcelOnlyFile,
}

impl LoadDiagnostic {
    pub fn slug(self) -> &'static str {
        match self {
            Self::ImportedFromExcelOnlyFile => "imported-from-excel-only-file",
        }
    }

    pub fn user_message(self) -> &'static str {
        match self {
            Self::ImportedFromExcelOnlyFile => {
                "Imported from an Excel-only file — context defaults applied. \
                 Save to write a full DnaOneCalc formula file."
            }
        }
    }
}

impl fmt::Display for FormulaFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "failed to parse XML: {message}"),
            Self::NotADnaFormula(message) => write!(
                f,
                "file is not a recognisable DnaOneCalc formula: {message}",
            ),
            Self::UnsupportedVersion(version) => write!(
                f,
                "dna:Formula version `{version}` is not supported by this build",
            ),
        }
    }
}

impl std::error::Error for FormulaFileError {}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Serialise a `Scenario` to the `.dnafml` (or `.xml`) byte form.
/// Output is UTF-8 with `\n` line endings; callers responsible for
/// platform newlines if they care. The output begins with the XML
/// processing instruction and the `<?mso-application?>` PI so Excel
/// associates it with the spreadsheet renderer.
///
/// The cell value (`<Data>` text node) is the formula text when the
/// entry is a literal value/text, the empty string for a Formula entry
/// (Excel will recompute the value when it opens the file), and the
/// raw text for the `Empty` entry mode (writes an empty cell).
pub fn write_formula_xml(scenario: &Scenario) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    out.push('\n');
    out.push_str(r#"<?mso-application progid="Excel.Sheet"?>"#);
    out.push('\n');
    out.push_str(r#"<Workbook xmlns=""#);
    out.push_str(SS_NAMESPACE);
    out.push_str("\"\n");
    out.push_str(r#"          xmlns:o="urn:schemas-microsoft-com:office:office""#);
    out.push('\n');
    out.push_str(r#"          xmlns:x="urn:schemas-microsoft-com:office:excel""#);
    out.push('\n');
    out.push_str(r#"          xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet""#);
    out.push('\n');
    out.push_str(r#"          xmlns:dna=""#);
    out.push_str(DNA_NAMESPACE);
    out.push_str("\">\n");

    // Slice 2: Excel-native fidelity — emit native sub-elements that
    // a fresh Excel double-click renders correctly. Each native
    // emission is paired with the `dna:` form so the next OneCalc
    // load reads from a single canonical source (per §5.3 write
    // rule). The native emission is conditional: empty fields
    // (no number format, no colours, default Date1904) skip emit
    // so simple scenarios stay simple.
    write_excel_workbook_options(&mut out, scenario);
    write_styles_block(&mut out, scenario);
    write_worksheet(&mut out, scenario);
    write_dna_formula(&mut out, scenario);
    for bundle in &scenario.bundles {
        write_dna_compare_bundle(&mut out, bundle);
    }
    // Forward-compat: re-emit any unrecognised workbook-root
    // children verbatim (per the unknown-element preservation
    // contract). Indented by two spaces to match the rest of the
    // workbook body.
    for unknown in &scenario.unknown_root_xml {
        out.push_str("  ");
        out.push_str(unknown);
        if !unknown.ends_with('\n') {
            out.push('\n');
        }
    }

    out.push_str("</Workbook>\n");
    out
}

/// Emit the `<ExcelWorkbook>` block when the scenario's locale
/// requires it. Only `<Date1904>` is emitted today; future workbook
/// options (refmode, iteration, protection) land here when their
/// `dna:` schema slots are added.
fn write_excel_workbook_options(out: &mut String, scenario: &Scenario) {
    if !scenario.context.locale.date1904 {
        return;
    }
    out.push_str("  <ExcelWorkbook>\n");
    out.push_str("    <Date1904/>\n");
    out.push_str("  </ExcelWorkbook>\n");
}

/// Emit a `<Styles>` block with one `<Style>` per non-empty
/// publication-context style group. Slice 2 covers NumberFormat,
/// Font color, and Interior color — the core round-trip targets in
/// the WS-14 admitted formatting set. Borders, Alignment, and the
/// rest of the Font and Interior attribute matrix are reserved for
/// later slices once their `dna:` schema slots exist.
fn write_styles_block(out: &mut String, scenario: &Scenario) {
    if !needs_default_style(scenario) {
        return;
    }
    out.push_str("  <Styles>\n");
    let style_id = effective_style_id(scenario);
    out.push_str("    <Style ss:ID=\"");
    out.push_str(&xml_attr_escape(&style_id));
    out.push_str("\">\n");
    let pc = &scenario.context.publication_context;
    if !pc.number_format_code.is_empty() {
        out.push_str("      <NumberFormat ss:Format=\"");
        out.push_str(&xml_attr_escape(&pc.number_format_code));
        out.push_str("\"/>\n");
    }
    if !pc.font_color.is_empty() {
        out.push_str("      <Font ss:Color=\"");
        out.push_str(&xml_attr_escape(&pc.font_color));
        out.push_str("\"/>\n");
    }
    if !pc.fill_color.is_empty() {
        out.push_str("      <Interior ss:Color=\"");
        out.push_str(&xml_attr_escape(&pc.fill_color));
        out.push_str("\" ss:Pattern=\"Solid\"/>\n");
    }
    out.push_str("    </Style>\n");
    out.push_str("  </Styles>\n");
}

fn needs_default_style(scenario: &Scenario) -> bool {
    let pc = &scenario.context.publication_context;
    !pc.number_format_code.is_empty()
        || !pc.font_color.is_empty()
        || !pc.fill_color.is_empty()
        || !pc.style_id.is_empty()
}

fn effective_style_id(scenario: &Scenario) -> String {
    let pc = &scenario.context.publication_context;
    if !pc.style_id.is_empty() {
        pc.style_id.clone()
    } else {
        "dna-cell-style".to_string()
    }
}

fn write_dna_compare_bundle(out: &mut String, bundle: &CompareBundle) {
    out.push_str("  <dna:CompareBundle");
    write_attr(out, "bundle-id", &bundle.bundle_id);
    write_attr(out, "compared-at", &bundle.compared_at);
    write_attr(out, "excel-host-id", &bundle.excel_host_id);
    write_attr(out, "for-formula-state", &bundle.for_formula_state);
    write_attr(out, "value-verdict", bundle.value_verdict.as_str());
    write_attr(out, "display-verdict", bundle.display_verdict.as_str());
    write_attr(out, "replay-verdict", bundle.replay_verdict.as_str());
    if let Some(summary) = bundle.summary.as_deref() {
        out.push_str(">\n");
        out.push_str("    <dna:Summary>");
        out.push_str(&xml_text_escape(summary));
        out.push_str("</dna:Summary>\n");
        out.push_str("  </dna:CompareBundle>\n");
    } else {
        out.push_str("/>\n");
    }
}

fn write_worksheet(out: &mut String, scenario: &Scenario) {
    out.push_str("  <Worksheet ss:Name=\"Formula\">\n");
    out.push_str("    <Table>\n");
    out.push_str("      <Row>\n");
    write_cell(out, scenario);
    out.push_str("      </Row>\n");
    out.push_str("    </Table>\n");
    write_native_conditional_formatting(out, scenario);
    out.push_str("  </Worksheet>\n");
}

fn write_cell(out: &mut String, scenario: &Scenario) {
    let raw = &scenario.entry.text;
    let style_attr = if needs_default_style(scenario) {
        format!(
            " ss:StyleID=\"{}\"",
            xml_attr_escape(&effective_style_id(scenario))
        )
    } else {
        String::new()
    };
    match scenario.entry.mode {
        EntryMode::Formula => {
            out.push_str("        <Cell");
            out.push_str(&style_attr);
            out.push_str(" ss:Formula=\"");
            out.push_str(&xml_attr_escape(raw));
            out.push_str("\"><Data ss:Type=\"String\"></Data></Cell>\n");
        }
        EntryMode::Value => {
            // Try to render as a number when the raw text parses; else
            // render as a string Cell. This is what Excel users expect
            // from the canonical "value" entry mode.
            if let Ok(number) = raw.parse::<f64>() {
                if number.is_finite() {
                    out.push_str("        <Cell");
                    out.push_str(&style_attr);
                    out.push_str("><Data ss:Type=\"Number\">");
                    out.push_str(&xml_text_escape(raw));
                    out.push_str("</Data></Cell>\n");
                    return;
                }
            }
            out.push_str("        <Cell");
            out.push_str(&style_attr);
            out.push_str("><Data ss:Type=\"String\">");
            out.push_str(&xml_text_escape(raw));
            out.push_str("</Data></Cell>\n");
        }
        EntryMode::Text => {
            // Forced text via leading apostrophe — drop the apostrophe
            // for the Excel-visible cell since Excel handles that prefix
            // natively.
            let stripped = raw.strip_prefix('\'').unwrap_or(raw);
            out.push_str("        <Cell");
            out.push_str(&style_attr);
            out.push_str("><Data ss:Type=\"String\">");
            out.push_str(&xml_text_escape(stripped));
            out.push_str("</Data></Cell>\n");
        }
        EntryMode::Empty => {
            out.push_str("        <Cell");
            out.push_str(&style_attr);
            out.push_str("><Data ss:Type=\"String\"></Data></Cell>\n");
        }
    }
}

/// Emit one `<ConditionalFormatting>` block per CF rule. Slice 2
/// covers both qualifier-comparison rules (when `rule_kind` is
/// `CellIs` and a value/operator are present) and Expression rules
/// (when `formula` is present). The rule's effective style is
/// expressed as inline `<Font>` / `<Interior>` children. The full
/// CF schema (color scales, data bars, icon sets) is OOXML-only and
/// out of scope for SpreadsheetML 2003.
fn write_native_conditional_formatting(out: &mut String, scenario: &Scenario) {
    let rules = &scenario.context.publication_context.cf_rules;
    if rules.is_empty() {
        return;
    }
    for rule in rules {
        if rule.range.is_empty() {
            continue;
        }
        out.push_str("    <ConditionalFormatting ss:Range=\"");
        out.push_str(&xml_attr_escape(&rule.range));
        out.push_str("\">\n");
        out.push_str("      <Condition");
        if rule.rule_kind.as_deref() == Some("Expression") {
            out.push_str(" ss:Type=\"Expression\"");
        }
        if let Some(formula) = rule.formula.as_deref() {
            if !formula.is_empty() {
                out.push_str(" ss:Formula=\"");
                out.push_str(&xml_attr_escape(formula));
                out.push_str("\"");
            }
        }
        out.push_str("/>\n");
        out.push_str("    </ConditionalFormatting>\n");
    }
}

fn write_dna_formula(out: &mut String, scenario: &Scenario) {
    out.push_str("  <dna:Formula version=\"");
    out.push_str(FORMULA_VERSION);
    out.push_str("\">\n");

    write_dna_identity(out, &scenario.identity);
    write_dna_entry(out, &scenario.entry);
    write_dna_context(out, &scenario.context);
    write_dna_ui_preferences(out, &scenario.ui_preferences);

    out.push_str("  </dna:Formula>\n");
}

fn write_dna_identity(out: &mut String, identity: &Identity) {
    out.push_str("    <dna:Identity");
    write_attr(out, "id", &identity.id);
    write_attr(out, "name", &identity.name);
    write_attr(out, "created-at", &identity.created_at);
    write_attr(out, "modified-at", &identity.modified_at);
    out.push_str("/>\n");
}

fn write_dna_entry(out: &mut String, entry: &Entry) {
    out.push_str("    <dna:Entry mode=\"");
    out.push_str(entry.mode.as_str());
    out.push_str("\">");
    out.push_str(&xml_text_escape(&entry.text));
    out.push_str("</dna:Entry>\n");
}

fn write_dna_context(out: &mut String, context: &Context) {
    out.push_str("    <dna:Context>\n");
    out.push_str("      <dna:HostProfile");
    write_attr(out, "profile-id", &context.host_profile.profile_id);
    write_attr(
        out,
        "requires-excel-observation",
        bool_attr(context.host_profile.requires_excel_observation),
    );
    out.push_str("/>\n");

    out.push_str("      <dna:Locale");
    write_attr(out, "id", &context.locale.id);
    write_attr(out, "date1904", bool_attr(context.locale.date1904));
    out.push_str("/>\n");

    write_dna_publication_context(out, &context.publication_context);

    out.push_str("      <dna:ScenarioPolicy>");
    out.push_str(context.scenario_policy.as_str());
    out.push_str("</dna:ScenarioPolicy>\n");
    out.push_str("    </dna:Context>\n");
}

fn write_dna_publication_context(out: &mut String, pc: &PublicationContext) {
    out.push_str("      <dna:PublicationContext");
    write_attr(out, "format-profile", &pc.format_profile);
    write_attr(out, "number-format-code", &pc.number_format_code);
    write_attr(out, "style-id", &pc.style_id);
    write_attr(out, "font-color", &pc.font_color);
    write_attr(out, "fill-color", &pc.fill_color);
    out.push_str(">\n");

    out.push_str("        <dna:StyleHierarchy>\n");
    for level in &pc.style_hierarchy {
        out.push_str("          <dna:StyleLevel");
        write_attr(out, "id", level);
        out.push_str("/>\n");
    }
    out.push_str("        </dna:StyleHierarchy>\n");

    out.push_str("        <dna:CfRules>\n");
    for rule in &pc.cf_rules {
        out.push_str("          <dna:CfRule");
        write_attr(out, "range", &rule.range);
        if let Some(formula) = rule.formula.as_deref() {
            write_attr(out, "formula", formula);
        }
        if let Some(rule_kind) = rule.rule_kind.as_deref() {
            write_attr(out, "rule-kind", rule_kind);
        }
        if let Some(operator) = rule.operator.as_deref() {
            write_attr(out, "operator", operator);
        }
        if let Some(font_color) = rule.font_color.as_deref() {
            write_attr(out, "font-color", font_color);
        }
        if let Some(fill_color) = rule.fill_color.as_deref() {
            write_attr(out, "fill-color", fill_color);
        }
        let typed_rule_json = rule.typed_rule.as_ref().map(write_cf_typed_rule_json);
        if rule.thresholds.is_empty() && typed_rule_json.is_none() {
            out.push_str("/>\n");
        } else {
            out.push_str(">\n");
            for threshold in &rule.thresholds {
                out.push_str("            <dna:Threshold>");
                out.push_str(&xml_text_escape(threshold));
                out.push_str("</dna:Threshold>\n");
            }
            if let Some(json) = typed_rule_json {
                out.push_str("            <dna:TypedRule>");
                out.push_str(&xml_text_escape(&json));
                out.push_str("</dna:TypedRule>\n");
            }
            out.push_str("          </dna:CfRule>\n");
        }
    }
    out.push_str("        </dna:CfRules>\n");
    out.push_str("      </dna:PublicationContext>\n");
}

fn write_dna_ui_preferences(out: &mut String, prefs: &UiPreferences) {
    out.push_str("    <dna:UiPreferences");
    write_attr(
        out,
        "formula-drill-expanded",
        bool_attr(prefs.formula_drill_expanded),
    );
    write_attr(
        out,
        "result-drill-expanded",
        bool_attr(prefs.result_drill_expanded),
    );
    write_attr(out, "expanded-editor", bool_attr(prefs.expanded_editor));
    out.push_str("/>\n");
}

fn write_attr(out: &mut String, name: &str, value: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    out.push_str(&xml_attr_escape(value));
    out.push('"');
}

fn bool_attr(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// Escape an attribute value. Replaces the five XML metacharacters
/// (`& < > " '`) plus all control characters except tab/CR/LF.
fn xml_attr_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(ch),
            ch if (ch as u32) < 0x20 => {
                // Drop other control characters; XML 1.0 forbids them.
            }
            ch => out.push(ch),
        }
    }
    out
}

/// Escape a text-node value. Same rules as attribute except `'` and
/// `"` don't strictly need escaping inside text — but we keep parity
/// for safety (no harm; readability unchanged).
fn xml_text_escape(value: &str) -> String {
    xml_attr_escape(value)
}

// ---------------------------------------------------------------------------
// Typed CF rule JSON ser/de
//
// The typed CF rule payload is round-tripped through XML as a JSON
// blob inside a `<dna:TypedRule>` child element of `<dna:CfRule>`. We
// build / parse the JSON by hand (no `serde_json::to_value` round-
// tripping through `Value`) so the encoded shape is stable and the
// `.dnafml` files stay diffable. The keys mirror the upstream
// `oxfml_core::publication::ConditionalFormattingTypedRule` JSON shape.
// ---------------------------------------------------------------------------

fn write_cf_typed_rule_json(rule: &CfTypedRule) -> String {
    let mut json = String::from("{");
    let mut first = true;
    let field = |json: &mut String, name: &str, value: String, first: &mut bool| {
        if !*first {
            json.push(',');
        }
        *first = false;
        json.push('"');
        json.push_str(name);
        json.push_str("\":");
        json.push_str(&value);
    };
    if let Some(options) = rule.color_scale.as_ref() {
        field(
            &mut json,
            "color_scale",
            color_scale_options_json(options),
            &mut first,
        );
    }
    if let Some(options) = rule.data_bar.as_ref() {
        field(
            &mut json,
            "data_bar",
            data_bar_options_json(options),
            &mut first,
        );
    }
    if let Some(options) = rule.icon_set.as_ref() {
        field(
            &mut json,
            "icon_set",
            icon_set_options_json(options),
            &mut first,
        );
    }
    if let Some(options) = rule.rank.as_ref() {
        field(&mut json, "rank", rank_options_json(options), &mut first);
    }
    if let Some(options) = rule.average.as_ref() {
        field(
            &mut json,
            "average",
            average_options_json(options),
            &mut first,
        );
    }
    json.push('}');
    json
}

fn color_scale_options_json(options: &CfColorScaleRuleOptions) -> String {
    let mut json = String::from("{\"stops\":[");
    for (index, stop) in options.stops.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        json.push_str("\"position\":");
        json.push_str(&threshold_json(&stop.position));
        json.push_str(",\"color\":");
        json.push_str(&json_string(&stop.color));
        json.push('}');
    }
    json.push_str("]}");
    json
}

fn data_bar_options_json(options: &CfDataBarRuleOptions) -> String {
    let mut json = String::from("{");
    let mut parts: Vec<String> = Vec::new();
    if let Some(threshold) = options.minimum.as_ref() {
        parts.push(format!("\"minimum\":{}", threshold_json(threshold)));
    }
    if let Some(threshold) = options.maximum.as_ref() {
        parts.push(format!("\"maximum\":{}", threshold_json(threshold)));
    }
    if let Some(color) = options.bar_color.as_ref() {
        parts.push(format!("\"bar_color\":{}", json_string(color)));
    }
    if let Some(direction) = options.direction {
        let label = match direction {
            CfDataBarDirection::Left => "Left",
            CfDataBarDirection::Right => "Right",
        };
        parts.push(format!("\"direction\":\"{label}\""));
    }
    if options.show_bar_only {
        parts.push("\"show_bar_only\":true".to_string());
    }
    json.push_str(&parts.join(","));
    json.push('}');
    json
}

fn icon_set_options_json(options: &CfIconSetRuleOptions) -> String {
    let mut json = String::from("{\"set_kind\":");
    json.push_str(&json_string(&options.set_kind));
    json.push_str(",\"thresholds\":[");
    for (index, threshold) in options.thresholds.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(&threshold_json(threshold));
    }
    json.push_str("]}");
    json
}

fn rank_options_json(options: &CfRankRuleOptions) -> String {
    match &options.rank {
        CfRank::Count(count) => format!("{{\"rank\":{{\"Count\":{count}}}}}"),
        CfRank::Percent(value) => format!("{{\"rank\":{{\"Percent\":{}}}}}", number_json(*value)),
    }
}

fn average_options_json(options: &CfAverageRuleOptions) -> String {
    let mut parts: Vec<String> = Vec::new();
    if options.include_equal {
        parts.push("\"include_equal\":true".to_string());
    }
    if let Some(value) = options.stddev_multiplier {
        parts.push(format!("\"stddev_multiplier\":{}", number_json(value)));
    }
    let mut json = String::from("{");
    json.push_str(&parts.join(","));
    json.push('}');
    json
}

fn threshold_json(threshold: &CfThreshold) -> String {
    match threshold {
        CfThreshold::Min => "\"Min\"".to_string(),
        CfThreshold::Mid => "\"Mid\"".to_string(),
        CfThreshold::Max => "\"Max\"".to_string(),
        CfThreshold::Percent(value) => format!("{{\"Percent\":{}}}", number_json(*value)),
        CfThreshold::Percentile(value) => format!("{{\"Percentile\":{}}}", number_json(*value)),
        CfThreshold::Number(value) => format!("{{\"Number\":{}}}", number_json(*value)),
    }
}

fn number_json(value: f64) -> String {
    if value.is_finite() {
        // Use shortest unambiguous f64 representation. For integer
        // values we still emit a decimal point so the value parses
        // back as an f64 (`3` would parse fine, but `3.0` is more
        // readable in diffs).
        if value.fract() == 0.0 && value.abs() < 1e16 {
            format!("{value:.1}")
        } else {
            format!("{value}")
        }
    } else {
        // Non-finite f64s have no JSON representation; emit `null`
        // so the reader stamps a default. Authoring should never
        // produce them but persistence shouldn't panic.
        "null".to_string()
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn parse_cf_typed_rule_json(json: &str) -> Option<CfTypedRule> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let object = value.as_object()?;
    Some(CfTypedRule {
        color_scale: object
            .get("color_scale")
            .and_then(parse_color_scale_options),
        data_bar: object.get("data_bar").and_then(parse_data_bar_options),
        icon_set: object.get("icon_set").and_then(parse_icon_set_options),
        rank: object.get("rank").and_then(parse_rank_options),
        average: object.get("average").and_then(parse_average_options),
    })
}

fn parse_color_scale_options(value: &serde_json::Value) -> Option<CfColorScaleRuleOptions> {
    let object = value.as_object()?;
    let stops_value = object.get("stops")?;
    let stops_array = stops_value.as_array()?;
    let mut stops: Vec<CfColorScaleStop> = Vec::with_capacity(stops_array.len());
    for entry in stops_array {
        let entry_object = entry.as_object()?;
        let position = parse_threshold(entry_object.get("position")?)?;
        let color = entry_object.get("color")?.as_str()?.to_string();
        stops.push(CfColorScaleStop { position, color });
    }
    Some(CfColorScaleRuleOptions { stops })
}

fn parse_data_bar_options(value: &serde_json::Value) -> Option<CfDataBarRuleOptions> {
    let object = value.as_object()?;
    let minimum = object.get("minimum").and_then(parse_threshold);
    let maximum = object.get("maximum").and_then(parse_threshold);
    let bar_color = object
        .get("bar_color")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let direction = match object.get("direction").and_then(|v| v.as_str()) {
        Some("Left") => Some(CfDataBarDirection::Left),
        Some("Right") => Some(CfDataBarDirection::Right),
        _ => None,
    };
    let show_bar_only = object
        .get("show_bar_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some(CfDataBarRuleOptions {
        minimum,
        maximum,
        bar_color,
        direction,
        show_bar_only,
    })
}

fn parse_icon_set_options(value: &serde_json::Value) -> Option<CfIconSetRuleOptions> {
    let object = value.as_object()?;
    let set_kind = object.get("set_kind")?.as_str()?.to_string();
    let thresholds_array = object.get("thresholds")?.as_array()?;
    let mut thresholds: Vec<CfThreshold> = Vec::with_capacity(thresholds_array.len());
    for entry in thresholds_array {
        thresholds.push(parse_threshold(entry)?);
    }
    Some(CfIconSetRuleOptions {
        set_kind,
        thresholds,
    })
}

fn parse_rank_options(value: &serde_json::Value) -> Option<CfRankRuleOptions> {
    let rank_value = value.as_object()?.get("rank")?;
    let rank_object = rank_value.as_object()?;
    if let Some(count_value) = rank_object.get("Count") {
        let count = count_value.as_u64()? as usize;
        return Some(CfRankRuleOptions {
            rank: CfRank::Count(count),
        });
    }
    if let Some(percent_value) = rank_object.get("Percent") {
        let percent = percent_value.as_f64()?;
        return Some(CfRankRuleOptions {
            rank: CfRank::Percent(percent),
        });
    }
    None
}

fn parse_average_options(value: &serde_json::Value) -> Option<CfAverageRuleOptions> {
    let object = value.as_object()?;
    let include_equal = object
        .get("include_equal")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let stddev_multiplier = object.get("stddev_multiplier").and_then(|v| v.as_f64());
    Some(CfAverageRuleOptions {
        include_equal,
        stddev_multiplier,
    })
}

fn parse_threshold(value: &serde_json::Value) -> Option<CfThreshold> {
    if let Some(label) = value.as_str() {
        return match label {
            "Min" => Some(CfThreshold::Min),
            "Mid" => Some(CfThreshold::Mid),
            "Max" => Some(CfThreshold::Max),
            _ => None,
        };
    }
    let object = value.as_object()?;
    if let Some(num) = object.get("Number").and_then(|v| v.as_f64()) {
        return Some(CfThreshold::Number(num));
    }
    if let Some(num) = object.get("Percent").and_then(|v| v.as_f64()) {
        return Some(CfThreshold::Percent(num));
    }
    if let Some(num) = object.get("Percentile").and_then(|v| v.as_f64()) {
        return Some(CfThreshold::Percentile(num));
    }
    None
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Parse a `.dnafml` (or `.xml`) byte form back into a `Scenario`.
///
/// The reader prefers the `<dna:Formula>` extension when present (full
/// fidelity). When absent — e.g. file was saved by Excel after a round
/// trip — the parser falls back to the `<Worksheet>` cell text and
/// fills sensible defaults for everything else, surfacing a
/// `FormulaFileError::NotADnaFormula` only when the file isn't even
/// recognisable as SpreadsheetML 2003. (Slice 3 will replace the
/// hard error with a soft "imported from Excel-only file" load
/// diagnostic; today the partial-fallback path is on but the warning
/// channel is not yet plumbed.)
pub fn read_formula_xml(xml: &str) -> Result<LoadedFormula, FormulaFileError> {
    let document =
        Document::parse(xml).map_err(|error| FormulaFileError::Parse(error.to_string()))?;

    let workbook = document.root_element();
    if workbook.tag_name().name() != "Workbook" {
        return Err(FormulaFileError::NotADnaFormula(format!(
            "root element is `{}`, expected `Workbook`",
            workbook.tag_name().name(),
        )));
    }

    let dna_formula = find_child_in_namespace(workbook, DNA_NAMESPACE, "Formula");
    let bundles = read_compare_bundles(workbook);
    let unknown_root_xml = collect_unknown_root_xml(xml, workbook);

    if let Some(dna_formula) = dna_formula {
        let mut scenario = read_with_dna_formula(workbook, dna_formula, bundles)?;
        scenario.unknown_root_xml = unknown_root_xml;
        return Ok(LoadedFormula {
            scenario,
            diagnostics: Vec::new(),
        });
    }

    // Excel-only fallback: pull from the worksheet cell.
    let mut scenario = read_excel_only(workbook, bundles)?;
    scenario.unknown_root_xml = unknown_root_xml;
    Ok(LoadedFormula {
        scenario,
        diagnostics: vec![LoadDiagnostic::ImportedFromExcelOnlyFile],
    })
}

/// Collect raw outer-XML of any workbook-root element this build
/// does not recognise. Indices are byte ranges into the original
/// source string. Per `Scenario::unknown_root_xml` — only root-
/// level unknowns are captured today.
fn collect_unknown_root_xml(source: &str, workbook: Node<'_, '_>) -> Vec<String> {
    workbook
        .children()
        .filter(|child| child.is_element() && !is_known_workbook_child(*child))
        .filter_map(|node| {
            let range = node.range();
            source.get(range).map(str::to_string)
        })
        .collect()
}

/// Whitelist of workbook-root children this build understands. Any
/// other element is captured into `unknown_root_xml` for verbatim
/// round-trip.
fn is_known_workbook_child(node: Node<'_, '_>) -> bool {
    let tag = node.tag_name();
    let name = tag.name();
    let namespace = tag.namespace();
    if namespace == Some(DNA_NAMESPACE) {
        // dna:Formula and dna:CompareBundle are known; other
        // dna: elements at the root are unknown to this build
        // (forward-compat for future schema extensions like
        // dna:Workspace).
        matches!(name, "Formula" | "CompareBundle")
    } else if namespace == Some(SS_NAMESPACE) || namespace.is_none() {
        // Top-level SpreadsheetML elements this build emits or
        // tolerates: Worksheet, ExcelWorkbook (+x: prefix from
        // the excel namespace, but at the root it's spreadsheet-
        // namespaced in our emitter), Styles, DocumentProperties.
        matches!(
            name,
            "Worksheet" | "ExcelWorkbook" | "Styles" | "DocumentProperties"
        )
    } else if namespace == Some("urn:schemas-microsoft-com:office:office") {
        matches!(name, "DocumentProperties")
    } else if namespace == Some("urn:schemas-microsoft-com:office:excel") {
        matches!(name, "ExcelWorkbook")
    } else {
        false
    }
}

fn read_with_dna_formula(
    workbook: Node<'_, '_>,
    dna_formula: Node<'_, '_>,
    bundles: Vec<CompareBundle>,
) -> Result<Scenario, FormulaFileError> {
    let version = dna_formula
        .attribute("version")
        .unwrap_or(FORMULA_VERSION)
        .to_string();
    if version != FORMULA_VERSION {
        return Err(FormulaFileError::UnsupportedVersion(version));
    }

    let identity = read_identity(dna_formula);
    let entry = read_entry(dna_formula).unwrap_or_else(|| read_excel_entry(workbook));
    let context = read_context(dna_formula).unwrap_or_default();
    let ui_preferences = read_ui_preferences(dna_formula);

    Ok(Scenario {
        identity,
        entry,
        context,
        ui_preferences,
        bundles,
        unknown_root_xml: Vec::new(),
    })
}

fn read_excel_only(
    workbook: Node<'_, '_>,
    bundles: Vec<CompareBundle>,
) -> Result<Scenario, FormulaFileError> {
    let entry = read_excel_entry(workbook);
    Ok(Scenario {
        identity: Identity::default(),
        entry,
        context: Context::default(),
        ui_preferences: UiPreferences::default(),
        bundles,
        unknown_root_xml: Vec::new(),
    })
}

fn read_compare_bundles(workbook: Node<'_, '_>) -> Vec<CompareBundle> {
    let mut bundles: Vec<CompareBundle> = workbook
        .children()
        .filter(|child| {
            child.is_element()
                && child.tag_name().namespace() == Some(DNA_NAMESPACE)
                && child.tag_name().name() == "CompareBundle"
        })
        .map(read_compare_bundle)
        .collect();
    // Defensive: enforce chronological-ascending order regardless of
    // the input. Per §11.9 of the plan this is the canonical order.
    bundles.sort_by(|left, right| left.compared_at.cmp(&right.compared_at));
    bundles
}

fn read_compare_bundle(node: Node<'_, '_>) -> CompareBundle {
    CompareBundle {
        bundle_id: node.attribute("bundle-id").unwrap_or_default().to_string(),
        compared_at: node
            .attribute("compared-at")
            .unwrap_or_default()
            .to_string(),
        excel_host_id: node
            .attribute("excel-host-id")
            .unwrap_or_default()
            .to_string(),
        for_formula_state: node
            .attribute("for-formula-state")
            .unwrap_or_default()
            .to_string(),
        value_verdict: BundleVerdict::parse(node.attribute("value-verdict").unwrap_or("unknown")),
        display_verdict: BundleVerdict::parse(
            node.attribute("display-verdict").unwrap_or("unknown"),
        ),
        replay_verdict: BundleVerdict::parse(node.attribute("replay-verdict").unwrap_or("unknown")),
        summary: find_child_in_namespace(node, DNA_NAMESPACE, "Summary")
            .and_then(|summary| summary.text())
            .map(ToOwned::to_owned),
    }
}

fn read_identity(dna_formula: Node<'_, '_>) -> Identity {
    let identity_node = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "Identity");
    let Some(identity_node) = identity_node else {
        return Identity::default();
    };
    Identity {
        id: identity_node
            .attribute("id")
            .unwrap_or_default()
            .to_string(),
        name: identity_node
            .attribute("name")
            .unwrap_or_default()
            .to_string(),
        created_at: identity_node
            .attribute("created-at")
            .unwrap_or_default()
            .to_string(),
        modified_at: identity_node
            .attribute("modified-at")
            .unwrap_or_default()
            .to_string(),
    }
}

fn read_entry(dna_formula: Node<'_, '_>) -> Option<Entry> {
    let entry_node = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "Entry")?;
    let mode = entry_node
        .attribute("mode")
        .and_then(EntryMode::parse)
        .unwrap_or_default();
    let text = entry_node.text().unwrap_or("").to_string();
    Some(Entry { mode, text })
}

fn read_excel_entry(workbook: Node<'_, '_>) -> Entry {
    let cell = find_first_cell(workbook);
    let Some(cell) = cell else {
        return Entry::default();
    };
    if let Some(formula) = cell_attribute_in_namespace(cell, SS_NAMESPACE, "Formula") {
        return Entry {
            mode: EntryMode::Formula,
            text: formula.to_string(),
        };
    }
    let data = cell
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "Data");
    let Some(data) = data else {
        return Entry::default();
    };
    let data_text = data.text().unwrap_or("").to_string();
    let data_type = cell_attribute_in_namespace(data, SS_NAMESPACE, "Type").unwrap_or("String");
    let mode = match data_type {
        _ if data_text.is_empty() => EntryMode::Empty,
        "Number" => EntryMode::Value,
        "Boolean" => EntryMode::Value,
        _ => EntryMode::Text,
    };
    Entry {
        mode,
        text: data_text,
    }
}

fn find_first_cell<'a>(workbook: Node<'a, '_>) -> Option<Node<'a, 'a>> {
    workbook
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "Cell")
}

fn read_context(dna_formula: Node<'_, '_>) -> Option<Context> {
    let context_node = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "Context")?;
    let host_profile = read_host_profile(context_node);
    let locale = read_locale(context_node);
    let publication_context = read_publication_context(context_node);
    let scenario_policy = find_child_in_namespace(context_node, DNA_NAMESPACE, "ScenarioPolicy")
        .and_then(|node| node.text())
        .and_then(ScenarioPolicy::parse)
        .unwrap_or_default();
    Some(Context {
        host_profile,
        locale,
        publication_context,
        scenario_policy,
    })
}

fn read_host_profile(context_node: Node<'_, '_>) -> HostProfile {
    let Some(node) = find_child_in_namespace(context_node, DNA_NAMESPACE, "HostProfile") else {
        return HostProfile::default();
    };
    HostProfile {
        profile_id: node.attribute("profile-id").unwrap_or_default().to_string(),
        requires_excel_observation: parse_bool_attr(node, "requires-excel-observation"),
    }
}

fn read_locale(context_node: Node<'_, '_>) -> Locale {
    let Some(node) = find_child_in_namespace(context_node, DNA_NAMESPACE, "Locale") else {
        return Locale::default();
    };
    Locale {
        id: node.attribute("id").unwrap_or_default().to_string(),
        date1904: parse_bool_attr(node, "date1904"),
    }
}

fn read_publication_context(context_node: Node<'_, '_>) -> PublicationContext {
    let Some(node) = find_child_in_namespace(context_node, DNA_NAMESPACE, "PublicationContext")
    else {
        return PublicationContext::default();
    };
    let style_hierarchy = find_child_in_namespace(node, DNA_NAMESPACE, "StyleHierarchy")
        .map(|hierarchy| {
            hierarchy
                .children()
                .filter(|child| {
                    child.is_element()
                        && child.tag_name().namespace() == Some(DNA_NAMESPACE)
                        && child.tag_name().name() == "StyleLevel"
                })
                .map(|level| level.attribute("id").unwrap_or_default().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let cf_rules = find_child_in_namespace(node, DNA_NAMESPACE, "CfRules")
        .map(|rules| {
            rules
                .children()
                .filter(|child| {
                    child.is_element()
                        && child.tag_name().namespace() == Some(DNA_NAMESPACE)
                        && child.tag_name().name() == "CfRule"
                })
                .map(|rule| {
                    let thresholds = rule
                        .children()
                        .filter(|child| {
                            child.is_element()
                                && child.tag_name().namespace() == Some(DNA_NAMESPACE)
                                && child.tag_name().name() == "Threshold"
                        })
                        .map(|threshold| threshold.text().unwrap_or_default().to_string())
                        .collect::<Vec<_>>();
                    let typed_rule = find_child_in_namespace(rule, DNA_NAMESPACE, "TypedRule")
                        .and_then(|node| node.text())
                        .and_then(parse_cf_typed_rule_json);
                    CfRule {
                        range: rule.attribute("range").unwrap_or_default().to_string(),
                        formula: rule.attribute("formula").map(ToOwned::to_owned),
                        rule_kind: rule.attribute("rule-kind").map(ToOwned::to_owned),
                        operator: rule.attribute("operator").map(ToOwned::to_owned),
                        thresholds,
                        font_color: rule.attribute("font-color").map(ToOwned::to_owned),
                        fill_color: rule.attribute("fill-color").map(ToOwned::to_owned),
                        typed_rule,
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    PublicationContext {
        format_profile: node
            .attribute("format-profile")
            .unwrap_or_default()
            .to_string(),
        number_format_code: node
            .attribute("number-format-code")
            .unwrap_or_default()
            .to_string(),
        style_id: node.attribute("style-id").unwrap_or_default().to_string(),
        font_color: node.attribute("font-color").unwrap_or_default().to_string(),
        fill_color: node.attribute("fill-color").unwrap_or_default().to_string(),
        style_hierarchy,
        cf_rules,
    }
}

fn read_ui_preferences(dna_formula: Node<'_, '_>) -> UiPreferences {
    let Some(node) = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "UiPreferences") else {
        return UiPreferences::default();
    };
    UiPreferences {
        formula_drill_expanded: parse_bool_attr(node, "formula-drill-expanded"),
        result_drill_expanded: parse_bool_attr(node, "result-drill-expanded"),
        expanded_editor: parse_bool_attr(node, "expanded-editor"),
    }
}

fn parse_bool_attr(node: Node<'_, '_>, attr: &str) -> bool {
    matches!(
        node.attribute(attr).map(str::to_ascii_lowercase).as_deref(),
        Some("true" | "1" | "yes")
    )
}

fn find_child_in_namespace<'a>(
    parent: Node<'a, '_>,
    namespace: &str,
    local_name: &str,
) -> Option<Node<'a, 'a>> {
    parent.children().find(|child| {
        child.is_element()
            && child.tag_name().namespace() == Some(namespace)
            && child.tag_name().name() == local_name
    })
}

fn cell_attribute_in_namespace<'a>(
    node: Node<'a, '_>,
    namespace: &str,
    local_name: &str,
) -> Option<&'a str> {
    node.attributes().find_map(|attr| {
        if attr.namespace() == Some(namespace) && attr.name() == local_name {
            Some(attr.value())
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_full_scenario() -> Scenario {
        Scenario {
            identity: Identity {
                id: "invoice-eu-tax".to_string(),
                name: "invoice-eu-tax".to_string(),
                created_at: "2026-04-22T10:14:22Z".to_string(),
                modified_at: "2026-04-26T14:22:01Z".to_string(),
            },
            entry: Entry {
                mode: EntryMode::Formula,
                text: "=SUM(1,2,3)".to_string(),
            },
            context: Context {
                host_profile: HostProfile {
                    profile_id: "Excel365Win".to_string(),
                    requires_excel_observation: true,
                },
                locale: Locale {
                    id: "EnUs".to_string(),
                    date1904: false,
                },
                publication_context: PublicationContext {
                    format_profile: String::new(),
                    number_format_code: "€ #,##0.00".to_string(),
                    style_id: String::new(),
                    font_color: String::new(),
                    fill_color: String::new(),
                    style_hierarchy: vec!["base".to_string(), "currency".to_string()],
                    cf_rules: vec![CfRule {
                        range: "A1".to_string(),
                        formula: Some("=A1>0".to_string()),
                        rule_kind: Some("CellIs".to_string()),
                        operator: None,
                        thresholds: Vec::new(),
                        font_color: None,
                        fill_color: None,
                        typed_rule: None,
                    }],
                },
                scenario_policy: ScenarioPolicy::Deterministic,
            },
            ui_preferences: UiPreferences {
                formula_drill_expanded: false,
                result_drill_expanded: true,
                expanded_editor: false,
            },
            bundles: Vec::new(),
            unknown_root_xml: Vec::new(),
        }
    }

    fn round_trip(scenario: &Scenario) -> Scenario {
        let xml = write_formula_xml(scenario);
        read_formula_xml(&xml)
            .expect("round-trip parse must succeed")
            .scenario
    }

    #[test]
    fn full_scenario_round_trips_verbatim() {
        let scenario = sample_full_scenario();
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn empty_scenario_round_trips() {
        let scenario = Scenario::default();
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn formula_text_with_all_xml_metacharacters_round_trips() {
        let mut scenario = sample_full_scenario();
        scenario.entry.text = r#"=IF(A1<5, "<&>'\"", "OK")"#.to_string();
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn unicode_formula_text_round_trips() {
        let mut scenario = sample_full_scenario();
        scenario.entry.text = "=\"日本語と数式 → 結果\"".to_string();
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn value_entry_with_numeric_text_writes_number_data_type() {
        let scenario = Scenario {
            entry: Entry {
                mode: EntryMode::Value,
                text: "12345.67".to_string(),
            },
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        assert!(
            xml.contains(r#"<Data ss:Type="Number">12345.67</Data>"#),
            "expected Number-typed Data; got xml:\n{xml}",
        );
        let restored = read_formula_xml(&xml).expect("round-trip").scenario;
        assert_eq!(restored.entry.text, "12345.67");
        assert_eq!(restored.entry.mode, EntryMode::Value);
    }

    #[test]
    fn text_entry_strips_apostrophe_in_excel_cell_but_preserves_in_dna_entry() {
        let scenario = Scenario {
            entry: Entry {
                mode: EntryMode::Text,
                text: "'42".to_string(),
            },
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        // Excel cell sees `42` (no leading apostrophe).
        assert!(
            xml.contains(r#"<Data ss:Type="String">42</Data>"#),
            "expected leading apostrophe stripped from cell; got xml:\n{xml}",
        );
        // dna:Entry preserves the raw `'42`.
        assert!(
            xml.contains("<dna:Entry mode=\"Text\">&apos;42</dna:Entry>"),
            "expected dna:Entry to preserve the raw '42; got xml:\n{xml}",
        );
        let restored = read_formula_xml(&xml).expect("round-trip").scenario;
        assert_eq!(restored.entry.text, "'42");
        assert_eq!(restored.entry.mode, EntryMode::Text);
    }

    #[test]
    fn dna_formula_wins_when_excel_cell_diverges() {
        // Excel-side cell reads `=ABS(1)`; dna:Entry reads `=SUM(1,2)`.
        // Per §5.3 of the plan the dna: branch wins for fields Excel
        // could not have edited; for the formula text both can plausibly
        // edit it, so we accept whichever the writer produces. This test
        // pins the current behaviour: dna:Entry wins on read.
        let xml = format!(
            r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:dna="{dna}">
  <Worksheet ss:Name="Formula">
    <Table>
      <Row>
        <Cell ss:Formula="=ABS(1)"><Data ss:Type="String"></Data></Cell>
      </Row>
    </Table>
  </Worksheet>
  <dna:Formula version="1">
    <dna:Entry mode="Formula">=SUM(1,2)</dna:Entry>
  </dna:Formula>
</Workbook>
"#,
            dna = DNA_NAMESPACE
        );
        let scenario = read_formula_xml(&xml).expect("parse").scenario;
        assert_eq!(scenario.entry.text, "=SUM(1,2)");
    }

    #[test]
    fn excel_only_fallback_when_dna_formula_absent_pulls_cell_into_entry() {
        let xml = r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
  <Worksheet ss:Name="Sheet1">
    <Table>
      <Row>
        <Cell ss:Formula="=SUM(7,8)"><Data ss:Type="String"></Data></Cell>
      </Row>
    </Table>
  </Worksheet>
</Workbook>
"#;
        let loaded = read_formula_xml(xml).expect("parse");
        let scenario = loaded.scenario;
        assert_eq!(scenario.entry.text, "=SUM(7,8)");
        assert_eq!(scenario.entry.mode, EntryMode::Formula);
        // Identity / context / ui-prefs default since dna: extension was absent.
        assert_eq!(scenario.identity, Identity::default());
        assert_eq!(scenario.context, Context::default());
        assert_eq!(scenario.ui_preferences, UiPreferences::default());
        // The Excel-only fallback path raises the warning chip.
        assert_eq!(
            loaded.diagnostics,
            vec![LoadDiagnostic::ImportedFromExcelOnlyFile],
        );
    }

    // -----------------------------------------------------------------
    // Forward-compat: unknown-element preservation
    // -----------------------------------------------------------------

    #[test]
    fn unknown_dna_root_element_round_trips_verbatim() {
        // Future schema extension: imagine v2 adds <dna:Workspace>
        // as a workbook-root element. This v1 build must not
        // silently drop it on save.
        let xml = format!(
            r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:dna="{dna}">
  <dna:Formula version="1">
    <dna:Entry mode="Formula">=A1</dna:Entry>
  </dna:Formula>
  <dna:Workspace name="future-extension"><dna:Tab id="t1"/></dna:Workspace>
</Workbook>
"#,
            dna = DNA_NAMESPACE,
        );
        let loaded = read_formula_xml(&xml).expect("parse");
        assert_eq!(loaded.scenario.unknown_root_xml.len(), 1);
        let preserved = &loaded.scenario.unknown_root_xml[0];
        assert!(
            preserved.contains("dna:Workspace") && preserved.contains("future-extension"),
            "unknown element must be preserved verbatim; got {preserved:?}",
        );

        let rewritten = write_formula_xml(&loaded.scenario);
        assert!(
            rewritten.contains("dna:Workspace") && rewritten.contains("future-extension"),
            "rewriter must re-emit the unknown element verbatim; got xml:\n{rewritten}",
        );
    }

    #[test]
    fn foreign_namespace_root_element_round_trips_verbatim() {
        // A third-party tool might add a workbook-root element in
        // its own namespace (e.g. some extension's metadata block).
        // We preserve it.
        let xml = r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:thirdparty="urn:thirdparty:tool:1">
  <thirdparty:Annotation key="value">third-party content</thirdparty:Annotation>
</Workbook>
"#;
        let loaded = read_formula_xml(xml).expect("parse");
        assert_eq!(loaded.scenario.unknown_root_xml.len(), 1);
        let preserved = &loaded.scenario.unknown_root_xml[0];
        assert!(
            preserved.contains("thirdparty:Annotation"),
            "got {preserved:?}",
        );

        let rewritten = write_formula_xml(&loaded.scenario);
        assert!(
            rewritten.contains("thirdparty:Annotation"),
            "rewriter must re-emit the foreign-namespace element verbatim; got xml:\n{rewritten}",
        );
    }

    #[test]
    fn known_root_elements_are_not_captured_into_unknowns() {
        // Worksheet / ExcelWorkbook / Styles / dna:Formula /
        // dna:CompareBundle / DocumentProperties are all in the
        // whitelist. A file containing only those should yield an
        // empty unknown-root list.
        let scenario = sample_full_scenario();
        let xml = write_formula_xml(&scenario);
        let loaded = read_formula_xml(&xml).expect("parse");
        assert!(
            loaded.scenario.unknown_root_xml.is_empty(),
            "known elements must not be captured as unknowns; got {:?}",
            loaded.scenario.unknown_root_xml,
        );
    }

    #[test]
    fn full_dna_load_carries_no_diagnostics() {
        let scenario = sample_full_scenario();
        let xml = write_formula_xml(&scenario);
        let loaded = read_formula_xml(&xml).expect("parse");
        assert!(
            loaded.diagnostics.is_empty(),
            "full dna: load must not surface a diagnostic; got {:?}",
            loaded.diagnostics,
        );
    }

    #[test]
    fn unknown_dna_formula_version_returns_unsupported_error() {
        let xml = format!(
            r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:dna="{dna}">
  <dna:Formula version="9999">
    <dna:Entry mode="Formula">=A1</dna:Entry>
  </dna:Formula>
</Workbook>
"#,
            dna = DNA_NAMESPACE
        );
        let result = read_formula_xml(&xml);
        match result {
            Err(FormulaFileError::UnsupportedVersion(version)) => assert_eq!(version, "9999"),
            other => panic!("expected UnsupportedVersion(9999), got {other:?}"),
        }
    }

    #[test]
    fn malformed_xml_returns_parse_error() {
        let result = read_formula_xml("<not-well-formed");
        assert!(matches!(result, Err(FormulaFileError::Parse(_))));
    }

    #[test]
    fn non_workbook_root_returns_not_a_dna_formula_error() {
        let result = read_formula_xml(r#"<root xmlns="x"></root>"#);
        match result {
            Err(FormulaFileError::NotADnaFormula(message)) => {
                assert!(message.contains("root"), "got message: {message}");
            }
            other => panic!("expected NotADnaFormula, got {other:?}"),
        }
    }

    #[test]
    fn output_starts_with_xml_declaration_and_mso_application_pi() {
        let scenario = sample_full_scenario();
        let xml = write_formula_xml(&scenario);
        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="utf-8"?>"#));
        assert!(xml.contains(r#"<?mso-application progid="Excel.Sheet"?>"#));
    }

    #[test]
    fn output_contains_dna_namespace_declaration_at_workbook_root() {
        let xml = write_formula_xml(&sample_full_scenario());
        assert!(xml.contains(r#"xmlns:dna="urn:dnakode:dnaonecalc:formula:1""#));
    }

    #[test]
    fn round_trip_with_publication_context_style_hierarchy_and_cf_rules() {
        let scenario = sample_full_scenario();
        let restored = round_trip(&scenario);
        assert_eq!(
            restored.context.publication_context.style_hierarchy,
            scenario.context.publication_context.style_hierarchy,
        );
        assert_eq!(
            restored.context.publication_context.cf_rules,
            scenario.context.publication_context.cf_rules,
        );
    }

    #[test]
    fn round_trip_with_live_recalc_scenario_policy() {
        let mut scenario = sample_full_scenario();
        scenario.context.scenario_policy = ScenarioPolicy::LiveRecalc;
        let restored = round_trip(&scenario);
        assert_eq!(restored.context.scenario_policy, ScenarioPolicy::LiveRecalc);
    }

    /// Cover the W073 typed CF rule round-trip. Each of the five
    /// supported families (color scale / data bar / icon set / rank /
    /// average) plus the threshold variants must survive a full
    /// XML write+read pass via the `<dna:TypedRule>` JSON envelope.
    #[test]
    fn round_trip_with_typed_color_scale_rule() {
        let mut scenario = Scenario::default();
        scenario.context.publication_context.cf_rules = vec![CfRule {
            range: String::new(),
            formula: None,
            rule_kind: Some("colorScale".to_string()),
            operator: None,
            thresholds: vec![
                "min:#F8696B".to_string(),
                "mid:#FFEB84".to_string(),
                "max:#63BE7B".to_string(),
            ],
            font_color: None,
            fill_color: None,
            typed_rule: Some(CfTypedRule {
                color_scale: Some(CfColorScaleRuleOptions {
                    stops: vec![
                        CfColorScaleStop {
                            position: CfThreshold::Min,
                            color: "#F8696B".to_string(),
                        },
                        CfColorScaleStop {
                            position: CfThreshold::Percentile(50.0),
                            color: "#FFEB84".to_string(),
                        },
                        CfColorScaleStop {
                            position: CfThreshold::Max,
                            color: "#63BE7B".to_string(),
                        },
                    ],
                }),
                ..CfTypedRule::default()
            }),
        }];
        let restored = round_trip(&scenario);
        assert_eq!(
            restored.context.publication_context.cf_rules,
            scenario.context.publication_context.cf_rules,
        );
    }

    #[test]
    fn round_trip_with_typed_data_bar_rule() {
        let mut scenario = Scenario::default();
        scenario.context.publication_context.cf_rules = vec![CfRule {
            range: String::new(),
            formula: None,
            rule_kind: Some("dataBar".to_string()),
            operator: None,
            thresholds: Vec::new(),
            font_color: None,
            fill_color: Some("#638EC6".to_string()),
            typed_rule: Some(CfTypedRule {
                data_bar: Some(CfDataBarRuleOptions {
                    minimum: Some(CfThreshold::Min),
                    maximum: Some(CfThreshold::Max),
                    bar_color: Some("#638EC6".to_string()),
                    direction: Some(CfDataBarDirection::Right),
                    show_bar_only: true,
                }),
                ..CfTypedRule::default()
            }),
        }];
        let restored = round_trip(&scenario);
        assert_eq!(
            restored.context.publication_context.cf_rules,
            scenario.context.publication_context.cf_rules,
        );
    }

    #[test]
    fn round_trip_with_typed_icon_set_rule() {
        let mut scenario = Scenario::default();
        scenario.context.publication_context.cf_rules = vec![CfRule {
            range: String::new(),
            formula: None,
            rule_kind: Some("iconSet".to_string()),
            operator: None,
            thresholds: vec!["3Arrows".to_string()],
            font_color: None,
            fill_color: None,
            typed_rule: Some(CfTypedRule {
                icon_set: Some(CfIconSetRuleOptions {
                    set_kind: "3Arrows".to_string(),
                    thresholds: vec![CfThreshold::Percent(33.0), CfThreshold::Percent(67.0)],
                }),
                ..CfTypedRule::default()
            }),
        }];
        let restored = round_trip(&scenario);
        assert_eq!(
            restored.context.publication_context.cf_rules,
            scenario.context.publication_context.cf_rules,
        );
    }

    #[test]
    fn round_trip_with_typed_rank_rule() {
        let mut scenario = Scenario::default();
        scenario.context.publication_context.cf_rules = vec![
            CfRule {
                rule_kind: Some("top".to_string()),
                thresholds: vec!["10".to_string()],
                typed_rule: Some(CfTypedRule {
                    rank: Some(CfRankRuleOptions {
                        rank: CfRank::Count(10),
                    }),
                    ..CfTypedRule::default()
                }),
                ..CfRule::default()
            },
            CfRule {
                rule_kind: Some("bottom".to_string()),
                thresholds: vec!["5".to_string()],
                typed_rule: Some(CfTypedRule {
                    rank: Some(CfRankRuleOptions {
                        rank: CfRank::Percent(5.0),
                    }),
                    ..CfTypedRule::default()
                }),
                ..CfRule::default()
            },
        ];
        let restored = round_trip(&scenario);
        assert_eq!(
            restored.context.publication_context.cf_rules,
            scenario.context.publication_context.cf_rules,
        );
    }

    #[test]
    fn round_trip_with_typed_average_rule() {
        let mut scenario = Scenario::default();
        scenario.context.publication_context.cf_rules = vec![CfRule {
            rule_kind: Some("aboveAverage".to_string()),
            typed_rule: Some(CfTypedRule {
                average: Some(CfAverageRuleOptions {
                    include_equal: true,
                    stddev_multiplier: Some(1.5),
                }),
                ..CfTypedRule::default()
            }),
            ..CfRule::default()
        }];
        let restored = round_trip(&scenario);
        assert_eq!(
            restored.context.publication_context.cf_rules,
            scenario.context.publication_context.cf_rules,
        );
    }

    /// CfRule with no typed_rule emits no `<dna:TypedRule>` element so
    /// older readers that don't know about the typed payload continue
    /// to parse the bounded-string convention without surprise.
    #[test]
    fn cf_rule_without_typed_rule_emits_no_typed_rule_element() {
        let scenario = Scenario {
            context: Context {
                publication_context: PublicationContext {
                    cf_rules: vec![CfRule {
                        rule_kind: Some("cell_value".to_string()),
                        operator: Some("greaterThan".to_string()),
                        thresholds: vec!["0".to_string()],
                        ..CfRule::default()
                    }],
                    ..PublicationContext::default()
                },
                ..Context::default()
            },
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        assert!(
            !xml.contains("<dna:TypedRule"),
            "expected no TypedRule element when typed_rule is None; got:\n{xml}",
        );
    }

    #[test]
    fn entry_mode_round_trips_for_all_four_variants() {
        for &mode in &[
            EntryMode::Formula,
            EntryMode::Value,
            EntryMode::Text,
            EntryMode::Empty,
        ] {
            let scenario = Scenario {
                entry: Entry {
                    mode,
                    text: match mode {
                        EntryMode::Formula => "=1+1".to_string(),
                        EntryMode::Value => "42".to_string(),
                        EntryMode::Text => "'hello".to_string(),
                        EntryMode::Empty => String::new(),
                    },
                },
                ..Scenario::default()
            };
            let restored = round_trip(&scenario);
            assert_eq!(restored.entry.mode, mode, "mode {mode:?}");
            assert_eq!(
                restored.entry.text, scenario.entry.text,
                "text for {mode:?}"
            );
        }
    }

    // -----------------------------------------------------------------
    // Compare bundles (slice 4)
    // -----------------------------------------------------------------

    fn sample_bundle(
        id: &str,
        compared_at: &str,
        excel_host: &str,
        for_state: &str,
    ) -> CompareBundle {
        CompareBundle {
            bundle_id: id.to_string(),
            compared_at: compared_at.to_string(),
            excel_host_id: excel_host.to_string(),
            for_formula_state: for_state.to_string(),
            value_verdict: BundleVerdict::Match,
            display_verdict: BundleVerdict::Mismatch,
            replay_verdict: BundleVerdict::Equivalent,
            summary: Some("display: thousands separator differs".to_string()),
        }
    }

    #[test]
    fn empty_bundle_list_emits_nothing_extra() {
        let scenario = Scenario::default();
        let xml = write_formula_xml(&scenario);
        assert!(
            !xml.contains("<dna:CompareBundle"),
            "empty bundle list must not emit CompareBundle elements; got xml:\n{xml}",
        );
    }

    #[test]
    fn single_bundle_round_trips_with_attributes_and_summary() {
        let scenario = Scenario {
            bundles: vec![sample_bundle(
                "cb-2026-04-26T1430-Excel365Win",
                "2026-04-26T14:30:11Z",
                "Excel365Win-16.0.18025",
                "sha256:abcd1234",
            )],
            ..Scenario::default()
        };
        let restored = round_trip(&scenario);
        assert_eq!(restored.bundles, scenario.bundles);
    }

    #[test]
    fn bundle_without_summary_round_trips_as_self_closing_element() {
        let mut bundle = sample_bundle("cb-1", "2026-04-26T14:30:11Z", "Excel365Win", "");
        bundle.summary = None;
        let scenario = Scenario {
            bundles: vec![bundle.clone()],
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        assert!(
            xml.contains("<dna:CompareBundle") && xml.contains("/>"),
            "no-summary bundle should emit a self-closing element; got xml:\n{xml}",
        );
        let restored = round_trip(&scenario);
        assert_eq!(restored.bundles, vec![bundle]);
    }

    #[test]
    fn multiple_bundles_round_trip_in_chronological_ascending_order() {
        let scenario = Scenario {
            bundles: vec![
                sample_bundle(
                    "cb-old",
                    "2026-04-22T10:14:22Z",
                    "Excel365Win",
                    "sha256:older",
                ),
                sample_bundle("cb-mid", "2026-04-23T10:14:22Z", "ExcelMac", "sha256:older"),
                sample_bundle(
                    "cb-new",
                    "2026-04-26T14:30:11Z",
                    "Excel365Win",
                    "sha256:newer",
                ),
            ],
            ..Scenario::default()
        };
        let restored = round_trip(&scenario);
        let restored_ids: Vec<_> = restored
            .bundles
            .iter()
            .map(|bundle| bundle.bundle_id.as_str())
            .collect();
        assert_eq!(restored_ids, vec!["cb-old", "cb-mid", "cb-new"]);
    }

    #[test]
    fn reader_sorts_out_of_order_bundles_chronological_ascending() {
        // Even if a tampered file emits bundles out of order, the
        // reader normalises them to chronological-ascending per
        // §11.9.
        let scenario = Scenario {
            bundles: vec![
                sample_bundle("cb-c", "2026-04-26T14:30:11Z", "host", "state"),
                sample_bundle("cb-a", "2026-04-22T10:14:22Z", "host", "state"),
                sample_bundle("cb-b", "2026-04-23T10:14:22Z", "host", "state"),
            ],
            ..Scenario::default()
        };
        let restored = round_trip(&scenario);
        let restored_ids: Vec<_> = restored
            .bundles
            .iter()
            .map(|bundle| bundle.bundle_id.as_str())
            .collect();
        // Reader sorted by compared-at; the original "out of order"
        // input is now ascending. The retention helper would dedup
        // the (state, host) pair, but the reader itself does not.
        assert_eq!(restored_ids, vec!["cb-a", "cb-b", "cb-c"]);
    }

    #[test]
    fn unknown_verdict_strings_round_trip_as_unknown() {
        let scenario = Scenario {
            bundles: vec![CompareBundle {
                bundle_id: "cb-unknown".to_string(),
                compared_at: "2026-04-26T14:30:11Z".to_string(),
                excel_host_id: "host".to_string(),
                for_formula_state: "state".to_string(),
                value_verdict: BundleVerdict::Unknown,
                display_verdict: BundleVerdict::Blocked,
                replay_verdict: BundleVerdict::Unknown,
                summary: None,
            }],
            ..Scenario::default()
        };
        let restored = round_trip(&scenario);
        assert_eq!(restored.bundles[0].value_verdict, BundleVerdict::Unknown,);
        assert_eq!(restored.bundles[0].display_verdict, BundleVerdict::Blocked,);
    }

    #[test]
    fn bundle_summary_with_xml_metacharacters_round_trips() {
        let mut bundle = sample_bundle("cb-1", "2026-04-26T14:30:11Z", "host", "state");
        bundle.summary = Some(r#"display mismatch: "<&>'\""#.to_string());
        let scenario = Scenario {
            bundles: vec![bundle.clone()],
            ..Scenario::default()
        };
        let restored = round_trip(&scenario);
        assert_eq!(restored.bundles, vec![bundle]);
    }

    // ----- retention policy (§9.5) --------------------------------------

    #[test]
    fn retention_policy_keeps_live_bundle_for_current_formula_state() {
        let mut bundles = vec![
            sample_bundle("cb-1", "t1", "host", "old-state"),
            sample_bundle("cb-2", "t2", "host", "new-state"),
        ];
        apply_bundle_retention_policy(&mut bundles, "new-state", 10);
        let ids: Vec<_> = bundles
            .iter()
            .map(|bundle| bundle.bundle_id.as_str())
            .collect();
        assert_eq!(ids, vec!["cb-1", "cb-2"]);
    }

    #[test]
    fn retention_policy_dedups_pair_keeping_most_recent() {
        // Two bundles for same (formula_state, host) — keep only the
        // most recent. A run with no significant change updates the
        // existing bundle's compared-at in place per §9.2.
        let mut bundles = vec![
            sample_bundle("cb-old-host-a", "t1", "Excel365Win", "state"),
            sample_bundle("cb-new-host-a", "t2", "Excel365Win", "state"),
            sample_bundle("cb-old-host-b", "t1", "ExcelMac", "state"),
        ];
        apply_bundle_retention_policy(&mut bundles, "state", 10);
        let ids: Vec<_> = bundles
            .iter()
            .map(|bundle| bundle.bundle_id.as_str())
            .collect();
        assert!(ids.contains(&"cb-new-host-a"));
        assert!(ids.contains(&"cb-old-host-b"));
        assert!(!ids.contains(&"cb-old-host-a"));
    }

    #[test]
    fn retention_policy_caps_history_only_bundles_keeping_oldest_dropped_first() {
        let mut bundles: Vec<CompareBundle> = (0..15)
            .map(|n| {
                sample_bundle(
                    &format!("cb-{n:02}"),
                    &format!("2026-04-{:02}", 1 + n),
                    &format!("host-{n}"),
                    &format!("state-{n}"),
                )
            })
            .collect();
        apply_bundle_retention_policy(&mut bundles, "no-current-state", 10);
        assert_eq!(bundles.len(), 10);
        // Oldest five should be dropped (cb-00..cb-04).
        let ids: Vec<_> = bundles
            .iter()
            .map(|bundle| bundle.bundle_id.as_str())
            .collect();
        assert!(!ids.contains(&"cb-00"));
        assert!(!ids.contains(&"cb-04"));
        assert!(ids.contains(&"cb-05"));
        assert!(ids.contains(&"cb-14"));
    }

    // -----------------------------------------------------------------
    // Excel-native fidelity (slice 2)
    // -----------------------------------------------------------------

    #[test]
    fn excel_workbook_block_is_omitted_when_date1904_is_default_false() {
        let scenario = Scenario::default();
        let xml = write_formula_xml(&scenario);
        assert!(
            !xml.contains("<ExcelWorkbook>"),
            "default Date1904=false must not emit ExcelWorkbook block; got:\n{xml}",
        );
    }

    #[test]
    fn excel_workbook_block_is_emitted_when_date1904_is_true() {
        let scenario = Scenario {
            context: Context {
                locale: Locale {
                    id: "EnUs".to_string(),
                    date1904: true,
                },
                ..Context::default()
            },
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        assert!(
            xml.contains("<ExcelWorkbook>") && xml.contains("<Date1904/>"),
            "Date1904=true must emit ExcelWorkbook/Date1904; got:\n{xml}",
        );
    }

    #[test]
    fn styles_block_is_omitted_when_no_publication_context_styling() {
        let scenario = Scenario::default();
        let xml = write_formula_xml(&scenario);
        assert!(
            !xml.contains("<Styles>"),
            "default scenario must not emit a Styles block; got:\n{xml}",
        );
        assert!(
            !xml.contains("ss:StyleID="),
            "default scenario must not emit ss:StyleID on the cell; got:\n{xml}",
        );
    }

    #[test]
    fn styles_block_emits_native_number_format_font_and_interior() {
        let scenario = Scenario {
            context: Context {
                publication_context: PublicationContext {
                    number_format_code: "€ #,##0.00".to_string(),
                    font_color: "#112233".to_string(),
                    fill_color: "#445566".to_string(),
                    style_id: "calc".to_string(),
                    ..PublicationContext::default()
                },
                ..Context::default()
            },
            entry: Entry {
                mode: EntryMode::Formula,
                text: "=SUM(1,2,3)".to_string(),
            },
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        // The Styles block exists and references the style by id.
        assert!(
            xml.contains("<Styles>"),
            "expected Styles block; got:\n{xml}"
        );
        assert!(
            xml.contains(r##"<Style ss:ID="calc">"##),
            "expected named Style entry; got:\n{xml}",
        );
        assert!(
            xml.contains(r##"<NumberFormat ss:Format="€ #,##0.00"/>"##),
            "expected NumberFormat; got:\n{xml}",
        );
        assert!(
            xml.contains(r##"<Font ss:Color="#112233"/>"##),
            "expected Font color; got:\n{xml}",
        );
        assert!(
            xml.contains(r##"<Interior ss:Color="#445566" ss:Pattern="Solid"/>"##),
            "expected Interior fill with Solid pattern; got:\n{xml}",
        );
        // Cell carries ss:StyleID referencing the style.
        assert!(
            xml.contains(r##"<Cell ss:StyleID="calc""##),
            "expected ss:StyleID on Cell; got:\n{xml}",
        );
    }

    #[test]
    fn styles_block_uses_default_style_id_when_publication_context_style_id_is_blank() {
        let scenario = Scenario {
            context: Context {
                publication_context: PublicationContext {
                    number_format_code: "0.00%".to_string(),
                    ..PublicationContext::default()
                },
                ..Context::default()
            },
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        assert!(
            xml.contains(r#"<Style ss:ID="dna-cell-style">"#),
            "expected default style id; got:\n{xml}",
        );
        assert!(
            xml.contains(r#"ss:StyleID="dna-cell-style""#),
            "expected default style id on Cell; got:\n{xml}",
        );
    }

    #[test]
    fn cf_rules_emit_native_conditional_formatting_inside_worksheet() {
        let scenario = Scenario {
            context: Context {
                publication_context: PublicationContext {
                    cf_rules: vec![CfRule {
                        range: "A1".to_string(),
                        formula: Some("=A1>0".to_string()),
                        rule_kind: Some("Expression".to_string()),
                        operator: None,
                        thresholds: Vec::new(),
                        font_color: None,
                        fill_color: None,
                        typed_rule: None,
                    }],
                    ..PublicationContext::default()
                },
                ..Context::default()
            },
            ..Scenario::default()
        };
        let xml = write_formula_xml(&scenario);
        assert!(
            xml.contains(r#"<ConditionalFormatting ss:Range="A1">"#),
            "expected CF block; got:\n{xml}",
        );
        assert!(
            xml.contains(r#"<Condition ss:Type="Expression" ss:Formula="=A1&gt;0"/>"#),
            "expected Expression-typed Condition with escaped formula; got:\n{xml}",
        );
    }

    #[test]
    fn excel_native_emit_round_trips_through_dna_extension() {
        // Even with native Excel emit on, the dna: extension still
        // round-trips the canonical PublicationContext / Locale
        // values verbatim — so the next OneCalc load reads from the
        // single canonical source.
        let scenario = Scenario {
            context: Context {
                locale: Locale {
                    id: "EnUs".to_string(),
                    date1904: true,
                },
                publication_context: PublicationContext {
                    number_format_code: "€ #,##0.00".to_string(),
                    font_color: "#112233".to_string(),
                    fill_color: "#445566".to_string(),
                    style_id: "calc".to_string(),
                    cf_rules: vec![CfRule {
                        range: "A1".to_string(),
                        formula: Some("=A1>0".to_string()),
                        rule_kind: Some("Expression".to_string()),
                        operator: None,
                        thresholds: Vec::new(),
                        font_color: None,
                        fill_color: None,
                        typed_rule: None,
                    }],
                    ..PublicationContext::default()
                },
                ..Context::default()
            },
            entry: Entry {
                mode: EntryMode::Formula,
                text: "=SUM(1,2,3)".to_string(),
            },
            ..Scenario::default()
        };
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn retention_policy_never_drops_live_bundle_to_satisfy_cap() {
        let mut bundles: Vec<CompareBundle> = (0..15)
            .map(|n| {
                sample_bundle(
                    &format!("cb-history-{n:02}"),
                    &format!("2026-04-{:02}", 1 + n),
                    &format!("host-{n}"),
                    "old-state",
                )
            })
            .collect();
        // One live bundle, oldest in the list — should survive even
        // though it's older than every history bundle.
        let mut live = sample_bundle("cb-live", "2025-01-01", "host", "current-state");
        live.summary = Some("live bundle".to_string());
        bundles.insert(0, live);

        apply_bundle_retention_policy(&mut bundles, "current-state", 10);

        let ids: Vec<_> = bundles
            .iter()
            .map(|bundle| bundle.bundle_id.as_str())
            .collect();
        assert!(
            ids.contains(&"cb-live"),
            "live bundle must never be dropped to satisfy the cap; got {ids:?}",
        );
    }
}
