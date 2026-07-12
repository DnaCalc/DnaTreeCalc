import { CheckCircle2, Grid3x3, Palette, ChevronRight } from 'lucide-react';

interface ResultPanelProps {
  onOpenFormatting: () => void;
}

export function ResultPanel({ onOpenFormatting }: ResultPanelProps) {
  return (
    <div className="flex flex-col h-full bg-[#faf7f1]">
      {/* Result Header */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-[#1f1c17]">Result</h2>
          <div className="flex items-center gap-2 text-xs">
            <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
            <span className="text-[#7a7568]">Evaluated 1.2ms</span>
          </div>
        </div>
      </div>

      {/* Result Value - Dominant */}
      <div className="p-6">
        <div className="bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10 border-2 border-[#1e4d4a]/20 rounded-xl p-8">
          <div className="font-mono text-6xl text-[#1e4d4a] mb-3 font-semibold">
            30
          </div>
          <div className="text-sm text-[#7a7568] font-medium">
            Number • Scalar
          </div>
        </div>
      </div>

      {/* Effective Display - Inline */}
      <div className="px-6 pb-4">
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-semibold text-[#1f1c17]">Effective Display</h3>
            <button
              onClick={onOpenFormatting}
              className="flex items-center gap-1.5 text-xs text-[#1e4d4a] hover:text-[#2d6864] font-medium"
            >
              Edit
              <ChevronRight className="w-3 h-3" />
            </button>
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="flex items-center justify-between py-1.5 px-2.5 bg-[#f7f3ea] rounded">
              <span className="text-[#7a7568]">Format</span>
              <span className="font-mono text-[#1f1c17]">General</span>
            </div>
            <div className="flex items-center justify-between py-1.5 px-2.5 bg-[#f7f3ea] rounded">
              <span className="text-[#7a7568]">Precision</span>
              <span className="font-mono text-[#1f1c17]">Auto</span>
            </div>
            <div className="flex items-center justify-between py-1.5 px-2.5 bg-[#f7f3ea] rounded">
              <span className="text-[#7a7568]">Style</span>
              <span className="font-mono text-[#1f1c17]">Default</span>
            </div>
            <div className="flex items-center justify-between py-1.5 px-2.5 bg-[#f7f3ea] rounded">
              <span className="text-[#7a7568]">Color</span>
              <span className="font-mono text-[#1f1c17]">Auto</span>
            </div>
          </div>
        </div>
      </div>

      {/* Array Preview - When Relevant */}
      <div className="flex-1 px-6 pb-6 overflow-y-auto">
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Grid3x3 className="w-4 h-4 text-[#3e5238]" />
              <h3 className="text-sm font-semibold text-[#1f1c17]">Array Preview</h3>
            </div>
            <button className="text-xs text-[#3e5238] hover:text-[#566b4f] font-medium flex items-center gap-1">
              Expand
              <ChevronRight className="w-3 h-3" />
            </button>
          </div>
          
          <div className="space-y-2 text-xs">
            <div className="text-[#7a7568] mb-2">Intermediate arrays in evaluation</div>
            <div className="space-y-2">
              <div className="p-2.5 bg-[#f7f3ea] rounded-lg">
                <div className="text-[#7a7568] text-[10px] mb-1 font-medium">values</div>
                <div className="font-mono text-[#1f1c17] text-xs">
                  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                </div>
              </div>
              <div className="p-2.5 bg-[#f7f3ea] rounded-lg">
                <div className="text-[#7a7568] text-[10px] mb-1 font-medium">filtered</div>
                <div className="font-mono text-[#1f1c17] text-xs">
                  [2, 4, 6, 8, 10]
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
