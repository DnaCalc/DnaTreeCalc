import { Code, FileText, Play, Eye, GitCompare, Users, Package, CheckCircle2, Shield } from 'lucide-react';

export function EvidenceBundlePanel() {
  const formula = `=LET(
  values, SEQUENCE(10, 1, 1, 1),
  filtered, FILTER(values, MOD(values, 2) = 0),
  SUM(filtered)
)`;

  return (
    <div className="flex flex-col h-full bg-[#faf7f1]">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <div className="flex items-center gap-2 mb-2">
          <Package className="w-4 h-4 text-[#b84532]" />
          <h2 className="text-sm font-semibold text-[#1f1c17]">Evidence Bundle</h2>
        </div>
        <div className="text-xs text-[#7a7568]">
          Traceable artifacts for this workbench session
        </div>
      </div>

      {/* Source Formula (Compact) */}
      <div className="border-b border-[#1f1c17]/10 bg-white">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 mb-2">
            <Code className="w-3.5 h-3.5 text-[#7a7568]" />
            <h3 className="text-xs font-semibold text-[#1f1c17]">Source Formula</h3>
          </div>
          <div className="font-mono text-[10px] leading-relaxed text-[#1f1c17] whitespace-pre-wrap mb-2">
            {formula}
          </div>
          <div className="flex items-center justify-between text-xs">
            <span className="text-[#7a7568]">Result</span>
            <span className="font-mono font-semibold text-[#b84532]">30</span>
          </div>
        </div>
      </div>

      {/* Reliability Badge */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10">
        <div className="flex items-center gap-2 mb-2">
          <Shield className="w-4 h-4 text-[#1e4d4a]" />
          <h3 className="text-xs font-semibold text-[#1f1c17]">Reliability</h3>
        </div>
        <div className="bg-white border-2 border-[#1e4d4a]/30 rounded-lg p-3">
          <div className="flex items-center gap-2 mb-2">
            <div className="text-2xl font-mono font-semibold text-[#1e4d4a]">98%</div>
            <div className="flex-1">
              <div className="h-2 bg-[#1e4d4a]/10 rounded-full overflow-hidden">
                <div className="h-full bg-[#1e4d4a] rounded-full" style={{ width: '98%' }} />
              </div>
            </div>
          </div>
          <div className="text-xs text-[#7a7568]">
            High confidence • 5 consistent runs
          </div>
        </div>
      </div>

      {/* Evidence Artifacts */}
      <div className="flex-1 overflow-y-auto p-4">
        <h3 className="text-xs font-semibold text-[#1f1c17] mb-3">Artifacts</h3>
        <div className="space-y-2">
          {/* Scenario */}
          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1.5">
              <FileText className="w-3.5 h-3.5 text-[#3e5238]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Scenario</span>
            </div>
            <div className="text-xs text-[#7a7568] mb-2">
              Deterministic evaluation context
            </div>
            <div className="flex items-center justify-between text-[10px]">
              <span className="text-[#7a7568]">ID</span>
              <span className="font-mono text-[#1f1c17]">scn-4f8e2a</span>
            </div>
          </div>

          {/* Run */}
          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1.5">
              <Play className="w-3.5 h-3.5 text-[#1e4d4a]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Latest Run</span>
            </div>
            <div className="text-xs text-[#7a7568] mb-2">
              Evaluation #5 • 1.2ms
            </div>
            <div className="flex items-center justify-between text-[10px]">
              <span className="text-[#7a7568]">ID</span>
              <span className="font-mono text-[#1f1c17]">run-9a2c5f</span>
            </div>
          </div>

          {/* Observation */}
          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1.5">
              <Eye className="w-3.5 h-3.5 text-[#3e5238]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Observation</span>
            </div>
            <div className="text-xs text-[#7a7568] mb-2">
              Captured state envelope
            </div>
            <div className="flex items-center justify-between text-[10px]">
              <span className="text-[#7a7568]">ID</span>
              <span className="font-mono text-[#1f1c17]">obs-7d3e1b</span>
            </div>
          </div>

          {/* Comparison */}
          <div className="bg-white border border-[#b84532]/30 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1.5">
              <GitCompare className="w-3.5 h-3.5 text-[#b84532]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Comparison</span>
            </div>
            <div className="text-xs text-[#7a7568] mb-2">
              Active comparison session
            </div>
            <div className="flex items-center justify-between text-[10px]">
              <span className="text-[#7a7568]">ID</span>
              <span className="font-mono text-[#1f1c17]">cmp-2f9a6d</span>
            </div>
          </div>

          {/* Witness */}
          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1.5">
              <Users className="w-3.5 h-3.5 text-[#c88d2e]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Witness</span>
            </div>
            <div className="text-xs text-[#7a7568] mb-2">
              3 verification points
            </div>
            <div className="flex items-center justify-between text-[10px]">
              <span className="text-[#7a7568]">ID</span>
              <span className="font-mono text-[#1f1c17]">wit-5c1e8a</span>
            </div>
          </div>
        </div>
      </div>

      {/* Bundle Status */}
      <div className="px-4 py-3 border-t border-[#1f1c17]/10 bg-[#f7f3ea]">
        <div className="flex items-center gap-2 text-xs text-[#1e4d4a]">
          <CheckCircle2 className="w-3.5 h-3.5" />
          <span className="font-medium">Bundle complete • Ready for handoff</span>
        </div>
      </div>
    </div>
  );
}
