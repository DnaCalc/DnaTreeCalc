import { useState } from 'react';
import { ChevronDown, ChevronRight, GitCompare, Package, AlertTriangle, CheckCircle2, Target, FileCheck } from 'lucide-react';

interface WorkbenchModeProps {
  onOpenDrawer: (drawer: 'envelope' | 'evidence') => void;
}

export function WorkbenchMode({ onOpenDrawer }: WorkbenchModeProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['outcome', 'lineage']));

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expandedSections);
    if (newExpanded.has(section)) {
      newExpanded.delete(section);
    } else {
      newExpanded.add(section);
    }
    setExpandedSections(newExpanded);
  };

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-7xl mx-auto">
        <div className="grid grid-cols-3 gap-6">
          {/* Left Column: Comparison Outcome - Dominant */}
          <div className="col-span-2 space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-base font-semibold text-[#1f1c17]">Comparison Outcome</h2>
              <button
                onClick={() => onOpenDrawer('envelope')}
                className="text-xs text-[#b84532] hover:text-[#d15745] font-medium"
              >
                View full envelope →
              </button>
            </div>

            {/* Comparison Header - Comparison-level */}
            <div className="bg-gradient-to-br from-[#1e4d4a]/5 via-[#1e4d4a]/10 to-[#3e5238]/5 border-2 border-[#1e4d4a]/20 rounded-xl p-6">
              <div className="flex items-start justify-between mb-5">
                <div>
                  <h3 className="text-lg font-semibold text-[#1f1c17] mb-1">Twin Oracle Comparison</h3>
                  <p className="text-sm text-[#7a7568]">DNA OneCalc vs Excel • Run #247</p>
                </div>
                <div className="flex items-center gap-2.5 px-4 py-2 bg-[#1e4d4a] text-white rounded-lg shadow-sm">
                  <CheckCircle2 className="w-5 h-5" />
                  <span className="font-semibold">Match</span>
                </div>
              </div>
              
              <div className="grid grid-cols-4 gap-4 text-sm">
                <div className="bg-white/60 rounded-lg p-3">
                  <div className="text-[#7a7568] mb-1">Reliability</div>
                  <div className="text-[#1e4d4a] font-semibold">High</div>
                </div>
                <div className="bg-white/60 rounded-lg p-3">
                  <div className="text-[#7a7568] mb-1">Envelope</div>
                  <div className="text-[#1f1c17] font-semibold">Full</div>
                </div>
                <div className="bg-white/60 rounded-lg p-3">
                  <div className="text-[#7a7568] mb-1">Mismatches</div>
                  <div className="text-[#1f1c17] font-semibold">0</div>
                </div>
                <div className="bg-white/60 rounded-lg p-3">
                  <div className="text-[#7a7568] mb-1">Blocked Dims</div>
                  <div className="text-[#7a7568] font-semibold">0</div>
                </div>
              </div>
            </div>

            {/* Replay Lineage - Run-level */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
              <button
                onClick={() => toggleSection('lineage')}
                className="w-full flex items-center justify-between px-4 py-3 hover:bg-[#f7f3ea] transition-colors text-left"
              >
                <div className="flex items-center gap-2">
                  <GitCompare className="w-5 h-5 text-[#3e5238]" />
                  <span className="text-sm font-semibold text-[#1f1c17]">Replay Lineage</span>
                </div>
                {expandedSections.has('lineage') ? (
                  <ChevronDown className="w-4 h-4 text-[#7a7568]" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                )}
              </button>
              {expandedSections.has('lineage') && (
                <div className="px-4 py-3 border-t border-[#1f1c17]/10">
                  <div className="space-y-1">
                    <div className="flex items-center gap-3 p-3 bg-[#f7f3ea] rounded-lg">
                      <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                      <div className="flex-1">
                        <div className="text-sm font-medium text-[#1f1c17]">Scenario: LET Formula Analysis</div>
                        <div className="text-xs text-[#7a7568] font-mono">scn_20260403_247 • Created 2026-04-03 14:30</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-3 p-3 pl-9 bg-[#f7f3ea] rounded-lg">
                      <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                      <div className="flex-1">
                        <div className="text-sm font-medium text-[#1f1c17]">DNA Run #247</div>
                        <div className="text-xs text-[#7a7568] font-mono">run_247_001 • OxFml v0.12.4 • 1.2ms</div>
                      </div>
                      <div className="px-2 py-1 bg-[#1e4d4a] text-white text-xs font-mono rounded">30</div>
                    </div>
                    <div className="flex items-center gap-3 p-3 pl-9 bg-[#f7f3ea] rounded-lg">
                      <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                      <div className="flex-1">
                        <div className="text-sm font-medium text-[#1f1c17]">Excel Observation</div>
                        <div className="text-xs text-[#7a7568] font-mono">obs_247_xl365 • Excel 365 • Windows 11</div>
                      </div>
                      <div className="px-2 py-1 bg-[#b84532] text-white text-xs font-mono rounded">30</div>
                    </div>
                    <div className="flex items-center gap-3 p-3 pl-14 bg-[#1e4d4a]/5 border border-[#1e4d4a]/20 rounded-lg">
                      <Target className="w-4 h-4 text-[#1e4d4a]" />
                      <div className="flex-1">
                        <div className="text-sm font-medium text-[#1f1c17]">Comparison Generated</div>
                        <div className="text-xs text-[#7a7568] font-mono">cmp_247_001 • Full envelope • High reliability</div>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Observation Envelope - Comparison-level */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
              <button
                onClick={() => toggleSection('envelope')}
                className="w-full flex items-center justify-between px-4 py-3 hover:bg-[#f7f3ea] transition-colors text-left"
              >
                <div className="flex items-center gap-2">
                  <FileCheck className="w-5 h-5 text-[#b84532]" />
                  <span className="text-sm font-semibold text-[#1f1c17]">Observation Envelope</span>
                </div>
                {expandedSections.has('envelope') ? (
                  <ChevronDown className="w-4 h-4 text-[#7a7568]" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                )}
              </button>
              {expandedSections.has('envelope') && (
                <div className="px-4 py-3 border-t border-[#1f1c17]/10">
                  <div className="space-y-3 text-sm">
                    <div className="grid grid-cols-2 gap-3">
                      <div className="p-3 bg-[#f7f3ea] rounded-lg">
                        <div className="text-xs text-[#7a7568] mb-1">Coverage</div>
                        <div className="font-semibold text-[#1e4d4a]">Full</div>
                      </div>
                      <div className="p-3 bg-[#f7f3ea] rounded-lg">
                        <div className="text-xs text-[#7a7568] mb-1">Dimensions</div>
                        <div className="font-mono font-semibold text-[#1f1c17]">7</div>
                      </div>
                    </div>
                    <div className="p-3 bg-[#f7f3ea] rounded-lg">
                      <div className="text-xs text-[#7a7568] mb-2">Observed Dimensions</div>
                      <div className="space-y-1 text-xs font-mono">
                        <div className="flex items-center gap-2">
                          <CheckCircle2 className="w-3 h-3 text-[#1e4d4a]" />
                          <span className="text-[#1f1c17]">result_value</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <CheckCircle2 className="w-3 h-3 text-[#1e4d4a]" />
                          <span className="text-[#1f1c17]">result_type</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <CheckCircle2 className="w-3 h-3 text-[#1e4d4a]" />
                          <span className="text-[#1f1c17]">result_shape</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <CheckCircle2 className="w-3 h-3 text-[#1e4d4a]" />
                          <span className="text-[#1f1c17]">display_text</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <CheckCircle2 className="w-3 h-3 text-[#1e4d4a]" />
                          <span className="text-[#1f1c17]">formatting</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Blocked Dimensions - Comparison-level */}
            <div className="bg-[#c88d2e]/5 border border-[#c88d2e]/20 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <AlertTriangle className="w-5 h-5 text-[#c88d2e] flex-shrink-0 mt-0.5" />
                <div className="flex-1">
                  <div className="text-sm font-medium text-[#1f1c17] mb-1">No Blocked Dimensions</div>
                  <div className="text-sm text-[#7a7568]">
                    All observable dimensions were successfully compared. No capability limitations or platform gates prevented comparison.
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Right Column: Evidence & Handoff */}
          <div className="space-y-6">
            {/* Reliability */}
            <div className="bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10 border-2 border-[#1e4d4a]/20 rounded-xl p-4">
              <h3 className="text-sm font-semibold text-[#1f1c17] mb-4">Reliability</h3>
              <div className="space-y-3">
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-xs text-[#7a7568]">Match Quality</span>
                    <span className="text-sm font-semibold text-[#1e4d4a]">High</span>
                  </div>
                  <div className="h-2 bg-[#1f1c17]/10 rounded-full overflow-hidden">
                    <div className="h-full bg-[#1e4d4a] w-full"></div>
                  </div>
                </div>
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-xs text-[#7a7568]">Coverage</span>
                    <span className="text-sm font-semibold text-[#1f1c17]">100%</span>
                  </div>
                  <div className="h-2 bg-[#1f1c17]/10 rounded-full overflow-hidden">
                    <div className="h-full bg-[#3e5238] w-full"></div>
                  </div>
                </div>
                <div className="pt-3 border-t border-[#1f1c17]/10 text-xs">
                  <div className="flex items-center justify-between py-1">
                    <span className="text-[#7a7568]">Confidence</span>
                    <span className="font-semibold text-[#1e4d4a]">Proven</span>
                  </div>
                  <div className="flex items-center justify-between py-1">
                    <span className="text-[#7a7568]">Envelope</span>
                    <span className="font-semibold text-[#1f1c17]">Full</span>
                  </div>
                </div>
              </div>
            </div>

            {/* Evidence Bundle - Comparison-level */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-2">
                  <Package className="w-4 h-4 text-[#c88d2e]" />
                  <h3 className="text-sm font-semibold text-[#1f1c17]">Evidence Bundle</h3>
                </div>
                <button
                  onClick={() => onOpenDrawer('evidence')}
                  className="text-xs text-[#c88d2e] hover:text-[#dda947] font-medium"
                >
                  Details →
                </button>
              </div>
              <div className="space-y-2 text-xs font-mono">
                <div className="p-2 bg-[#f7f3ea] rounded">
                  <div className="text-[#7a7568] text-[10px] mb-1">Scenario ID</div>
                  <div className="text-[#1f1c17]">scn_247</div>
                </div>
                <div className="p-2 bg-[#f7f3ea] rounded">
                  <div className="text-[#7a7568] text-[10px] mb-1">Run ID</div>
                  <div className="text-[#1f1c17]">run_247_001</div>
                </div>
                <div className="p-2 bg-[#f7f3ea] rounded">
                  <div className="text-[#7a7568] text-[10px] mb-1">Comparison ID</div>
                  <div className="text-[#1f1c17]">cmp_247_xl365</div>
                </div>
              </div>
            </div>

            {/* Handoff Actions - Comparison-level */}
            <div className="bg-gradient-to-br from-[#b84532]/5 to-[#b84532]/10 border border-[#b84532]/20 rounded-lg p-4">
              <h3 className="text-sm font-semibold text-[#1f1c17] mb-3">Handoff Actions</h3>
              <div className="space-y-2">
                <button className="w-full px-3 py-2.5 bg-[#1e4d4a] text-white rounded-lg hover:bg-[#2d6864] transition-colors font-medium text-sm shadow-sm">
                  Retain as Evidence
                </button>
                <button className="w-full px-3 py-2.5 border-2 border-[#1e4d4a]/30 text-[#1e4d4a] rounded-lg hover:bg-[#1e4d4a]/5 transition-colors font-medium text-sm">
                  Export Handoff Packet
                </button>
                <button className="w-full px-3 py-2.5 border border-[#1f1c17]/15 text-[#1f1c17] rounded-lg hover:bg-[#f7f3ea] transition-colors text-sm">
                  Mark for Review
                </button>
              </div>
            </div>

            {/* Widening Status - Comparison-level */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
              <h3 className="text-sm font-semibold text-[#1f1c17] mb-3">Widening Status</h3>
              <div className="space-y-2 text-xs">
                <div className="flex items-center justify-between py-1">
                  <span className="text-[#7a7568]">Admission</span>
                  <span className="px-2 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] rounded font-medium">
                    Eligible
                  </span>
                </div>
                <div className="flex items-center justify-between py-1">
                  <span className="text-[#7a7568]">Upstream ready</span>
                  <span className="px-2 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] rounded font-medium">
                    Yes
                  </span>
                </div>
                <div className="flex items-center justify-between py-1">
                  <span className="text-[#7a7568]">Handoff format</span>
                  <span className="font-mono text-[#1f1c17]">v1.0</span>
                </div>
              </div>
            </div>

            {/* Retained Runs */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
              <h3 className="text-sm font-semibold text-[#1f1c17] mb-3">Retained Runs (3)</h3>
              <div className="space-y-1 text-xs">
                <button className="w-full p-2 bg-[#f7f3ea] rounded hover:bg-[#e8e2d4] transition-colors text-left">
                  <div className="font-mono text-[#1f1c17]">run_247_001</div>
                  <div className="text-[#7a7568]">2026-04-03 14:32</div>
                </button>
                <button className="w-full p-2 bg-[#f7f3ea] rounded hover:bg-[#e8e2d4] transition-colors text-left">
                  <div className="font-mono text-[#1f1c17]">run_246_002</div>
                  <div className="text-[#7a7568]">2026-04-03 14:28</div>
                </button>
                <button className="w-full p-2 bg-[#f7f3ea] rounded hover:bg-[#e8e2d4] transition-colors text-left">
                  <div className="font-mono text-[#1f1c17]">run_245_001</div>
                  <div className="text-[#7a7568]">2026-04-03 14:25</div>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
