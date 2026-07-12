param(
    [switch]$ForceReinit
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..")
try {
    if ($ForceReinit -and (Test-Path ".beads")) {
        & br init --prefix dno --force | Out-Null
    }
    elseif (-not (Test-Path ".beads")) {
        & br init --prefix dno | Out-Null
    }
    else {
        $hasIssues = $false
        if (Test-Path ".beads/issues.jsonl") {
            $issueLine = Get-Content ".beads/issues.jsonl" | Where-Object { $_.Trim() -ne "" } | Select-Object -First 1
            $hasIssues = [bool]$issueLine
        }

        if ($hasIssues) {
            throw "seed-beads-from-worksets: .beads already contains issues. Re-run with -ForceReinit only if you want to overwrite the existing graph."
        }

        & br init --prefix dno | Out-Null
    }

    function New-Issue {
        param(
            [Parameter(Mandatory = $true)][string]$Title,
            [Parameter(Mandatory = $true)][string]$Type,
            [Parameter(Mandatory = $true)][string]$Priority,
            [Parameter(Mandatory = $true)][string]$Labels,
            [Parameter(Mandatory = $true)][string]$Description,
            [string]$Parent,
            [string[]]$Deps = @(),
            [string]$Acceptance = ""
        )

        $createArgs = @(
            "create",
            "--silent",
            "--title", $Title,
            "--type", $Type,
            "--priority", $Priority,
            "--labels", $Labels,
            "--description", $Description
        )

        if ($Parent) {
            $createArgs += @("--parent", $Parent)
        }

        $id = (& br @createArgs).Trim()
        if (-not $id) {
            throw "seed-beads-from-worksets: failed to create issue '$Title'"
        }

        if ($Acceptance) {
            & br update $id --acceptance-criteria $Acceptance | Out-Null
        }

        foreach ($dep in $Deps) {
            if ($dep) {
                & br dep add $id $dep | Out-Null
            }
        }

        return $id
    }

    $ids = @{}

    $ids.ws01_runtime = New-Issue `
        -Title "WS-01 Runtime dependency integration and seam boundary" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-01,vertical-slice" `
        -Description "Run scope: integrate real `OxFml`, `OxFunc`, and `OxReplay` dependencies into the workspace and establish the host-side runtime adapter boundary. This epic must land code, not seam-summary docs." `
        -Acceptance "Evidence: the workspace builds against the real upstream dependencies, and the host crate exposes a concrete adapter boundary for later editor and evaluation work."

    $ids.ws01_shell = New-Issue `
        -Title "WS-01 Desktop-first shell bootstrap and platform honesty" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-01,vertical-slice" `
        -Description "Run scope: stand up the desktop-first shell that will host formula entry, results, diagnostics, and visible product-honesty state. Do not close this epic on shell design notes alone." `
        -Acceptance "Evidence: the shell launches, the core regions exist, and host-profile or platform truth is visible from running code."

    $ids.ws01_t1 = New-Issue `
        -Title "Integrate OxFml, OxFunc, and OxReplay as real workspace dependencies" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-01,vertical-slice" `
        -Parent $ids.ws01_runtime `
        -Description "Run scope: update manifests to reference the actual upstream repos or packages, make the host crate compile against them, and remove any local-only assumption that pretends the dependencies are already present." `
        -Acceptance "Evidence: manifests reference the real dependencies, cargo build or check succeeds, and there is no local fake integration path."

    $ids.ws01_t2 = New-Issue `
        -Title "Establish the runtime adapter layer and host-profile substrate in code" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-01,vertical-slice" `
        -Parent $ids.ws01_runtime `
        -Deps @($ids.ws01_t1) `
        -Description "Run scope: implement the host-side adapter layer that later editor and evaluation beads will call, and encode host-profile plus packet-kind truth in code rather than notes." `
        -Acceptance "Evidence: runtime adapter types exist in code, host-profile and packet-kind truth are executable types, and later work can call them directly."

    $ids.ws01_t3 = New-Issue `
        -Title "Replace the proof entrypoint with a desktop shell that exposes formula, result, and diagnostics regions" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-01,vertical-slice" `
        -Parent $ids.ws01_shell `
        -Deps @($ids.ws01_t1) `
        -Description "Run scope: replace the proof-only entrypoint with a real shell path and create visible regions for formula entry, result output, and diagnostics. This is product code, not a screenshot bead." `
        -Acceptance "Evidence: the shell launches, the three core regions exist, and later editor work can land on the running shell."

    $ids.ws01_t4 = New-Issue `
        -Title "Surface host-profile, packet-kind, and platform honesty in the shell" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-01,product-honesty" `
        -Parent $ids.ws01_shell `
        -Deps @($ids.ws01_t2, $ids.ws01_t3) `
        -Description "Run scope: expose the active host-profile, packet-kind, and platform or host limitations directly in the shell from executable truth." `
        -Acceptance "Evidence: the shell visibly reports host-profile and packet-kind state, and blocked hosts or platforms are explicit rather than implied."

    $ids.ws02_editor = New-Issue `
        -Title "WS-02 Formula editor and OxFml diagnostics integration" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-02,vertical-slice" `
        -Description "Run scope: implement real formula editing in the shell and integrate the OxFml edit and diagnostics path as the authoritative editor substrate." `
        -Acceptance "Evidence: editing is real and keyboard-usable, OxFml edit packets are in the loop, and diagnostics are visible in the shell."

    $ids.ws02_t1 = New-Issue `
        -Title "Implement formula buffer, cursor, selection, and keyboard-first editing in the shell" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-02,vertical-slice" `
        -Parent $ids.ws02_editor `
        -Deps @($ids.ws01_t3) `
        -Description "Run scope: build a real editor interaction model in the shell, including buffer state, cursor movement, selection, and ordinary keyboard editing." `
        -Acceptance "Evidence: the editor supports ordinary keyboard editing and exposes buffer, cursor, and selection state in code."

    $ids.ws02_t2 = New-Issue `
        -Title "Wire OxFml FormulaEditRequest and FormulaEditResult through editor state" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-02,vertical-slice" `
        -Parent $ids.ws02_editor `
        -Deps @($ids.ws01_t2, $ids.ws02_t1) `
        -Description "Run scope: translate editor state into OxFml edit requests, consume edit results, and keep the host side aligned with the upstream packet model." `
        -Acceptance "Evidence: the shell editor flows through the real OxFml edit-packet path and no local stand-in packet flow remains."

    $ids.ws02_t3 = New-Issue `
        -Title "Project OxFml diagnostics and spans into visible UI state" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-02,vertical-slice" `
        -Parent $ids.ws02_editor `
        -Deps @($ids.ws02_t2) `
        -Description "Run scope: render OxFml diagnostics, spans, or locations visibly in the shell and keep their provenance tied to the upstream diagnostic output." `
        -Acceptance "Evidence: diagnostics update as the user edits, span or location information is visible, and the displayed state comes from real OxFml output."

    $ids.ws02_t4 = New-Issue `
        -Title "Add editor smoke verification for formula entry and diagnostics" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-02,verification" `
        -Parent $ids.ws02_editor `
        -Deps @($ids.ws02_t3) `
        -Description "Run scope: add the smallest honest verification path that proves formula entry, edit-packet flow, and diagnostics work together in the running shell." `
        -Acceptance "Evidence: a runnable proof or test exists and fails if the real editor-plus-diagnostics integration is broken."

    $ids.ws03_surface = New-Issue `
        -Title "WS-03 OxFunc surface and admitted-function policy integration" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-03,vertical-slice" `
        -Description "Run scope: consume the OxFunc library context in code, derive admitted-function labels, and surface the resulting function policy honestly in the running product." `
        -Acceptance "Evidence: function-surface truth comes from OxFunc data in code, admitted labels are derived rather than hard-coded, and the shell can report the active function policy."

    $ids.ws03_eval = New-Issue `
        -Title "WS-03 First dependency-backed formula evaluation slice" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-03,vertical-slice" `
        -Description "Run scope: deliver the first real formula-entry to evaluation to result slice through OxFml and OxFunc." `
        -Acceptance "Evidence: an admitted formula can be entered and evaluated through the intended dependencies, the shell shows the result honestly, and the whole path has runnable verification."

    $ids.ws03_help = New-Issue `
        -Title "WS-03 Completion and current help integration" `
        -Type "epic" `
        -Priority "P2" `
        -Labels "WS-03,editor-assistance" `
        -Description "Run scope: integrate deterministic completion and the current help or signature surfaces into the real editor flow without overclaiming upstream maturity." `
        -Acceptance "Evidence: completion and current help surfaces are visible in the editor and remain aligned with actual upstream support."

    $ids.ws03_t1 = New-Issue `
        -Title "Consume OxFunc library context and derive admitted function labels in code" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-03,vertical-slice" `
        -Parent $ids.ws03_surface `
        -Deps @($ids.ws01_t1) `
        -Description "Run scope: load the current OxFunc context, apply admitted-function labeling in code, and make the resulting policy available to later editor and evaluation work." `
        -Acceptance "Evidence: the host consumes OxFunc context in code, admitted labels are derived in code, and downstream evaluation beads can consult the resulting surface."

    $ids.ws03_t2 = New-Issue `
        -Title "Surface the active admitted function policy in the running shell" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-03,product-honesty" `
        -Parent $ids.ws03_surface `
        -Deps @($ids.ws03_t1, $ids.ws01_t4) `
        -Description "Run scope: project the active function policy into visible shell state so unsupported or provisional functions are explicit to the user." `
        -Acceptance "Evidence: the shell shows the current admitted function policy and does not imply unsupported functions are available."

    $ids.ws03_t3 = New-Issue `
        -Title "Wire evaluate action through OxFml and OxFunc for admitted formulas" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-03,vertical-slice" `
        -Parent $ids.ws03_eval `
        -Deps @($ids.ws02_t2, $ids.ws03_t1) `
        -Description "Run scope: connect the shell's evaluate action to the real OxFml and OxFunc-backed runtime path and ensure results are not synthesized locally." `
        -Acceptance "Evidence: evaluate action calls the upstream-backed runtime and the shell can execute at least one admitted formula honestly."

    $ids.ws03_t4 = New-Issue `
        -Title "Render evaluated result and effective-display status in the shell" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-03,vertical-slice" `
        -Parent $ids.ws03_eval `
        -Deps @($ids.ws03_t3) `
        -Description "Run scope: show the returned result and current effective-display truth in the running shell rather than in proof-only logs." `
        -Acceptance "Evidence: the shell visibly renders the result from the real evaluation path and updates when real runs occur."

    $ids.ws03_t5 = New-Issue `
        -Title "Add runnable verification for formula entry, evaluation, and result rendering" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-03,verification" `
        -Parent $ids.ws03_eval `
        -Deps @($ids.ws02_t4, $ids.ws03_t4) `
        -Description "Run scope: add the smallest useful proof that the first vertical slice really works from formula entry through result rendering." `
        -Acceptance "Evidence: a runnable proof or test exists, it exercises the real shell path, and it fails on real integration regressions."

    $ids.ws03_t6 = New-Issue `
        -Title "Integrate deterministic completion proposals into the real editor flow" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-03,editor-assistance" `
        -Parent $ids.ws03_help `
        -Deps @($ids.ws02_t2, $ids.ws03_t1) `
        -Description "Run scope: request deterministic completion from the current upstream path and project proposals into the real editor flow." `
        -Acceptance "Evidence: completion proposals come from the real upstream-backed path and are visible in the editor."

    $ids.ws03_t7 = New-Issue `
        -Title "Integrate current function and signature help surfaces into the editor" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-03,editor-assistance" `
        -Parent $ids.ws03_help `
        -Deps @($ids.ws03_t6, $ids.ws03_t2) `
        -Description "Run scope: show the currently available help or signature data in the editor while keeping missing upstream detail visibly absent or provisional." `
        -Acceptance "Evidence: current help or signature data is visible in the editor and the UI does not claim richer help than upstream truth provides."

    $ids.ws04_h1 = New-Issue `
        -Title "WS-04 Driven single-formula host and retained runs" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-04,H1" `
        -Description "Run scope: implement the H1 driven host model, retain real scenarios and runs, and reopen retained runs without drifting toward worksheet semantics." `
        -Acceptance "Evidence: the H1 path exists in code, retained scenario and run handling are real, and retained runs reopen successfully."

    $ids.ws04_compare = New-Issue `
        -Title "WS-04 Driven-run comparison and H1 verification" `
        -Type "epic" `
        -Priority "P2" `
        -Labels "WS-04,H1" `
        -Description "Run scope: compare retained driven runs and keep the H1 path alive with a promoted smoke family." `
        -Acceptance "Evidence: retained driven runs can be compared and the H1 path has a promoted smoke-family proof."

    $ids.ws04_t1 = New-Issue `
        -Title "Implement the H1 driven host model and recalc context" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-04,H1" `
        -Parent $ids.ws04_h1 `
        -Deps @($ids.ws03_t5) `
        -Description "Run scope: extend the first vertical slice into the H1 driven model with explicit recalc context while staying inside single-formula scope." `
        -Acceptance "Evidence: the H1 path exists in code, recalc context is explicit, and the implementation stays within admitted single-formula scope."

    $ids.ws04_t2 = New-Issue `
        -Title "Persist Scenario and ScenarioRun identities for real runs and reopen them" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-04,retained" `
        -Parent $ids.ws04_h1 `
        -Deps @($ids.ws04_t1) `
        -Description "Run scope: persist scenario and scenario-run identity fields for real H1 runs and reopen them through the product path." `
        -Acceptance "Evidence: real runs produce retained scenario and scenario-run records and those runs reopen through the product path."

    $ids.ws04_t3 = New-Issue `
        -Title "Add retained version-to-version comparison for driven runs" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-04,H1" `
        -Parent $ids.ws04_compare `
        -Deps @($ids.ws04_t2) `
        -Description "Run scope: compare two retained driven runs for the same scenario and prepare the path later replay or twin-compare work will build on." `
        -Acceptance "Evidence: two retained driven runs can be compared through real code and the output is usable by later compare surfaces."

    $ids.ws04_t4 = New-Issue `
        -Title "Add a promoted H1 smoke family that proves the driven path stays runnable" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-04,verification" `
        -Parent $ids.ws04_compare `
        -Deps @($ids.ws04_t3, $ids.ws03_t7) `
        -Description "Run scope: promote a small but meaningful H1 smoke family and use it to verify the driven path." `
        -Acceptance "Evidence: a promoted H1 smoke family exists and catches regressions in retained or driven behavior."

    $ids.ws05_artifacts = New-Issue `
        -Title "WS-05 Artifact spine and capability-snapshot implementation" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-05,retained" `
        -Description "Run scope: implement artifact identity, shared envelopes, lineage, and immutable capability snapshots in code." `
        -Acceptance "Evidence: the core artifact backbone exists in code and capability snapshots are emitted from real product state."

    $ids.ws05_persist = New-Issue `
        -Title "WS-05 Document persistence and ScenarioCapsule transport" `
        -Type "epic" `
        -Priority "P1" `
        -Labels "WS-05,persistence" `
        -Description "Run scope: implement isolated document round-trip and ScenarioCapsule export plus intake on top of the retained-artifact spine." `
        -Acceptance "Evidence: one document round-trips through the declared format, required invariants survive, and ScenarioCapsule export plus intake work on real data."

    $ids.ws05_t1 = New-Issue `
        -Title "Implement core artifact envelope, lineage, and stable references in code" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-05,retained" `
        -Parent $ids.ws05_artifacts `
        -Deps @($ids.ws04_t2) `
        -Description "Run scope: implement shared artifact identity and envelope types in code and capture lineage or reference relationships explicitly." `
        -Acceptance "Evidence: artifact identity and envelope types exist in code and later persistence or replay work can consume them directly."

    $ids.ws05_t2 = New-Issue `
        -Title "Emit immutable capability snapshots tied to retained runs and sessions" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-05,capability" `
        -Parent $ids.ws05_artifacts `
        -Deps @($ids.ws05_t1) `
        -Description "Run scope: generate immutable capability snapshots from the actual host environment and tie them to real runs or sessions." `
        -Acceptance "Evidence: retained runs can reference immutable capability snapshots emitted from executable truth."

    $ids.ws05_t3 = New-Issue `
        -Title "Implement SpreadsheetML round-trip for isolated OneCalc documents" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-05,persistence" `
        -Parent $ids.ws05_persist `
        -Deps @($ids.ws05_t1) `
        -Description "Run scope: save and reopen one isolated OneCalc document through the declared persisted document format without widening into workbook semantics." `
        -Acceptance "Evidence: a document can be saved and reopened through the declared format and remains within isolated OneCalc scope."

    $ids.ws05_t4 = New-Issue `
        -Title "Preserve retained identity and formatting invariants across document round-trip" `
        -Type "task" `
        -Priority "P1" `
        -Labels "WS-05,persistence" `
        -Parent $ids.ws05_persist `
        -Deps @($ids.ws05_t3) `
        -Description "Run scope: verify that retained identity and currently admitted formatting invariants survive save and reopen." `
        -Acceptance "Evidence: required identity and formatting invariants survive round-trip and the proof fails on real regressions."

    $ids.ws05_t5 = New-Issue `
        -Title "Implement ScenarioCapsule export and intake with lineage and capability refs intact" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-05,transport" `
        -Parent $ids.ws05_persist `
        -Deps @($ids.ws05_t2, $ids.ws05_t4) `
        -Description "Run scope: export one scenario with selected retained artifacts as a ScenarioCapsule, re-import it, and preserve lineage plus capability refs." `
        -Acceptance "Evidence: ScenarioCapsule export and intake work on real scenario data and do not lose lineage or capability truth."

    $ids.ws06_replay = New-Issue `
        -Title "WS-06 Replay capture and X-Ray surfaces" `
        -Type "epic" `
        -Priority "P2" `
        -Labels "WS-06,replay" `
        -Description "Run scope: emit replay outputs from retained runs, open replay from the product, and add X-Ray or diff surfaces over retained artifacts." `
        -Acceptance "Evidence: retained runs emit replay outputs, replay opens from the product, and X-Ray or diff surfaces are reachable on real retained data."

    $ids.ws06_handoff = New-Issue `
        -Title "WS-06 Witness, explain, and handoff generation" `
        -Type "epic" `
        -Priority "P3" `
        -Labels "WS-06,handoff" `
        -Description "Run scope: build witness, explain, and handoff generation on top of replay or diff state while sourcing gates from real capability truth." `
        -Acceptance "Evidence: witness or explain output and handoff packets are generated from real retained state and remain honestly gated."

    $ids.ws06_t1 = New-Issue `
        -Title "Emit replay capture outputs from retained runs and open them from the host" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-06,replay" `
        -Parent $ids.ws06_replay `
        -Deps @($ids.ws04_t2, $ids.ws05_t2) `
        -Description "Run scope: generate replay outputs from real retained runs and make the product open them with the current replay floor labeled honestly." `
        -Acceptance "Evidence: retained runs produce replay outputs, the product can open them, and the surfaced replay floor is explicit."

    $ids.ws06_t2 = New-Issue `
        -Title "Add X-Ray and diff surfaces over retained run artifacts" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-06,replay" `
        -Parent $ids.ws06_replay `
        -Deps @($ids.ws06_t1) `
        -Description "Run scope: expose retained artifact internals through X-Ray views and make diff surfaces reachable from the product." `
        -Acceptance "Evidence: X-Ray opens on retained run artifacts and diff surfaces are reachable on real retained data."

    $ids.ws06_t3 = New-Issue `
        -Title "Implement witness and explain generation over retained replay or diff state" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-06,handoff" `
        -Parent $ids.ws06_handoff `
        -Deps @($ids.ws06_t2) `
        -Description "Run scope: generate witness and explain output from retained replay or diff state and keep blocked dimensions visible rather than invented." `
        -Acceptance "Evidence: witness or explain output is generated from real retained state and blocked dimensions remain explicit."

    $ids.ws06_t4 = New-Issue `
        -Title "Implement handoff packet generation gated by capability truth" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-06,handoff" `
        -Parent $ids.ws06_handoff `
        -Deps @($ids.ws06_t3, $ids.ws05_t2) `
        -Description "Run scope: generate handoff packets from retained evidence and gate them with immutable capability truth rather than local prose." `
        -Acceptance "Evidence: handoff packets are generated from real retained evidence and capability snapshots control what they may claim."

    $ids.ws07_format = New-Issue `
        -Title "WS-07 Formatting and effective-display implementation" `
        -Type "epic" `
        -Priority "P2" `
        -Labels "WS-07,formatting" `
        -Description "Run scope: keep returned presentation hints and host style state separate and render effective display for the admitted formatting subset." `
        -Acceptance "Evidence: effective display is rendered from real code and the formatting-plane split is explicit in product state."

    $ids.ws07_cf = New-Issue `
        -Title "WS-07 Conditional-formatting subset and compare labeling" `
        -Type "epic" `
        -Priority "P3" `
        -Labels "WS-07,conditional-formatting" `
        -Description "Run scope: implement the admitted isolated conditional-formatting subset and keep it distinct from ordinary formatting in X-Ray and compare-adjacent surfaces." `
        -Acceptance "Evidence: the admitted CF subset exists in code and the product distinguishes formatting from conditional formatting honestly."

    $ids.ws07_t1 = New-Issue `
        -Title "Separate returned presentation hints from host style state in code" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-07,formatting" `
        -Parent $ids.ws07_format `
        -Deps @($ids.ws03_t4, $ids.ws05_t1) `
        -Description "Run scope: represent returned presentation hints separately from host style state and keep that distinction explicit in code and UI state." `
        -Acceptance "Evidence: the two planes are separate in code and later rendering can consume them without ambiguity."

    $ids.ws07_t2 = New-Issue `
        -Title "Render effective display for the admitted formatting subset" `
        -Type "task" `
        -Priority "P2" `
        -Labels "WS-07,formatting" `
        -Parent $ids.ws07_format `
        -Deps @($ids.ws07_t1) `
        -Description "Run scope: render effective display from the implemented formatting plane and expose the current display state visibly in the product." `
        -Acceptance "Evidence: effective display is visible in the product and derived from the implemented formatting plane."

    $ids.ws07_t3 = New-Issue `
        -Title "Implement the admitted isolated conditional-formatting carrier subset" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-07,conditional-formatting" `
        -Parent $ids.ws07_cf `
        -Deps @($ids.ws07_t1) `
        -Description "Run scope: add only the admitted isolated-instance conditional-formatting subset and keep unsupported CF behavior explicitly blocked." `
        -Acceptance "Evidence: the admitted CF subset exists in code and unsupported CF scope remains visibly unavailable."

    $ids.ws07_t4 = New-Issue `
        -Title "Surface formatting versus conditional-formatting truth distinctly in X-Ray and compare views" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-07,conditional-formatting" `
        -Parent $ids.ws07_cf `
        -Deps @($ids.ws07_t2, $ids.ws07_t3, $ids.ws06_t2) `
        -Description "Run scope: project the distinct formatting and CF planes into X-Ray and compare-adjacent surfaces so omitted dimensions remain explicit." `
        -Acceptance "Evidence: the UI distinguishes formatting from CF truth and unsupported dimensions remain visibly unavailable."

    $ids.ws08_obs = New-Issue `
        -Title "WS-08 Windows observation capture and twin compare" `
        -Type "epic" `
        -Priority "P3" `
        -Labels "WS-08,windows" `
        -Description "Run scope: integrate Windows-only Excel observation through OxXlPlay and build the twin-compare path with provenance, lossiness, and reliability made explicit." `
        -Acceptance "Evidence: Windows observation capture works, Observation and Comparison artifacts are produced from real runs, and the product can open the twin-compare path."

    $ids.ws08_t1 = New-Issue `
        -Title "Integrate Windows OxXlPlay capture-run and persist Observation artifacts with provenance and lossiness" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-08,windows" `
        -Parent $ids.ws08_obs `
        -Deps @($ids.ws05_t2) `
        -Description "Run scope: call the Windows capture path, retain Observation artifacts from real captures, and keep platform gating plus lossiness explicit." `
        -Acceptance "Evidence: the Windows capture path is integrated and Observation artifacts carry provenance plus lossiness classification."

    $ids.ws08_t2 = New-Issue `
        -Title "Build Comparison artifacts and twin-compare views over retained run versus Observation" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-08,windows" `
        -Parent $ids.ws08_obs `
        -Deps @($ids.ws08_t1, $ids.ws07_t4) `
        -Description "Run scope: compare a retained OneCalc run against a Windows Observation and present the result in the product with honest reliability labeling." `
        -Acceptance "Evidence: Comparison artifacts are generated in code and the twin-compare view opens on real data."

    $ids.ws08_t3 = New-Issue `
        -Title "Emit widening-request output when comparison envelope blocks intended validation" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-08,windows" `
        -Parent $ids.ws08_obs `
        -Deps @($ids.ws08_t2, $ids.ws06_t4) `
        -Description "Run scope: detect when the current compare envelope is too narrow and produce widening-pressure output from the real compare path." `
        -Acceptance "Evidence: widening-request output is generated from real compare state and blocked validation dimensions are explicit."

    $ids.ws09_ext = New-Issue `
        -Title "WS-09 Extension ABI and provider-loading rollout" `
        -Type "epic" `
        -Priority "P3" `
        -Labels "WS-09,extensions" `
        -Description "Run scope: create the concrete extension ABI and provider-loading child beads once the core runtime, shell, and capability substrate are stable enough to host them honestly." `
        -Acceptance "Evidence: explicit extension child beads exist with dependencies and proofs wired, and the path no longer depends on chat memory."

    $ids.ws09_rtd = New-Issue `
        -Title "WS-09 RTD subset and platform-gating rollout" `
        -Type "epic" `
        -Priority "P3" `
        -Labels "WS-09,extensions" `
        -Description "Run scope: create the concrete RTD child beads once the extension path and platform truth are explicit." `
        -Acceptance "Evidence: explicit RTD child beads exist with platform gates encoded."

    $ids.ws09_t1 = New-Issue `
        -Title "Roll out concrete extension ABI and provider-loading child beads" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-09,extensions" `
        -Parent $ids.ws09_ext `
        -Deps @($ids.ws03_t7, $ids.ws05_t2) `
        -Description "Run scope: inspect the implemented runtime and capability substrate and create explicit child beads for ABI definition, provider loading, registration, and invocation." `
        -Acceptance "Evidence: concrete extension child beads exist and are implementation-facing rather than document placeholders."

    $ids.ws09_t2 = New-Issue `
        -Title "Roll out concrete RTD child beads from the admitted extension subset" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-09,extensions" `
        -Parent $ids.ws09_rtd `
        -Deps @($ids.ws09_t1) `
        -Description "Run scope: derive the RTD child beads from the concrete extension path, keep the implementation within the admitted RTD subset, and encode platform gates explicitly." `
        -Acceptance "Evidence: RTD child beads exist explicitly and remain tied to the admitted extension and platform subset."

    $ids.ws10_workspace = New-Issue `
        -Title "WS-10 Workspace management and capability-center implementation" `
        -Type "epic" `
        -Priority "P3" `
        -Labels "WS-10,workspace" `
        -Description "Run scope: implement multi-file workspace management for isolated OneCalc documents and surface immutable capability truth through a real capability center." `
        -Acceptance "Evidence: multiple isolated documents can be managed together and the capability center reads real immutable snapshots."

    $ids.ws10_library = New-Issue `
        -Title "WS-10 Scenario library, acceptance, and upstream-pressure rollout" `
        -Type "epic" `
        -Priority "P3" `
        -Labels "WS-10,acceptance" `
        -Description "Run scope: create the concrete scenario-library, acceptance-matrix, and upstream-pressure child beads once the live product surfaces they depend on are in place." `
        -Acceptance "Evidence: explicit child beads exist for the later scenario-library and acceptance path, all tied to real product surfaces."

    $ids.ws10_t1 = New-Issue `
        -Title "Implement multi-file workspace management for isolated OneCalc documents" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-10,workspace" `
        -Parent $ids.ws10_workspace `
        -Deps @($ids.ws05_t5) `
        -Description "Run scope: manage multiple isolated documents together without introducing cross-instance recalc or workbook semantics." `
        -Acceptance "Evidence: multiple isolated documents can be managed in one workspace and their retained truth stays separate."

    $ids.ws10_t2 = New-Issue `
        -Title "Build the capability center and snapshot-diff UI over immutable capability snapshots" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-10,workspace" `
        -Parent $ids.ws10_workspace `
        -Deps @($ids.ws05_t2, $ids.ws10_t1) `
        -Description "Run scope: display the active immutable capability snapshot in the product and provide a diff flow sourced from the same truth retained runs use." `
        -Acceptance "Evidence: the capability center reads real snapshots and supports snapshot diff without maintaining a second capability-truth model."

    $ids.ws10_t3 = New-Issue `
        -Title "Roll out concrete scenario-library and promotion child beads from live replay, compare, persistence, and workspace pressure" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-10,acceptance" `
        -Parent $ids.ws10_library `
        -Deps @($ids.ws08_t3, $ids.ws10_t2) `
        -Description "Run scope: use the now-live replay, compare, persistence, and workspace surfaces to create explicit child beads for scenario promotion, library UX, and acceptance evidence." `
        -Acceptance "Evidence: concrete scenario-library child beads exist and depend on live product features rather than planning prose."

    $ids.ws10_t4 = New-Issue `
        -Title "Roll out concrete acceptance-matrix and upstream-pressure child beads from widened evidence" `
        -Type "task" `
        -Priority "P3" `
        -Labels "WS-10,acceptance" `
        -Parent $ids.ws10_library `
        -Deps @($ids.ws10_t3) `
        -Description "Run scope: create the detailed acceptance and upstream-pressure child beads once promoted-scenario and widened-evidence paths exist." `
        -Acceptance "Evidence: concrete acceptance and upstream-pressure child beads exist and identify the evidence they will consume."

    Write-Host "seed-beads-from-worksets: created $($ids.Count) issues"
}
finally {
    Pop-Location
}
