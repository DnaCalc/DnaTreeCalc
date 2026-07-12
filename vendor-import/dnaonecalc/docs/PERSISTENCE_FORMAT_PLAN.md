# DNA OneCalc Persistence Format Plan — XML Spreadsheet 2003 + Extension Lane

Status: `draft_planning`
Date: 2026-05-02
Scope: file format for the user-facing `formula` work unit (internal name `scenario`)

Companion documents:
1. [SCOPE_AND_SPEC.md §11](SCOPE_AND_SPEC.md) — original persistence
   direction, of which this doc is the implementation plan.
2. [APP_UX_REALIZATION.md §5](APP_UX_REALIZATION.md) — the JSON shape of
   the persisted scenario (pre-XML-container plan).
3. [APP_UX_BRIEF.md §1A](APP_UX_BRIEF.md) — terminology convention
   (`formula` user-facing, `scenario` internal).
4. [WS-14 progressive home plan §6](.claude/plans/revisit-our-ux-guidelines-swift-sphinx.md)
   — scenario lifecycle (single-active, breadcrumb dropdown, save/load,
   recents, pinned).

## 1. Purpose
Decide whether the user-facing `formula` file should be an **Excel-readable
XML Spreadsheet 2003 document with a DnaOneCalc extension lane**, instead
of an opaque JSON. Outcome: yes, with the design captured below.

The decision matters because the file the user keeps on disk is the only
artefact that crosses tool boundaries. If it opens in Excel as-is, the
formula is portable to any teammate, traceable in any backup, and
recoverable from any USB stick — without DnaOneCalc.

## 1A. Single file format: where everything goes

There are two kinds of state, with two different homes:

### 1A.1 Documents the user creates and shares — `.dnafml` / `.xml`
The user-saved formula file is a single XML Spreadsheet 2003 document
with the `dna:` extension lane. Two extensions are accepted as peers:

| Extension | Same content? | Same handler in OneCalc? | Excel double-click opens? | Typical use |
|---|---|---|---|---|
| `.dnafml` | yes | yes | usually no (user picks "Open with…") | "this is my OneCalc formula" — OneCalc-first sharing |
| `.xml` | yes (byte-identical) | yes | yes | "this opens in Excel by default" — share with non-OneCalc users, scripts, archive |

