import { useState } from 'react';
import { Play, AlertCircle, CheckCircle2, Info } from 'lucide-react';

interface FormulaEditorPanelProps {
  onEvaluate: () => void;
}

export function FormulaEditorPanel({ onEvaluate }: FormulaEditorPanelProps) {
  const [formula, setFormula] = useState('=LET(\n  values, SEQUENCE(10, 1, 1, 1),\n  filtered, FILTER(values, MOD(values, 2) = 0),\n  SUM(filtered)\n)');
  const lineCount = formula.split('\n').length;

  return (
    <div className="flex flex-col h-full">
      {/* Editor Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <div className="flex items-center gap-4">
          <h2 className="text-sm font-semibold text-[#1f1c17]">Formula</h2>
          <div className="flex items-center gap-3 text-xs text-[#7a7568]">
            <span>{lineCount} lines</span>
            <span>•</span>
            <span>24 tokens</span>
            <span>•</span>
            <span>4 functions</span>
          </div>
        </div>
        <button
          onClick={onEvaluate}
          className="flex items-center gap-2 px-4 py-2 bg-[#1e4d4a] text-white rounded-lg hover:bg-[#2d6864] transition-colors shadow-sm font-medium text-sm"
        >
          <Play className="w-4 h-4" />
          Evaluate
        </button>
      </div>

      {/* Editor Surface */}
      <div className="flex-1 bg-white border-b border-[#1f1c17]/10 relative overflow-hidden">
        <div className="absolute inset-0 flex">
          {/* Line Numbers */}
          <div className="select-none w-14 bg-[#faf7f1] border-r border-[#1f1c17]/10 py-4 text-right text-[#7a7568] font-mono text-sm">
            {Array.from({ length: lineCount }, (_, i) => (
              <div key={i} className="h-7 leading-7 px-3">
                {i + 1}
              </div>
            ))}
          </div>

          {/* Editor Content */}
          <div className="flex-1 overflow-auto">
            <textarea
              value={formula}
              onChange={(e) => setFormula(e.target.value)}
              className="w-full h-full px-4 py-4 font-mono text-[15px] leading-7 text-[#1f1c17] resize-none focus:outline-none"
              spellCheck={false}
              style={{
                minHeight: '100%',
                tabSize: 2,
              }}
            />
          </div>
        </div>
      </div>

      {/* Inline Diagnostics */}
      <div className="px-4 py-3 bg-[#1e4d4a]/5 border-b border-[#1e4d4a]/10">
        <div className="flex items-start gap-3">
          <div className="w-5 h-5 rounded bg-[#1e4d4a] flex items-center justify-center flex-shrink-0 mt-0.5">
            <CheckCircle2 className="w-3 h-3 text-white" />
          </div>
          <div className="flex-1">
            <div className="text-sm font-medium text-[#1f1c17] mb-0.5">No errors detected</div>
            <div className="text-sm text-[#7a7568]">Formula parsed successfully • Ready to evaluate</div>
          </div>
          <button className="text-xs text-[#1e4d4a] hover:text-[#2d6864] font-medium">
            View parse tree
          </button>
        </div>
      </div>
    </div>
  );
}
