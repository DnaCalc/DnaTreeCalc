import { ArrowLeft, CheckCircle2, Code } from 'lucide-react';

interface SourceFormulaPanelProps {
  onBackToExplore: () => void;
}

export function SourceFormulaPanel({ onBackToExplore }: SourceFormulaPanelProps) {
  const formula = `=LET(
  values, SEQUENCE(10, 1, 1, 1),
  filtered, FILTER(values, MOD(values, 2) = 0),
  SUM(filtered)
)`;

  return (
    <div className="flex flex-col h-full bg-[#faf7f1]">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <Code className="w-4 h-4 text-[#3e5238]" />
            <h2 className="text-sm font-semibold text-[#1f1c17]">Source Formula</h2>
          </div>
          <button
            onClick={onBackToExplore}
            className="flex items-center gap-1.5 text-xs text-[#3e5238] hover:text-[#566b4f] font-medium"
          >
            <ArrowLeft className="w-3 h-3" />
            Edit
          </button>
        </div>
        <div className="text-xs text-[#7a7568]">
          Read-only • Switch to Explore mode to edit
        </div>
      </div>

      {/* Compact Formula Display */}
      <div className="p-4 border-b border-[#1f1c17]/10 bg-white">
        <div className="font-mono text-xs leading-relaxed text-[#1f1c17] whitespace-pre-wrap">
          {formula}
        </div>
      </div>

      {/* Result Summary */}
      <div className="p-4 bg-[#faf7f1]">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-xs font-semibold text-[#1f1c17]">Result</h3>
          <CheckCircle2 className="w-3.5 h-3.5 text-[#3e5238]" />
        </div>
        <div className="bg-gradient-to-br from-[#3e5238]/5 to-[#3e5238]/10 border border-[#3e5238]/20 rounded-lg p-4">
          <div className="font-mono text-3xl text-[#3e5238] mb-1 font-semibold">
            30
          </div>
          <div className="text-xs text-[#7a7568]">Number • Scalar</div>
        </div>
      </div>

      {/* Quick Actions */}
      <div className="p-4 border-t border-[#1f1c17]/10">
        <button
          onClick={onBackToExplore}
          className="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-[#3e5238] text-white rounded-lg hover:bg-[#566b4f] transition-colors font-medium text-sm"
        >
          <ArrowLeft className="w-4 h-4" />
          Back to Explore
        </button>
      </div>
    </div>
  );
}