Both extensions resolve to the same loader / saver and to the same
on-disk bytes; the difference is purely the OS-level file association
the user wants. The save dialog offers both as first-class options
(see [§8](#8-file-extension-and-os-association)).

There is **no separate `.dnacomparebundle`**. Compare-with-Excel
evidence that used to be planned as a `.dnacomparebundle` JSON sibling
now lives inside the same `.dnafml` / `.xml` document as an optional
`<dna:CompareBundle>` element — see [§9](#9-compare-bundle-as-an-optional-sibling-element).

Reasoning for the merge:
1. A compare bundle's `scenario_snapshot` field is literally an inlined
   formula. Two files would always travel together.
2. The XML container already has a foreign-namespace extension lane
   Excel ignores; nothing stops us putting the bundle there too.
3. The user mental model is "this is my formula and the work I did on
   it" — one file, not two.
4. The save-bundle action becomes "save the current document with the
   bundle attached," not "write a second file alongside."

### 1A.2 App state that lives wherever the host stores config — `workspace.json`
`workspace.json` is **not a document** the user creates, opens, or
shares. It is the OneCalc host application's own config / state cache,
written to the platform's standard per-user app-data location:

| Host | Path |
|---|---|
| Windows (Tauri) | `%APPDATA%\DnaOneCalc\workspace.json` |
| macOS (Tauri) | `~/Library/Application Support/DnaOneCalc/workspace.json` |
| Linux (Tauri) | `~/.config/DnaOneCalc/workspace.json` |
| Browser host | `localStorage["dnaonecalc.workspace.v1"]` (no file on disk) |

It carries:
1. recents list (paths or session-cache ids of recently opened formulas),
2. pinned list (the user's curated favourites),
3. editor settings the user has changed from defaults,
4. session-cache slots for unsaved scratch formulas (per WS-14 plan §6.5),
5. last-window placement / size / mode preferences.

It does **not** carry per-formula content. Opening a `.dnafml` does not
mutate `workspace.json` except to record the path in the recents list.

Treat `workspace.json` like any other app's config file — it can be
deleted to reset the app to defaults; it is not part of the user's
deliverables; it does not get emailed, archived, or version-controlled
with their formula work. The format is JSON because it is operational
state, not a document, and it is read / written only by OneCalc itself.

### 1A.3 What JSON files exist today
**Zero user-facing JSON files exist on disk today.** The persistence
slice has not been implemented; nothing in the running app writes a
user file or a `workspace.json` yet.

What lives in the codebase:
1. Internal verification-bundle outputs in `target/onecalc-verification/...`
   — scratch artefacts produced when running a verification batch
   (`input-request.json`, `verification-bundle-report.json`,
   `xml-cell-extract.json`, per-case `scenario.json`, replay manifests,
   etc.). These are subsystem audit files, not user-saved documents
   and not app state, and they stay JSON because they're internal
   scratch.
2. The `.dnacomparebundle` JSON envelope was *planned* in
   [APP_UX_REALIZATION §5.2](APP_UX_REALIZATION.md) but never
   implemented. This plan supersedes that design — the bundle
   collapses into the XML file as described in §9.
3. The `.dnascenario` JSON envelope was the original
   [APP_UX_REALIZATION §5.1](APP_UX_REALIZATION.md) shape; this plan
   replaces it with the XML container described below.

### 1A.1 What JSON files exist today
**Zero user-facing JSON files exist on disk today.** The persistence
slice has not been implemented; nothing in the running app writes a
user file yet.

What lives in the codebase:
1. Internal verification-bundle outputs in `target/onecalc-verification/...`
   — scratch artefacts produced when running a verification batch
   (`input-request.json`, `verification-bundle-report.json`,
   `xml-cell-extract.json`, per-case `scenario.json`, replay manifests,
   etc.). These are subsystem audit files, not user-saved documents,
   and they stay JSON because they're internal scratch.
2. The `.dnacomparebundle` JSON envelope was *planned* in
   [APP_UX_REALIZATION §5.2](APP_UX_REALIZATION.md) but never
   implemented. This plan supersedes that design — the bundle
   collapses into the XML file as described in §9.
3. The `.dnascenario` JSON envelope was the original
   [APP_UX_REALIZATION §5.1](APP_UX_REALIZATION.md) shape; this plan
   replaces it with the XML container described below.

## 2. Past discussion summary

### 2.1 What `SCOPE_AND_SPEC §11` already says
1. The initial externally meaningful persistence target is `SpreadsheetML 2003`.
2. Reasons: simpler than OOXML, externally meaningful, matches the Foundation
   reference direction.
3. One XML file means one isolated DnaOneCalc instance.
4. The workbook envelope is an Excel-readable container, **not** a claim
   of workbook semantics.
5. Formatting state and conditional-formatting state must round-trip.
6. **XML extension lanes may be used where they are safe and where Excel
   will harmlessly ignore them.**
7. `ScenarioCapsule` (the comparison-bundle artefact) is a separate
   evidence-transport format and must not be conflated with document
   persistence.

### 2.2 What the round-trip invariants are (`§11.3`)
The first persistence implementation must preserve:
1. formula text,
2. host profile id,
3. host-driving / recalc metadata,
4. persistence-format metadata and attachment refs,
5. base formatting state,
6. conditional-formatting rule carriage for the admitted first subset,
7. retained artefact refs,
8. document or scenario ids.

If any of those cannot round-trip, the loss must be explicit and the
saved artefact must record the projection.

### 2.3 What `APP_UX_REALIZATION §5.1` proposed
A `.dnascenario` JSON envelope with these top-level keys:
- `dnascenario_version`, `id`, `name`, `created_at`, `modified_at`,
- `formula` ( `entered_text` + `entry_mode` ),
- `context` ( `host_profile`, `locale`, `date1904`, `publication_context.{format_profile, number_format_code, style_id, style_hierarchy, font_color, fill_color, cf_rules}`, `scenario_policy` ),
- `ui_preferences`,
- `attached_compare_bundle_path`.

This shape is the data we need to persist. The question this doc
answers is **not what data**, but **what container** carries it.

### 2.4 What code already exists
1. **Read side** — [`services/spreadsheet_xml.rs`](../src/dnaonecalc-host/src/services/spreadsheet_xml.rs)
   parses SpreadsheetML 2003 via `roxmltree`. Extracts: formula text,
   data type, style id, full style hierarchy, number-format code,
   font / fill colour, conditional-format rules, `Date1904` flag.
2. **Write side** — [`services/verification_bundle.rs::write_excel_2003_xml_workbook`](../src/dnaonecalc-host/src/services/verification_bundle.rs)
   already EMITS minimal SpreadsheetML 2003 (one cell, formula or value,
   no styles). Used today for fixture generation, not user persistence.

So the IO substrate already exists. The work is shaping the extension
lane and lifting both sides into the persistence service.

## 3. Format options considered

| Option | Excel opens it? | Carries our metadata? | Round-trip-safe? | Bytes overhead | Implementation work |
|---|---|---|---|---|---|
| `A. Plain JSON .dnafml` | ❌ | ✅ | ✅ | low | small (just serde) |
| `B. JSON .dnafml + sidecar .xml` | ✅ (sidecar) | ✅ (JSON) | ✅ | medium | two files, sync issues |
| `C. SpreadsheetML 2003 + foreign-namespace extension` | ✅ | ✅ | ✅ if Excel ignores foreign ns | medium | medium |
| `D. SpreadsheetML 2003 + custom-document-properties` | ✅ | ✅ (string-typed only) | ✅ (Excel preserves) | medium | medium |
| `E. OOXML .xlsx + custom XML part` | ✅ | ✅ | ✅ | high (zip + many xml parts) | large |

**Recommendation: option C (with D as a fallback / belt-and-braces).**
Excel-readable, single file, low overhead, the existing spreadsheet_xml
service is already the right substrate, and we get to write our own
schema in a namespace we own.

## 4. Why XML Spreadsheet 2003 wins as the container

1. **Excel reads it natively.** Double-click, drag onto Excel.exe, send
   over email — the formula appears in cell A1 of `Sheet1`. No conversion,
   no plugin.
2. **It is already in the spec direction.** `SCOPE_AND_SPEC §11` chose
   it; Foundation reference corpus has the relevant Excel openspec docs
   curated. Choosing differently would be a spec-level change, not a
   tactical one.
3. **It is plain text.** Diffable in git, grep-able, recoverable from a
   backup that lost its tooling.
4. **Foreign namespaces survive.** The 2003 SpreadsheetML schema is
   permissive: elements / attributes in unknown namespaces at the
   `Workbook` and `Worksheet` levels are tolerated by Excel when reading
   and ignored when rendering. Excel does drop them on save, but our
   round-trip path is **DnaOneCalc → disk → DnaOneCalc**; the **Excel →
   disk → DnaOneCalc** path is a one-way verification flow, not the user
   save loop.
5. **The OOXML alternative (.xlsx) is heavier** (zip + relationships +
   shared strings table + multiple XML parts) for no functional gain at
   our single-cell single-formula scope.
6. **No lock-in.** If we ever outgrow it we can write an `.xlsx`
   exporter in parallel; the internal data shape stays the same.

## 5. The container shape

### 5.1 Envelope
```xml
<?xml version="1.0"?>
<?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:o="urn:schemas-microsoft-com:office:office"
          xmlns:x="urn:schemas-microsoft-com:office:excel"
          xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:dna="urn:dnakode:dnaonecalc:formula:1">

  <!-- Excel-visible content. One worksheet, one cell. -->
  <Worksheet ss:Name="Formula">
    <Table>
      <Row>
        <Cell ss:Formula="=SUM(1,2,3)"
              ss:StyleID="dna-base">
          <Data ss:Type="Number">6</Data>
        </Cell>
      </Row>
    </Table>
  </Worksheet>

  <!-- DnaOneCalc extension lane.
       Foreign namespace; Excel reads, ignores, does not render. -->
  <dna:Formula version="1">
    <dna:Identity
      id="invoice-eu-tax"
      name="invoice-eu-tax"
      created-at="2026-04-22T10:14:22Z"
      modified-at="2026-04-26T14:22:01Z" />
    <dna:Entry mode="Formula">=SUM(1,2,3)</dna:Entry>
    <dna:Context>
      <dna:HostProfile profile-id="Excel365Win" requires-excel-observation="true"/>
      <dna:Locale id="EnUs" date1904="false"/>
      <dna:PublicationContext
        format-profile=""
        number-format-code="€ #,##0.00"
        style-id=""
        font-color=""
        fill-color="">
        <!-- Style hierarchy + cf rules nested as full XML so they
             diff cleanly. -->
        <dna:StyleHierarchy/>
        <dna:CfRules/>
      </dna:PublicationContext>
      <dna:ScenarioPolicy>Deterministic</dna:ScenarioPolicy>
    </dna:Context>
    <dna:UiPreferences
      formula-drill-expanded="false"
      result-drill-expanded="true"
      expanded-editor="false"/>
  </dna:Formula>

  <!-- REPEATABLE, OPTIONAL. Compare-with-Excel evidence for the
       formula above. Zero, one, or many sibling bundles per file,
       in chronological-ascending order by `compared-at`. Each
       carries a `for-formula-state` digest so the loader can tell
       which historical formula state it was about; bundles
       accumulate across edits as history rather than being
       dropped. Native XML elements; see §9. -->
  <dna:CompareBundle
    bundle-id="cb-2026-04-22T1014-Excel365Win"
    compared-at="2026-04-22T10:14:22Z"
    excel-host-id="Excel365Win-16.0.18025"
    for-formula-state="sha256:0000aaaa…"
    value-verdict="match"
    display-verdict="match"
    replay-verdict="equivalent">
    <!-- … historical bundle from before the formula was edited … -->
  </dna:CompareBundle>
  <dna:CompareBundle
    bundle-id="cb-2026-04-26T1430-Excel365Win"
    compared-at="2026-04-26T14:30:11Z"
    excel-host-id="Excel365Win-16.0.18025"
    for-formula-state="sha256:abcd1234…"
    value-verdict="match"
    display-verdict="mismatch"
    replay-verdict="equivalent">
    <dna:VerificationRequest>...</dna:VerificationRequest>
    <dna:VerificationReport>...</dna:VerificationReport>
    <dna:OxFmlSummary>...</dna:OxFmlSummary>
    <dna:ExcelObservationSummary>...</dna:ExcelObservationSummary>
    <dna:ReplayMismatches>
      <dna:Mismatch>...</dna:Mismatch>
    </dna:ReplayMismatches>
    <dna:ReplayExplains>
      <dna:Explain>...</dna:Explain>
    </dna:ReplayExplains>
  </dna:CompareBundle>

  <!-- Optional: the same data also embedded as a Custom Document
       Property whose name is "DnaOneCalc.Formula.v1.json". This
       gives us a belt-and-braces survival path if a future Excel
       version starts stripping foreign root-level elements. -->
  <DocumentProperties xmlns="urn:schemas-microsoft-com:office:office">
    <Author>DnaOneCalc</Author>
  </DocumentProperties>
</Workbook>
```

### 5.2 What lives where
| OneCalc concept | Where in the file | Visibility in Excel |
|---|---|---|
| formula text | `Cell/@ss:Formula` AND `dna:Entry` | shown in cell + formula bar |
| computed value (last evaluation) | `Cell/Data` | shown in cell when Excel doesn't recalc |
| number-format code | `Style/NumberFormat` AND `dna:PublicationContext/@number-format-code` | applied to cell, shown formatted |
| base font / fill colour | `Style/Font` / `Style/Interior` AND `dna:PublicationContext/@*-color` | applied to cell |
| conditional-formatting rules | `x:ConditionalFormatting` AND `dna:CfRules` | applied to cell |
| `Date1904` flag | `x:WorkbookOptions/x:Date1904` AND `dna:Locale/@date1904` | affects date semantics |
| host profile, scenario policy, UI prefs | `dna:*` only | invisible to Excel |
| identity (id, timestamps), entry mode pill | `dna:Identity`, `dna:Entry/@mode` | invisible to Excel |
| compare-with-Excel evidence | `dna:CompareBundle` (optional, see [§9](#9-compare-bundle-as-an-optional-sibling-element)) | invisible to Excel |

### 5.3 The write rule
For **every** field that has an Excel-native representation, write **both**:
1. the native form (so Excel renders it correctly), AND
2. the `dna:` form (so DnaOneCalc reads from a single canonical source on
   the next load).

When the two disagree on read (Excel-edited file), the Excel-native form
wins for fields Excel could plausibly have edited (cell value, number
format, colours, CF rules); the `dna:` form wins for fields Excel cannot
express (host profile, scenario policy, identity, UI prefs, attachment
refs). Read code must explicitly note divergence in a load report.

### 5.4 The read rule
When loading a `.dnafml`:
1. Try to parse the `dna:Formula` extension. If present and the version
   is one we know, take it as the authoritative source.
2. If absent (e.g. file was saved by Excel after a round-trip), fall
   back to reading the Excel-native fields — this is the same code path
   `services/spreadsheet_xml.rs` already exercises, with the missing
   fields filled with sensible defaults. Surface a "loaded from
   Excel-only fields; some context defaults applied" warning.
3. If the version is newer than we know, refuse to silently downgrade.
   Surface a "this file was saved by a newer DnaOneCalc; some fields
   may be ignored or lost" warning before showing the formula.

## 6. Round-trip discipline

### 6.1 OneCalc → disk → OneCalc (the user save loop)
1. All round-trip-listed fields (`§11.3`) must survive verbatim.
2. Browser invariants for every fixture in the test corpus must save
   and reload, comparing the in-memory `Scenario` struct before and
   after.
3. Lossy fields (e.g. ephemeral live-bridge state) must not be written
   in the first place.

### 6.2 OneCalc → Excel (verification flow)
The expectation: a teammate can open the formula in Excel and see the
formula in cell A1. Defined names, references to other cells, and rich
values (lambdas, RTD targets) might error in Excel because no host
context is present — that is acceptable and expected. The file is still
*open-able* and *inspectable*; what fails is *evaluation*.

### 6.3 Excel → disk → OneCalc (recovery flow)
A user might save a `.dnafml` from Excel after editing. We must:
1. Still load the file.
2. Detect that the `dna:` extension is gone or stale (Excel will likely
   strip our foreign-namespace elements on save).
3. Reconstruct the scenario from the Excel-native fields with sensible
   defaults for everything we can't recover (scenario policy, host
   profile, UI prefs).
4. Mark the loaded scenario as "imported from Excel-only file"; first
   save back to disk re-establishes the full extension.

This is the same shape as the existing verification-bundle import path,
which already loads SpreadsheetML 2003 from Excel's emitter for
verification cases.

## 7. Defined-name handling

A formula like `=SUM(FILTER(sales, region = "EU"), IF(tax_applied, 0.21, 0) * base)`
references names (`sales`, `region`, `tax_applied`, `base`) that have no
binding in our single-formula scenario. Two paths:

1. **Carry the names as `<Names>` in the workbook root** with
   placeholder definitions (e.g. `<NamedRange ss:Name="sales" ss:RefersTo="={1;2;3}"/>`),
   so Excel sees a defined name and the formula doesn't `#NAME?` on
   first open. The placeholder values can be sample data carried in the
   `dna:` extension.
2. **Don't bind the names.** The formula errors `#NAME?` in Excel; the
   formula text is preserved verbatim and the user understands the file
   is a captured-formula, not a self-evaluating workbook.

Recommendation: **path 2 by default**, with path 1 as an opt-in for the
small fraction of formulas where the user wants Excel to compute a
sensible value. Path 2 keeps the file simple and the contract clear.
Path 1 belongs in a follow-up bead behind a flag.

## 8. File extension and OS association

`.dnafml` and `.xml` are **both first-class supported extensions**.
Same loader, same saver, byte-identical content. The user picks based
on which OS file association they want — not based on what's inside.

### 8.1 The two peers

| Extension | Excel auto-opens? | OneCalc recognises it? | When to choose it |
|---|---|---|---|
| `.dnafml` | usually no (user picks "Open with…" the first time, then association sticks) | ✅ (registered handler) | "this is a OneCalc formula" — sharing within OneCalc-using teams |
| `.xml` | yes | ✅ (handler reads any SpreadsheetML 2003 XML) | "this opens in Excel by default" — sharing with non-OneCalc users; archiving; scripted pipelines |

The save dialog presents both as peers:
```
Save as:  invoice-eu-tax
Format:   ◉ DnaOneCalc Formula (*.dnafml)
          ○ Excel XML Spreadsheet (*.xml)
```
Whichever the user chooses, the bytes written are identical. The only
difference is the filename suffix.

### 8.2 What to avoid
| Approach | Why not |
|---|---|
| `.dnafml.xml` (compound) | confuses both file managers; users see two dots and assume a tooling artefact |
| `.xls` rename | Excel expects BIFF8 binary at that extension; we'd be lying about content |
| `.xlsx` rename | OOXML is a zip; we'd be lying about content |
| OneCalc-only `.dnafml` (drop `.xml`) | forces "Open with…" friction every time the user hands the file to a non-OneCalc colleague |
| Excel-only `.xml` (drop `.dnafml`) | OS can't tell our XML from any other; opening defaults to Excel even when OneCalc is installed |

### 8.3 Loader policy
The OneCalc reader accepts any extension and detects the format from
content (the `<?mso-application progid="Excel.Sheet"?>` PI plus the
SpreadsheetML 2003 namespaces). It does **not** require a particular
extension. This means:
1. A user who renamed their `.dnafml` to `.xml` and back gets the same
   experience either way.
2. Files emailed without an extension (some webmail systems strip
   them) still load if the bytes are recognisable.
3. We can later add `.dnafml.xml` as a third recognised suffix at zero
   cost if a user ecosystem demands it — but we don't promote it.

### 8.4 Saver policy
The default save extension is **`.dnafml`** (matches the breadcrumb's
`Save as…` action which the user invoked from inside OneCalc). The
save dialog's format dropdown lets them switch to `.xml` for the same
save. After a successful save, the dialog remembers the per-formula
choice for the next "Save" (Ctrl+S without "as…").

### 8.5 OS-association strategy
Tauri installer registers OneCalc as a handler for `.dnafml` (default)
and as a *secondary* handler for `.xml` (Excel keeps the default for
`.xml`, the user can right-click → "Open with → DnaOneCalc"). On
browser host neither extension has a true OS association — the user
downloads a file, then uploads it back via the breadcrumb's `Open…`
dialog, the same way they handle any download/upload pair.

## 9. Compare bundles as repeatable sibling elements

Compare-with-Excel evidence (verification request, verification report,
OxFml summary, Excel observation summary, replay mismatch + explain
records) lives **inside the same `.dnafml`** as one or more
`<dna:CompareBundle>` siblings under `<Workbook>`. It does **not** live
in a separate `.dnacomparebundle` file.

This supersedes the earlier `APP_UX_REALIZATION §5.2` proposal (which
was a leftover from when the formula file itself was JSON) and the
"single, drop-on-edit" model from the previous draft of this plan.

### 9.1 Why in-file
1. A bundle's `scenario_snapshot` is literally an inlined formula —
   the two artefacts always travel together.
2. The XML container already has the `dna:` extension lane Excel
   ignores, so adding one more sibling is free.
3. One file is easier to email, archive, attach to a bug report.
4. The user concept is "the formula and the verification I ran on it"
   — one piece of work, one file.

### 9.2 Why multiple, not one
A user typically runs **Compare with Excel many times** during a
formula's life:
- Tuesday with Excel365Win → match-mismatch-equivalent verdicts
- edit the formula
- Wednesday with Excel365Win again → new verdicts
- Thursday with ExcelMac to check cross-host parity → different
  verdicts again

Each run is real evidence. The previous "single bundle, drop on edit"
model would throw most of that history away. Replacing it with a
repeatable element that **accumulates as history** lets the user (and
X-Ray, and replay) reason about how a formula's behaviour against
Excel evolved over edits and host versions.

The contract:
1. `<dna:CompareBundle>` is a repeatable child of `<Workbook>`.
2. Each bundle records `compared-at`, `excel-host-id` (e.g.
   `Excel365Win-16.0.18025`), and a `for-formula-state` digest of the
   formula text + context that was compared. The digest lets the
   loader / UI distinguish bundles for the *current* formula state
   from bundles for *prior* formula states.
3. Saves always preserve all existing bundles. Edits do not drop
   bundles — they just mean the current-formula-state digest no longer
   matches the old bundles, and the UI flags those bundles as
   "history" rather than "live."
4. Running Compare with **no significant change** since the last
   bundle for the same `(for-formula-state, excel-host-id)` updates
   that bundle's `compared-at` timestamp in place. Bundles are not
   precious audit evidence — we only want a new entry when something
   the user cares about has actually changed (formula state, Excel
   host id, or any of the three verdicts). The same-everything
   re-run shows up as "last verified <new-date>" on the existing
   row instead of cluttering the history.
5. Running Compare with a significant change (different
   `for-formula-state`, different `excel-host-id`, or different
   verdicts) **appends** a new bundle.
6. The user can **delete** any bundle and **replace** any bundle
   from the UI. Bundles are not precious audit evidence — they
   are local notes the user keeps about their formula's behaviour.
   Deleting an old bundle is a normal operation, not a destructive
   one.

### 9.3 Native XML, not CDATA-JSON
Bundle content is encoded as **native XML elements** inside
`<dna:CompareBundle>`. We do not embed JSON inside CDATA.

| Option | Status |
|---|---|
| `A. Native XML elements` | **chosen.** Self-consistent file (the rest of the file is XML; the bundle should be too); diffable in git per-record; readable in a text editor; XSLT-friendly when X-Ray wants to walk the bundle structure later; future-proof. |
| `B. CDATA-wrapped JSON` | rejected. Was the previous draft's recommendation on the grounds of "zero serde rework," but: rework is a one-off cost; format consistency is forever; a `.dnafml` whose bundle is opaque to XML tooling is a bug we'd ship and never undo. |

Implementation cost: the existing `VerificationCaseReport` /
`OxReplayMismatchRecord` / `OxReplayExplainRecord` /
`ExcelObservationSummary` shapes get a parallel XML
serializer / deserializer next to their existing JSON path. A
straightforward `quick-xml::de`-style derive or hand-rolled
writer / reader in `persistence/formula_file.rs`. The internal Rust
types are unchanged; only the (de)serialization surface grows.

The bundle schema sketch:
```xml
<dna:CompareBundle
  bundle-id="cb-2026-04-26T1430-Excel365Win"
  compared-at="2026-04-26T14:30:11Z"
  excel-host-id="Excel365Win-16.0.18025"
  for-formula-state="sha256:abcd1234…"
  value-verdict="match"
  display-verdict="mismatch"
  replay-verdict="equivalent">

  <dna:VerificationRequest>
    <!-- full VerificationBatchRequest, child elements per field -->
  </dna:VerificationRequest>

  <dna:VerificationReport>
    <!-- full VerificationCaseReport, including the three verdicts and
         per-view payloads -->
  </dna:VerificationReport>

  <dna:OxFmlSummary>
    <!-- OxfmlVerificationSummary -->
  </dna:OxFmlSummary>

  <dna:ExcelObservationSummary>
    <!-- ExcelObservationSummary or absent -->
  </dna:ExcelObservationSummary>

  <dna:ReplayMismatches>
    <dna:Mismatch>
      <!-- one per OxReplayMismatchRecord -->
    </dna:Mismatch>
    <!-- ... -->
  </dna:ReplayMismatches>

  <dna:ReplayExplains>
    <dna:Explain>
      <!-- one per OxReplayExplainRecord -->
    </dna:Explain>
    <!-- ... -->
  </dna:ReplayExplains>
</dna:CompareBundle>
```

### 9.4 The `for-formula-state` digest
Each bundle carries a digest of the exact formula state it was
compared against — formula text plus the relevant slice of the context
(host profile, locale, date1904, number format code, base style,
non-derivable CF rules). The digest is what tells the UI whether a
bundle is current or historical, without re-running anything.

The digest input is canonicalised (sorted keys, normalised whitespace)
so trivial reformatting of the on-disk file does not invalidate
historical bundles.

### 9.5 Retention policy
File size grows with each retained bundle. Default cap, designed to
keep recent + cross-host history without unbounded growth:

1. **Always keep** the most recent bundle whose `for-formula-state`
   matches the current formula state (the "live" bundle).
2. **Keep** the most recent bundle for each distinct
   `(for-formula-state, excel-host-id)` pair.
3. **Cap** the total bundle count at 10. When over the cap, drop the
   oldest history-only bundles first; never drop a live bundle to
   satisfy the cap.
4. **Override** per file via a `dna:Formula/@bundle-retention="all"`
   attribute (no pruning); workspace settings exposes a global
   default.

The retention policy runs at save time, not load time, so opening a
file with 50 bundles does not silently destroy 40 of them.

### 9.6 UI implications (forward-pointer)
The compare view's left rail or a new "history" panel should let the
user:
1. See the list of bundles attached to the current formula, sorted by
   recency.
2. Pick one to view its verdicts and mismatch list (the existing
   compare view machinery acts on whichever bundle is selected).
3. Diff two bundles to see how behaviour changed between them.
4. Pin a bundle to prevent it being pruned by the retention policy.
5. **Delete** a bundle (a normal operation; bundles are local
   notes, not precious audit evidence).
6. **Replace** a bundle in place — re-run Compare for the same
   `(formula-state, excel-host-id)` pair and overwrite the existing
   bundle's payload. Useful when the user wants to refresh evidence
   without growing the history list.

Out of scope here, but the file format must support each of those —
which is why bundles carry stable `bundle-id` attributes the UI can
target.

### 9.7 What this replaces
- `APP_UX_REALIZATION §5.2` `.dnacomparebundle` JSON file: gone.
- `dna:Formula/dna:AttachedCompareBundle/@path` (the path-reference
  field in §5.1's mapping): gone. The bundle is in the same file, no
  reference needed.
- The previous draft's "single bundle, drop on edit" model: gone.
  Bundles accumulate as history; edits mark old bundles as historical
  rather than deleting them.
- `services/persistence/compare_bundle.rs` as a separate I/O module:
  unnecessary. The compare-bundle (de)serialiser is a set of helpers
  inside `persistence/formula_file.rs`.

## 10. Implementation seam ladder

### 10.1 Slice 1 — minimum viable persistence
Goal: user clicks **Save as…** in the breadcrumb dropdown → file written
to disk → user clicks **Open…** in the breadcrumb dropdown → file
loaded back → state matches. Save dialog offers both `.dnafml` and
`.xml` extensions per §8.

1. New crate module `persistence/formula_file.rs` exposing:
   ```rust
   pub fn write_formula_file(path: &Path, scenario: &Scenario) -> Result<(), FormulaFileError>;
   pub fn read_formula_file(path: &Path) -> Result<LoadedFormula, FormulaFileError>;
   ```
   where `Scenario` is the in-memory shape and `LoadedFormula` carries
   the scenario plus a `Vec<LoadDiagnostic>` (for warnings like
   "loaded from Excel-only fields", "dna version newer than we know").
   The functions accept any extension; the loader detects format from
   content, the writer writes the same bytes regardless of the path's
   suffix (per §8.3 / §8.4).
2. Promote the existing `verification_bundle.rs::write_excel_2003_xml_workbook`
   into `persistence/spreadsheet_ml.rs` so both verification fixtures
   and user persistence share the emitter.
3. Promote the relevant subset of `services/spreadsheet_xml.rs` parsing
   helpers next to it.
4. Wire `Save as…` and `Open…` actions in the breadcrumb dropdown to
   the new functions; replace the SEAM stubs. Save dialog format
   dropdown carries both extensions as peer options.
5. Browser invariant: round-trip every fixture in the test corpus
   through `write_formula_file` → `read_formula_file` and assert
   `Scenario` equality. Run the matrix once with `.dnafml` paths and
   once with `.xml` paths to pin the extension-agnostic claim.

### 10.2 Slice 2 — Excel-native fidelity
1. Emit native `<Styles>` / `<Style>` elements that match the
   `dna:PublicationContext` style fields.
2. Emit `<x:ConditionalFormatting>` for the admitted CF subset.
3. Emit `<x:WorkbookOptions>/<x:Date1904>` based on `dna:Locale/@date1904`.
4. Manual verification: open the saved file in Excel, confirm the
   formula renders with the correct format / colours / CF.

### 10.3 Slice 3 — Excel-recovery import
1. Implement the fallback read path: when `dna:Formula` is absent or
   unparseable, reconstruct the scenario from native fields with
   sensible defaults.
2. Surface the "imported from Excel-only file" warning in a status-foot
   chip until the user explicitly saves back.

### 10.4 Slice 4 — Compare-bundle merge
See §9 for the full bundle design (native XML, repeatable element,
accumulate-as-history retention model).

1. Define the `dna:CompareBundle` schema as native XML elements (no
   CDATA, no embedded JSON).
2. Add `write_compare_bundle_into(scenario, bundle)` (mutates the
   scenario's bundle list) and corresponding read helpers inside
   `persistence/formula_file.rs`.
3. Wire the compare view's `Save bundle` action to append a new
   `<dna:CompareBundle>` to the current `.dnafml`.
4. Implement the retention policy from §9.5 (default LRU cap, user
   override).
5. Browser invariant: run compare twice (different timestamps), save
   both times; assert two `<dna:CompareBundle>` elements are present
   in DOM order with correct `compared-at` attributes; load and
   confirm both round-trip.

### 10.5 Slice 5 — `<Names>` placeholder mode (deferred)
Opt-in flag in `dna:Formula/@bind-placeholders` that adds workbook-level
`<Names>` for the formula's identifiers, with placeholder values
captured in the `dna:` extension so OneCalc round-trips them.

## 11. Open questions

1. **Foreign-namespace tolerance across Excel versions.** Excel 2003,
   Excel 2007, Excel 365 (Win/Mac/Web), LibreOffice — do they all read
   foreign-namespace elements at the `Workbook` root without complaint?
   Need a manual verification matrix before committing the design.
   Fallback: move `dna:Formula` into `<DocumentProperties>` as a single
   string-typed custom property, which Excel preserves explicitly across
   versions.
2. **CF-rule schema mapping.** The `VerificationConditionalFormattingRule`
   shape (`OxFml::publication`) and the SpreadsheetML 2003 `<x:ConditionalFormatting>`
   schema do not match 1:1. Document the projection rules and which
   subset survives Excel round-trip.
3. **Locale id format.** `dna:Locale/@id="EnUs"` (PascalCase per
   `LocaleProfileId`) vs `xml:lang="en-US"` (BCP-47). Carry both? Or
   define a stable mapping table in the schema.
4. **`.xll` / RTD scenario fields.** Out of scope for v1; the format
   must reserve attribute / element names for them so we don't have to
   bump the version when they land.
5. **Browser-host save target.** Tauri can write `.dnafml` to disk
   directly. The browser host has to use `localStorage` or
   `OPFS` (Origin Private File System) until a Save File Picker shim is
   wired. Document the SEAM (`SEAM-ONECALC-SCENARIO-PERSIST` already
   covers it).
6. **Schema versioning policy.** `dna:Formula/@version="1"` is a major
   version. Define the bump rules: unknown attributes are tolerated,
   unknown elements at Identity / Context / UiPreferences level are
   tolerated, unknown sub-elements within those bump the major version.
7. **OS file-association strategy.** Windows / macOS Tauri installer
   should register OneCalc as the default handler for `.dnafml` and a
   secondary handler for `.xml` (Excel keeps the default for `.xml`,
   the user can right-click → "Open with → DnaOneCalc"). Browser host
   has no association; users download then upload via the breadcrumb's
   `Open…` dialog. Document the installer step in the Tauri build doc.
8. **Bundle retention default — DECIDED.** Cap at 10 bundles. The
   user can delete bundles from the UI; pruning is per §9.5.
9. **Bundle ordering — DECIDED.** Chronological ascending by
   `compared-at` (oldest first, newest last) so a tail-read-only diff
   shows the freshest bundle at the bottom. Confirm and pin in the
   schema.

## 12. What changes vs the existing JSON plan

| `APP_UX_REALIZATION §5.1` JSON field | New XML location |
|---|---|
| `dnascenario_version` | `dna:Formula/@version` |
| `id`, `name`, `created_at`, `modified_at` | `dna:Identity/@*` |
| `formula.entered_text` | `Cell/@ss:Formula` (when starts with `=`) AND `dna:Entry` text |
| `formula.entry_mode` | `dna:Entry/@mode` |
| `context.host_profile` | `dna:HostProfile/@*` |
| `context.locale` | `dna:Locale/@id` |
| `context.date1904` | `x:WorkbookOptions/x:Date1904` AND `dna:Locale/@date1904` |
| `context.publication_context.*` | `Style/...` AND `dna:PublicationContext/@*` |
| `context.publication_context.cf_rules` | `x:ConditionalFormatting` AND `dna:CfRules/dna:Rule` |
| `context.scenario_policy` | `dna:ScenarioPolicy` |
| `ui_preferences.*` | `dna:UiPreferences/@*` |
| `attached_compare_bundle_path` | (removed — bundle is in-file) |

| `APP_UX_REALIZATION §5.2` `.dnacomparebundle` field | New XML location |
|---|---|
| (whole file, formerly one) | `dna:CompareBundle` (repeatable sibling of `dna:Formula`) |
| `dnacomparebundle_version` | (per-bundle implicit; `dna:Formula/@version` covers schema) |
| `created_at` | `dna:CompareBundle/@compared-at` |
| (new) bundle stable id | `dna:CompareBundle/@bundle-id` |
| (new) Excel host the bundle was captured against | `dna:CompareBundle/@excel-host-id` |
| (new) digest of formula state at compare time | `dna:CompareBundle/@for-formula-state` |
| (new) three top-level verdicts (cached on bundle root) | `dna:CompareBundle/@{value,display,replay}-verdict` |
| `scenario_snapshot` | (removed — the same file IS the scenario; the digest tells the loader which historical formula state the bundle was about) |
| `verification_request` | `dna:CompareBundle/dna:VerificationRequest` (native XML elements) |
| `verification_report` | `dna:CompareBundle/dna:VerificationReport` (native XML elements) |
| `oxfml_summary` | `dna:CompareBundle/dna:OxFmlSummary` (native XML elements) |
| `excel_observation_summary` | `dna:CompareBundle/dna:ExcelObservationSummary` (native XML elements) |
| `replay_mismatch_records[]` | `dna:CompareBundle/dna:ReplayMismatches/dna:Mismatch` (one per record) |
| `replay_explain_records[]` | `dna:CompareBundle/dna:ReplayExplains/dna:Explain` (one per record) |

Internal Rust types are unchanged. Existing JSON serialisers stay in
place (the verification subsystem still writes scratch JSON under
`target/onecalc-verification/...`). The persistence module gains a
parallel XML serialiser / deserialiser for the same types.

## 13. Decision

Proceed with **one user-facing file format** — XML Spreadsheet 2003 +
DnaOneCalc-namespace extension. Two extensions accepted as peers:
`.dnafml` (default OneCalc-first) and `.xml` (default Excel-first).
The same file carries the formula and zero-or-more compare-with-Excel
bundles, all encoded as native XML elements. There is no separate
`.dnacomparebundle` file, and bundles are not embedded as JSON inside
CDATA.

The only JSON state OneCalc writes is `workspace.json`, which is the
host-app's own config / state cache stored in the platform's per-user
app-data location (or `localStorage` on browser host) — not a
document, not per-formula, not part of the user's deliverables.

Code lifecycle (full sequence is the seam ladder in §10):

1. Lift the existing emit + parse code into a dedicated `persistence/`
   module.
2. Add the `dna:Formula` extension schema and the round-trip
   serialiser.
3. Wire the breadcrumb's `Save as…` and `Open…` actions to it.
4. Write the `format-round-trip` invariant suite first, before exposing
   the actions in the UI.
5. Add the `dna:CompareBundle` element after the formula round-trip is
   stable; merge the previously-planned `.dnacomparebundle` JSON path
   into the compare-view's `Save bundle` action writing into the
   current `.dnafml`.

The matching internal-architectural term remains `scenario`; the
on-disk extension namespace is `dna:` (short for `dnaonecalc`); the
user-facing extension is `.dnafml`.
