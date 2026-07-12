import { useState } from 'react';
import { ChevronDown, ChevronRight, CheckCircle2, CircleDot, Layers, Database, Activity, Target, Flag } from 'lucide-react';
import { FormulaWalkInspector } from './FormulaWalkInspector';

interface InspectModeProps {
  onOpenDrawer: (drawer: 'details' | 'flags') => void;
}

export function InspectMode({ onOpenDrawer }: InspectModeProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['walk']));

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
          {/* Left Column: Formula Walk - Dominant */}
          <div className="col-span-2 space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-base font-semibold text-[#1f1c17]">Formula Walk</h2>
              <button
                onClick={() => onOpenDrawer('details')}
                className="text-xs text-[#3e5238] hover:text-[#566b4f] font-medium"
              >
                Show full tree details →
              </button>
            </div>

            <div className="bg-white border-2 border-[#3e5238]/20 rounded-xl p-6">
              <FormulaWalkInspector />
            </div>

            {/* Parse Summary */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
              <button
                onClick={() => toggleSection('parse')}
                className="w-full flex items-center justify-between px-4 py-3 hover:bg-[#f7f3ea] transition-colors text-left"
              >
                <div className="flex items-center gap-2">
                  <Layers className="w-5 h-5 text-[#1e4d4a]" />
                  <span className="text-sm font-semibold text-[#1f1c17]">Parse Context</span>
                </div>
                {expandedSections.has('parse') ? (
                  <ChevronDown className="w-4 h-4 text-[#7a7568]" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                )}
              </button>
              {expandedSections.has('parse') && (
                <div className="px-4 py-3 border-t border-[#1f1c17]/10">
                  <div className="grid grid-cols-3 gap-4 text-sm">
                    <div className="bg-[#f7f3ea] rounded-lg p-3">
                      <div className="text-xs text-[#7a7568] mb-1">Status</div>
                      <div className="font-semibold text-[#1e4d4a]">Valid</div>
                    </div>
                    <div className="bg-[#f7f3ea] rounded-lg p-3">
                      <div className="text-xs text-[#7a7568] mb-1">Tokens</div>
                      <div className="font-mono font-semibold text-[#1f1c17]">24</div>
                    </div>
                    <div className="bg-[#f7f3ea] rounded-lg p-3">
                      <div className="text-xs text-[#7a7568] mb-1">Functions</div>
                      <div className="font-mono font-semibold text-[#1f1c17]">4</div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Bind Summary */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
              <button
                onClick={() => toggleSection('bind')}
                className="w-full flex items-center justify-between px-4 py-3 hover:bg-[#f7f3ea] transition-colors text-left"
              >
                <div className="flex items-center gap-2">
                  <Database className="w-5 h-5 text-[#3e5238]" />
                  <span className="text-sm font-semibold text-[#1f1c17]">Bind Context</span>
                </div>
                {expandedSections.has('bind') ? (
                  <ChevronDown className="w-4 h-4 text-[#7a7568]" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                )}
              </button>
              {expandedSections.has('bind') && (
                <div className="px-4 py-3 border-t border-[#1f1c17]/10">
                  <div className="space-y-3 text-sm">
                    <div className="flex items-center justify-between p-3 bg-[#f7f3ea] rounded-lg">
                      <span className="text-[#7a7568]">Variables Bound</span>
                      <span className="font-mono font-semibold text-[#1f1c17]">2</span>
                    </div>
                    <div className="flex items-center justify-between p-3 bg-[#f7f3ea] rounded-lg">
                      <span className="text-[#7a7568]">External References</span>
                      <span className="font-mono font-semibold text-[#1f1c17]">0</span>
                    </div>
                    <div className="flex items-center justify-between p-3 bg-[#f7f3ea] rounded-lg">
                      <span className="text-[#7a7568]">Scope Depth</span>
                      <span className="font-mono font-semibold text-[#1f1c17]">1</span>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Eval Summary */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
              <button
                onClick={() => toggleSection('eval')}
                className="w-full flex items-center justify-between px-4 py-3 hover:bg-[#f7f3ea] transition-colors text-left"
              >
                <div className="flex items-center gap-2">
                  <Activity className="w-5 h-5 text-[#c88d2e]" />
                  <span className="text-sm font-semibold text-[#1f1c17]">Eval Context</span>
                </div>
                {expandedSections.has('eval') ? (
                  <ChevronDown className="w-4 h-4 text-[#7a7568]" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                )}
              </button>
              {expandedSections.has('eval') && (
                <div className="px-4 py-3 border-t border-[#1f1c17]/10">
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div className="bg-[#f7f3ea] rounded-lg p-3">
                      <div className="text-xs text-[#7a7568] mb-1">Evaluation Steps</div>
                      <div className="font-mono font-semibold text-[#1f1c17]">7</div>
                    </div>
                    <div className="bg-[#f7f3ea] rounded-lg p-3">
                      <div className="text-xs text-[#7a7568] mb-1">Duration</div>
                      <div className="font-mono font-semibold text-[#1f1c17]">1.2ms</div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Right Column: Host State & Context */}
          <div className="space-y-6">
            {/* Host State - Formula-space-level */}
            <div className="bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10 border-2 border-[#1e4d4a]/20 rounded-xl p-4">
              <div className="flex items-center gap-2 mb-4">
                <Target className="w-5 h-5 text-[#1e4d4a]" />
                <h3 className="text-sm font-semibold text-[#1f1c17]">Host State</h3>
              </div>
              <div className="space-y-2 text-xs">
                <div className="flex items-center justify-between py-1.5">
                  <span className="text-[#7a7568]">Profile</span>
                  <span className="font-mono font-semibold text-[#1e4d4a]">OC-H0</span>
                </div>
                <div className="flex items-center justify-between py-1.5">
                  <span className="text-[#7a7568]">Platform</span>
                  <span className="font-mono text-[#1f1c17]">Windows</span>
                </div>
                <div className="flex items-center justify-between py-1.5">
                  <span className="text-[#7a7568]">Runtime</span>
                  <span className="font-mono text-[#1f1c17]">Native</span>
                </div>
                <div className="flex items-center justify-between py-1.5">
                  <span className="text-[#7a7568]">Capability Floor</span>
                  <span className="font-mono font-semibold text-[#1e4d4a]">Full</span>
                </div>
              </div>
            </div>

            {/* Scenario-affecting Host Flags */}
            <div className="bg-white border border-[#c88d2e]/20 rounded-lg p-4">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-2">
                  <Flag className="w-4 h-4 text-[#c88d2e]" />
                  <h3 className="text-sm font-semibold text-[#1f1c17]">Scenario Flags</h3>
                </div>
                <button
                  onClick={() => onOpenDrawer('flags')}
                  className="text-xs text-[#c88d2e] hover:text-[#dda947] font-medium"
                >
                  Edit →
                </button>
              </div>
              <div className="space-y-2">
                <label className="flex items-center justify-between text-xs cursor-pointer">
                  <span className="text-[#7a7568]">Allow volatile</span>
                  <input type="checkbox" className="rounded border-[#1f1c17]/20" />
                </label>
                <label className="flex items-center justify-between text-xs cursor-pointer">
                  <span className="text-[#7a7568]">Freeze arrays</span>
                  <input type="checkbox" defaultChecked className="rounded border-[#1f1c17]/20" />
                </label>
                <label className="flex items-center justify-between text-xs cursor-pointer">
                  <span className="text-[#7a7568]">Cache results</span>
                  <input type="checkbox" defaultChecked className="rounded border-[#1f1c17]/20" />
                </label>
                <label className="flex items-center justify-between text-xs cursor-pointer">
                  <span className="text-[#7a7568]">Strict mode</span>
                  <input type="checkbox" className="rounded border-[#1f1c17]/20" />
                </label>
              </div>
            </div>

            {/* Packet Context - Run-level */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
              <h3 className="text-sm font-semibold text-[#1f1c17] mb-3">Packet Context</h3>
              <div className="space-y-2 text-xs font-mono">
                <div className="p-2 bg-[#f7f3ea] rounded">
                  <div className="text-[#7a7568] text-[10px] mb-1">Packet Kind</div>
                  <div className="text-[#1e4d4a]">edit_accept_recalc</div>
                </div>
                <div className="p-2 bg-[#f7f3ea] rounded">
                  <div className="text-[#7a7568] text-[10px] mb-1">Run Token</div>
                  <div className="text-[#1f1c17] break-all text-[10px]">e5f9254907f3ea74</div>
                </div>
                <div className="p-2 bg-[#f7f3ea] rounded">
                  <div className="text-[#7a7568] text-[10px] mb-1">Timing</div>
                  <div className="text-[#1f1c17]">1.2ms</div>
                </div>
              </div>
            </div>

            {/* Function Flags */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-semibold text-[#1f1c17]">Function Flags</h3>
                <button
                  onClick={() => onOpenDrawer('flags')}
                  className="text-xs text-[#1e4d4a] hover:text-[#2d6864] font-medium"
                >
                  View all →
                </button>
              </div>
              <div className="space-y-1.5 text-xs">
                <div className="flex items-center gap-2 p-2 bg-[#f7f3ea] rounded">
                  <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
                  <span className="text-[#1f1c17] font-mono">LET</span>
                  <span className="ml-auto text-[#7a7568]">Supported</span>
                </div>
                <div className="flex items-center gap-2 p-2 bg-[#f7f3ea] rounded">
                  <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
                  <span className="text-[#1f1c17] font-mono">SEQUENCE</span>
                  <span className="ml-auto text-[#7a7568]">Supported</span>
                </div>
                <div className="flex items-center gap-2 p-2 bg-[#f7f3ea] rounded">
                  <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
                  <span className="text-[#1f1c17] font-mono">FILTER</span>
                  <span className="ml-auto text-[#7a7568]">Supported</span>
                </div>
                <div className="flex items-center gap-2 p-2 bg-[#f7f3ea] rounded">
                  <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
                  <span className="text-[#1f1c17] font-mono">SUM</span>
                  <span className="ml-auto text-[#7a7568]">Supported</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
