import { useState } from 'react';
import { Play, AlertCircle, Grid3x3, Palette, Sliders } from 'lucide-react';

interface ExploreModeProps {
  onOpenDrawer: (drawer: 'completions' | 'help' | 'formatting') => void;
}

export function ExploreMode({ onOpenDrawer }: ExploreModeProps) {
  const [formula, setFormula] = useState('=LET(\n  values, SEQUENCE(10, 1, 1, 1),\n  filtered, FILTER(values, MOD(values, 2) = 0),\n  SUM(filtered)\n)');

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-7xl mx-auto space-y-6">
        {/* Formula Editor - Dominant */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="text-base font-semibold text-[#1f1c17]">Formula</h2>
            <div className="flex items-center gap-2">
              <button
                onClick={() => onOpenDrawer('formatting')}
                className="px-3 py-1.5 text-xs font-medium text-[#1f1c17] border border-[#1f1c17]/15 rounded hover:bg-[#f7f3ea] transition-colors flex items-center gap-2"
              >
                <Palette className="w-3.5 h-3.5" />
                Formatting
              </button>
              <button className="px-4 py-2 text-sm font-medium bg-[#1e4d4a] text-white rounded-lg hover:bg-[#2d6864] transition-colors flex items-center gap-2 shadow-sm">
                <Play className="w-4 h-4" />
                Evaluate
              </button>
            </div>
          </div>

          <div className="bg-white border-2 border-[#1f1c17]/15 rounded-xl overflow-hidden shadow-sm">
            <div className="h-10 bg-[#f7f3ea] border-b border-[#1f1c17]/10 flex items-center justify-between px-4">
              <div className="flex items-center gap-3 text-xs text-[#7a7568]">
                <span className="font-medium">4 lines</span>
                <span>•</span>
                <span>24 tokens</span>
              </div>
              <button
                onClick={() => onOpenDrawer('completions')}
                className="text-xs text-[#1e4d4a] hover:text-[#2d6864] font-medium"
              >
                Show completions →
              </button>
            </div>

            <div className="relative font-mono text-[15px] leading-7">
              <div className="flex">
                <div className="select-none w-12 py-4 text-right text-[#7a7568] bg-[#faf7f1] border-r border-[#1f1c17]/10">
                  {formula.split('\n').map((_, i) => (
                    <div key={i} className="px-3">{i + 1}</div>
                  ))}
                </div>
                <div className="flex-1 py-4 px-4">
                  <textarea
                    value={formula}
                    onChange={(e) => setFormula(e.target.value)}
                    className="w-full bg-transparent text-[#1f1c17] resize-none focus:outline-none"
                    rows={formula.split('\n').length}
                    spellCheck={false}
                  />
                </div>
              </div>
            </div>
          </div>

          {/* Diagnostics - Formula-space-level */}
          <div className="bg-[#1e4d4a]/5 border border-[#1e4d4a]/20 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <div className="w-5 h-5 rounded bg-[#1e4d4a] flex items-center justify-center flex-shrink-0 mt-0.5">
                <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                </svg>
              </div>
              <div className="flex-1">
                <div className="text-sm font-medium text-[#1f1c17] mb-1">No errors detected</div>
                <div className="text-sm text-[#7a7568]">Formula is valid • Ready to evaluate • 0 diagnostics</div>
              </div>
            </div>
          </div>
        </div>

        {/* Result - Prominent */}
        <div className="grid grid-cols-2 gap-6">
          <div className="space-y-3">
            <h3 className="text-base font-semibold text-[#1f1c17]">Result</h3>
            
            <div className="bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10 border-2 border-[#1e4d4a]/20 rounded-xl p-8">
              <div className="font-mono text-5xl text-[#1e4d4a] mb-3 font-semibold">
                30
              </div>
              <div className="text-sm text-[#7a7568] font-medium">
                Number • Scalar
              </div>
            </div>

            {/* Effective Display - Formula-space-level */}
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4 space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-semibold text-[#1f1c17]">Effective Display</h4>
                <button
                  onClick={() => onOpenDrawer('formatting')}
                  className="text-xs text-[#1e4d4a] hover:text-[#2d6864] font-medium"
                >
                  Edit →
                </button>
              </div>
              <div className="grid grid-cols-2 gap-2 text-xs">
                <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                  <span className="text-[#7a7568]">Format</span>
                  <span className="font-mono text-[#1f1c17]">General</span>
                </div>
                <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                  <span className="text-[#7a7568]">Precision</span>
                  <span className="font-mono text-[#1f1c17]">Auto</span>
                </div>
                <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                  <span className="text-[#7a7568]">Style</span>
                  <span className="font-mono text-[#1f1c17]">Default</span>
                </div>
                <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                  <span className="text-[#7a7568]">Color</span>
                  <span className="font-mono text-[#1f1c17]">Auto</span>
                </div>
              </div>
            </div>
          </div>

          {/* Array Preview (when applicable) */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-base font-semibold text-[#1f1c17]">Array Preview</h3>
              <button className="text-xs text-[#1e4d4a] hover:text-[#2d6864] font-medium">
                Expand grid →
              </button>
            </div>
            
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
              <div className="space-y-2 text-xs font-mono">
                <div className="flex items-center gap-2 text-[#7a7568]">
                  <Grid3x3 className="w-4 h-4" />
                  <span>Intermediate arrays in evaluation</span>
                </div>
                <div className="mt-3 space-y-2">
                  <div className="p-2 bg-[#f7f3ea] rounded">
                    <div className="text-[#7a7568] text-[10px] mb-1">values</div>
                    <div className="text-[#1f1c17]">[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]</div>
                  </div>
                  <div className="p-2 bg-[#f7f3ea] rounded">
                    <div className="text-[#7a7568] text-[10px] mb-1">filtered</div>
                    <div className="text-[#1f1c17]">[2, 4, 6, 8, 10]</div>
                  </div>
                </div>
              </div>
            </div>

            {/* Quick Actions */}
            <div className="bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border border-[#c88d2e]/20 rounded-lg p-4">
              <div className="text-xs font-semibold text-[#1f1c17] mb-3">Quick Actions</div>
              <div className="space-y-2">
                <button
                  onClick={() => onOpenDrawer('help')}
                  className="w-full px-3 py-2 text-xs font-medium text-[#1f1c17] bg-white border border-[#1f1c17]/15 rounded hover:bg-[#f7f3ea] transition-colors text-left"
                >
                  View function help
                </button>
                <button className="w-full px-3 py-2 text-xs font-medium text-[#1f1c17] bg-white border border-[#1f1c17]/15 rounded hover:bg-[#f7f3ea] transition-colors text-left">
                  Open in Inspect mode
                </button>
                <button className="w-full px-3 py-2 text-xs font-medium text-[#1f1c17] bg-white border border-[#1f1c17]/15 rounded hover:bg-[#f7f3ea] transition-colors text-left">
                  Capture as run
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
