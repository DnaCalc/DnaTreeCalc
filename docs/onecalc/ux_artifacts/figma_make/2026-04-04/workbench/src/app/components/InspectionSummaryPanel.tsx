import { FileCode, Link2, Zap, Server, Flag, ChevronRight } from 'lucide-react';

export function InspectionSummaryPanel() {
  return (
    <div className="flex flex-col h-full bg-[#faf7f1] border-l border-[#1f1c17]/10">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <h2 className="text-sm font-semibold text-[#1f1c17]">Inspection Summary</h2>
      </div>

      {/* Parse Summary */}
      <div className="border-b border-[#1f1c17]/10 bg-white">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 mb-3">
            <FileCode className="w-4 h-4 text-[#3e5238]" />
            <h3 className="text-sm font-semibold text-[#1f1c17]">Parse</h3>
          </div>
          <div className="space-y-2 text-xs">
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Status</span>
              <span className="font-medium text-[#3e5238]">Success</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Tokens</span>
              <span className="font-mono text-[#1f1c17]">24</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Functions</span>
              <span className="font-mono text-[#1f1c17]">4</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Depth</span>
              <span className="font-mono text-[#1f1c17]">3</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Bindings</span>
              <span className="font-mono text-[#1f1c17]">2</span>
            </div>
          </div>
        </div>
      </div>

      {/* Bind Summary */}
      <div className="border-b border-[#1f1c17]/10 bg-white">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 mb-3">
            <Link2 className="w-4 h-4 text-[#c88d2e]" />
            <h3 className="text-sm font-semibold text-[#1f1c17]">Bind</h3>
          </div>
          <div className="space-y-2">
            <div className="p-2 bg-[#c88d2e]/5 border border-[#c88d2e]/20 rounded">
              <div className="flex items-center justify-between mb-1">
                <code className="font-mono text-xs font-semibold text-[#c88d2e]">values</code>
                <span className="text-[10px] text-[#7a7568]">Array[10]</span>
              </div>
              <div className="text-xs text-[#7a7568]">
                Bound to SEQUENCE(10, 1, 1, 1)
              </div>
            </div>
            <div className="p-2 bg-[#c88d2e]/5 border border-[#c88d2e]/20 rounded">
              <div className="flex items-center justify-between mb-1">
                <code className="font-mono text-xs font-semibold text-[#c88d2e]">filtered</code>
                <span className="text-[10px] text-[#7a7568]">Array[5]</span>
              </div>
              <div className="text-xs text-[#7a7568]">
                Bound to FILTER(values, ...)
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Eval Summary */}
      <div className="border-b border-[#1f1c17]/10 bg-white">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 mb-3">
            <Zap className="w-4 h-4 text-[#1e4d4a]" />
            <h3 className="text-sm font-semibold text-[#1f1c17]">Eval</h3>
          </div>
          <div className="space-y-2 text-xs">
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Status</span>
              <span className="font-medium text-[#1e4d4a]">Complete</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Duration</span>
              <span className="font-mono text-[#1f1c17]">1.2ms</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Nodes</span>
              <span className="font-mono text-[#1f1c17]">11</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Arrays</span>
              <span className="font-mono text-[#1f1c17]">3</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Cache hits</span>
              <span className="font-mono text-[#1f1c17]">0</span>
            </div>
          </div>
        </div>
      </div>

      {/* Host Context */}
      <div className="border-b border-[#1f1c17]/10 bg-gradient-to-br from-[#3e5238]/5 to-[#3e5238]/10">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 mb-3">
            <Server className="w-4 h-4 text-[#3e5238]" />
            <h3 className="text-sm font-semibold text-[#1f1c17]">Host Context</h3>
          </div>
          <div className="space-y-2 text-xs">
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Profile</span>
              <span className="font-mono font-semibold text-[#3e5238]">OC-H0</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">OxFml</span>
              <span className="font-mono text-[#1f1c17]">v0.12.4</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">OxFunc</span>
              <span className="font-mono text-[#1f1c17]">v0.8.2</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Locale</span>
              <span className="font-mono text-[#1f1c17]">en-US</span>
            </div>
          </div>
        </div>
      </div>

      {/* Scenario Flags */}
      <div className="flex-1 overflow-y-auto">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 mb-3">
            <Flag className="w-4 h-4 text-[#3e5238]" />
            <h3 className="text-sm font-semibold text-[#1f1c17]">Active Flags</h3>
          </div>
          <div className="space-y-2">
            <div className="flex items-start gap-2 p-2 bg-white border border-[#3e5238]/20 rounded text-xs">
              <div className="w-1.5 h-1.5 rounded-full bg-[#3e5238] flex-shrink-0 mt-1.5" />
              <div className="flex-1">
                <div className="text-[#1f1c17] font-medium mb-0.5">Freeze intermediate arrays</div>
                <div className="text-[#7a7568]">Enabled for inspection</div>
              </div>
            </div>
            <div className="flex items-start gap-2 p-2 bg-white border border-[#3e5238]/20 rounded text-xs">
              <div className="w-1.5 h-1.5 rounded-full bg-[#3e5238] flex-shrink-0 mt-1.5" />
              <div className="flex-1">
                <div className="text-[#1f1c17] font-medium mb-0.5">Result caching</div>
                <div className="text-[#7a7568]">Enabled</div>
              </div>
            </div>
            <div className="flex items-start gap-2 p-2 bg-white border border-[#7a7568]/20 rounded text-xs">
              <div className="w-1.5 h-1.5 rounded-full bg-[#7a7568] flex-shrink-0 mt-1.5" />
              <div className="flex-1">
                <div className="text-[#1f1c17] font-medium mb-0.5">Volatile functions</div>
                <div className="text-[#7a7568]">Disabled (Deterministic)</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
