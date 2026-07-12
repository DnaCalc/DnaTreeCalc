import { X, GitBranch, Lock, Info, BookOpen } from 'lucide-react';

type DrawerType = 'provenance' | 'context' | 'node' | null;

interface InspectDrawerProps {
  type: DrawerType;
  onClose: () => void;
}

export function InspectDrawer({ type, onClose }: InspectDrawerProps) {
  if (!type) return null;

  return (
    <aside className="w-[380px] border-l border-[#1f1c17]/10 bg-[#f7f3ea] overflow-y-auto flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-0 bg-[#ede7da] border-b border-[#1f1c17]/10 p-4 z-10">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {type === 'provenance' && <GitBranch className="w-5 h-5 text-[#3e5238]" />}
            {type === 'context' && <Info className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'node' && <BookOpen className="w-5 h-5 text-[#1e4d4a]" />}
            <h3 className="text-sm font-semibold text-[#1f1c17]">
              {type === 'provenance' && 'Provenance Chain'}
              {type === 'context' && 'Host Context Detail'}
              {type === 'node' && 'Node Detail'}
            </h3>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-[#f7f3ea] rounded transition-colors"
          >
            <X className="w-4 h-4 text-[#7a7568]" />
          </button>
        </div>
        <div className="text-xs text-[#7a7568] mt-1">
          {type === 'provenance' && 'Trace the origin and transformation of this value'}
          {type === 'context' && 'Host environment and scenario configuration'}
          {type === 'node' && 'Detailed information about selected node'}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-4">
        {type === 'provenance' && <ProvenanceContent />}
        {type === 'context' && <ContextContent />}
        {type === 'node' && <NodeContent />}
      </div>
    </aside>
  );
}

function ProvenanceContent() {
  return (
    <div className="space-y-4">
      {/* Value Chain */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Value Chain</h4>
        <div className="space-y-3">
          {/* Step 1 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#3e5238] border-2 border-white shadow" />
            <div className="absolute left-[5px] top-5 bottom-0 w-0.5 bg-[#3e5238]/20" />
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
              <div className="text-xs font-semibold text-[#1f1c17] mb-1">Source: SEQUENCE</div>
              <code className="font-mono text-xs text-[#7a7568] block mb-2">
                SEQUENCE(10, 1, 1, 1)
              </code>
              <div className="text-xs text-[#7a7568]">
                Generated array [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
              </div>
            </div>
          </div>

          {/* Step 2 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#c88d2e] border-2 border-white shadow" />
            <div className="absolute left-[5px] top-5 bottom-0 w-0.5 bg-[#c88d2e]/20" />
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
              <div className="text-xs font-semibold text-[#1f1c17] mb-1">Binding: values</div>
              <div className="text-xs text-[#7a7568] mb-2">
                Named reference created in LET scope
              </div>
              <code className="font-mono text-xs text-[#c88d2e]">
                values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
              </code>
            </div>
          </div>

          {/* Step 3 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#1e4d4a] border-2 border-white shadow" />
            <div className="absolute left-[5px] top-5 bottom-0 w-0.5 bg-[#1e4d4a]/20" />
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
              <div className="text-xs font-semibold text-[#1f1c17] mb-1">Transform: FILTER</div>
              <code className="font-mono text-xs text-[#7a7568] block mb-2">
                FILTER(values, MOD(values, 2) = 0)
              </code>
              <div className="text-xs text-[#7a7568]">
                Filtered to [2, 4, 6, 8, 10]
              </div>
            </div>
          </div>

          {/* Step 4 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#c88d2e] border-2 border-white shadow" />
            <div className="absolute left-[5px] top-5 bottom-0 w-0.5 bg-[#c88d2e]/20" />
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
              <div className="text-xs font-semibold text-[#1f1c17] mb-1">Binding: filtered</div>
              <div className="text-xs text-[#7a7568] mb-2">
                Named reference created in LET scope
              </div>
              <code className="font-mono text-xs text-[#c88d2e]">
                filtered = [2, 4, 6, 8, 10]
              </code>
            </div>
          </div>

          {/* Step 5 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#1e4d4a] border-2 border-white shadow" />
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
              <div className="text-xs font-semibold text-[#1f1c17] mb-1">Result: SUM</div>
              <code className="font-mono text-xs text-[#7a7568] block mb-2">
                SUM(filtered)
              </code>
              <div className="text-xs text-[#7a7568] mb-2">
                Final result: 30
              </div>
              <div className="text-xs text-[#3e5238] font-medium">
                ✓ Evaluation complete
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Metadata */}
      <div className="pt-4 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Metadata</h4>
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3 space-y-2 text-xs">
          <div className="flex items-center justify-between">
            <span className="text-[#7a7568]">Transformations</span>
            <span className="font-mono text-[#1f1c17]">2</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[#7a7568]">Bindings</span>
            <span className="font-mono text-[#1f1c17]">2</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[#7a7568]">Array operations</span>
            <span className="font-mono text-[#1f1c17]">2</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function ContextContent() {
  return (
    <div className="space-y-6">
      {/* Host Profile */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Host Profile</h4>
        <div className="bg-gradient-to-br from-[#3e5238]/5 to-[#3e5238]/10 border border-[#3e5238]/20 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-3">
            <div className="font-mono text-lg font-semibold text-[#3e5238]">OC-H0</div>
          </div>
          <div className="space-y-2 text-xs">
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">Profile name</span>
              <span className="text-[#1f1c17] font-medium">OneCalc Default</span>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">Function set</span>
              <span className="text-[#1f1c17] font-medium">Excel-compatible</span>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">Array support</span>
              <span className="text-[#3e5238] font-medium">Dynamic</span>
            </div>
          </div>
        </div>
      </div>

      {/* Scenario Policy */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Scenario Policy</h4>
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-semibold text-[#1f1c17]">Deterministic</span>
            <span className="px-2 py-0.5 bg-[#3e5238] text-white rounded text-[10px] font-medium">
              Active
            </span>
          </div>
          <div className="text-xs text-[#7a7568] leading-relaxed">
            NOW(), TODAY(), RAND() frozen at scenario creation. All evaluations produce consistent results.
          </div>
        </div>
      </div>

      {/* Scenario Flags */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Scenario Flags</h4>
        <div className="space-y-2">
          <div className="bg-white border border-[#3e5238]/20 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1">
              <div className="w-2 h-2 rounded-full bg-[#3e5238]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Freeze intermediate arrays</span>
            </div>
            <div className="text-xs text-[#7a7568]">
              Enabled • Required for inspection mode
            </div>
          </div>
          <div className="bg-white border border-[#3e5238]/20 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1">
              <div className="w-2 h-2 rounded-full bg-[#3e5238]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Result caching</span>
            </div>
            <div className="text-xs text-[#7a7568]">
              Enabled • Cache identical formula results
            </div>
          </div>
          <div className="bg-white border border-[#7a7568]/20 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1">
              <div className="w-2 h-2 rounded-full bg-[#7a7568]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Volatile functions</span>
            </div>
            <div className="text-xs text-[#7a7568]">
              Disabled • Controlled by Deterministic policy
            </div>
          </div>
        </div>
      </div>

      {/* Environment */}
      <div className="pt-4 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Environment</h4>
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3 space-y-2 text-xs">
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">OxFml version</span>
            <span className="font-mono text-[#1f1c17]">v0.12.4</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">OxFunc version</span>
            <span className="font-mono text-[#1f1c17]">v0.8.2</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Locale</span>
            <span className="font-mono text-[#1f1c17]">en-US</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Date format</span>
            <span className="font-mono text-[#1f1c17]">MM/DD/YYYY</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Decimal separator</span>
            <span className="font-mono text-[#1f1c17]">.</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function NodeContent() {
  return (
    <div className="space-y-6">
      {/* Node Info */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Node Information</h4>
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-3">
            <div className="px-2 py-1 bg-[#1e4d4a]/10 border border-[#1e4d4a]/20 rounded text-xs font-semibold text-[#1e4d4a]">
              EVALUATED
            </div>
          </div>
          <code className="font-mono text-sm text-[#1f1c17] block mb-3">
            FILTER(values, MOD(values, 2) = 0)
          </code>
          <div className="text-xs text-[#7a7568] mb-4">
            Filter even numbers from values array
          </div>
          <div className="space-y-2 text-xs">
            <div className="flex items-center justify-between py-1.5 border-b border-[#1f1c17]/10">
              <span className="text-[#7a7568]">Type</span>
              <span className="font-mono text-[#1f1c17]">Array[5]</span>
            </div>
            <div className="flex items-center justify-between py-1.5 border-b border-[#1f1c17]/10">
              <span className="text-[#7a7568]">Result</span>
              <span className="font-mono text-[#1e4d4a] font-semibold">[2, 4, 6, 8, 10]</span>
            </div>
            <div className="flex items-center justify-between py-1.5">
              <span className="text-[#7a7568]">Eval time</span>
              <span className="font-mono text-[#1f1c17]">0.3ms</span>
            </div>
          </div>
        </div>
      </div>

      {/* Function Signature */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Function Signature</h4>
        <div className="bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border border-[#c88d2e]/20 rounded-lg p-4">
          <div className="font-mono text-sm text-[#1f1c17] mb-3">
            FILTER(array, include, [if_empty])
          </div>
          <div className="space-y-2">
            <div className="bg-white p-2.5 rounded border border-[#1f1c17]/10">
              <div className="font-mono text-xs text-[#1e4d4a] font-semibold mb-0.5">array</div>
              <div className="text-xs text-[#7a7568]">Required. The range or array to filter.</div>
            </div>
            <div className="bg-white p-2.5 rounded border border-[#1f1c17]/10">
              <div className="font-mono text-xs text-[#c88d2e] font-semibold mb-0.5">include</div>
              <div className="text-xs text-[#7a7568]">Required. Boolean array of rows to keep.</div>
            </div>
            <div className="bg-white p-2.5 rounded border border-[#1f1c17]/10">
              <div className="font-mono text-xs text-[#7a7568] font-semibold mb-0.5">if_empty</div>
              <div className="text-xs text-[#7a7568]">Optional. Value if no rows match.</div>
            </div>
          </div>
        </div>
      </div>

      {/* Opaque/Blocked Reasons (when applicable) */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Inspection Status</h4>
        <div className="bg-[#3e5238]/5 border border-[#3e5238]/20 rounded-lg p-3 flex items-start gap-2">
          <Info className="w-4 h-4 text-[#3e5238] flex-shrink-0 mt-0.5" />
          <div className="text-xs text-[#7a7568] leading-relaxed">
            Full evaluation detail available. All subexpressions and intermediate values are visible.
          </div>
        </div>
      </div>
    </div>
  );
}
