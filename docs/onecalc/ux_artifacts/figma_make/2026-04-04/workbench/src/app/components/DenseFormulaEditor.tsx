import { useState } from 'react';
import { Play, ChevronDown, ChevronRight, Settings, FileCode, AlertTriangle } from 'lucide-react';

interface DenseFormulaEditorProps {
  onEvaluate?: () => void;
}

export function DenseFormulaEditor({ onEvaluate }: DenseFormulaEditorProps) {
  const [formula, setFormula] = useState('=SUM(1,2,3)');
  const [showSettings, setShowSettings] = useState(false);

  return (
    <div className="space-y-3">
      {/* Compact Header with Settings */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold text-[#1f1c17]">Formula</h2>
          <button
            onClick={() => setShowSettings(!showSettings)}
            className="p-1 hover:bg-[#e8e2d4] rounded transition-colors"
          >
            <Settings className="w-3.5 h-3.5 text-[#7a7568]" />
          </button>
        </div>
        <div className="flex items-center gap-2">
          <button 
            onClick={onEvaluate}
            className="px-3 py-1.5 text-xs font-medium bg-[#1e4d4a] text-white rounded hover:bg-[#2d6864] transition-colors flex items-center gap-1.5 shadow-sm"
          >
            <Play className="w-3 h-3" />
            Evaluate
          </button>
        </div>
      </div>

      {/* Formula Input - More Compact */}
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
        <textarea
          value={formula}
          onChange={(e) => setFormula(e.target.value)}
          className="w-full px-3 py-2 font-mono text-sm text-[#1f1c17] resize-none focus:outline-none focus:ring-1 focus:ring-[#1e4d4a]"
          rows={1}
          spellCheck={false}
        />
      </div>

      {/* Warnings/Errors - Compact */}
      <div className="bg-[#c88d2e]/10 border border-[#c88d2e]/30 rounded px-3 py-1.5 flex items-start gap-2">
        <AlertTriangle className="w-3.5 h-3.5 text-[#c88d2e] flex-shrink-0 mt-0.5" />
        <div className="text-xs text-[#7a7568]">
          owner=11 selection=11..11 selected_text=""
        </div>
      </div>

      {/* Conditional Formatting Settings - Collapsible */}
      {showSettings && (
        <div className="bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border border-[#c88d2e]/20 rounded-lg p-3 space-y-2">
          <div className="text-xs font-semibold text-[#1f1c17] mb-2 flex items-center gap-1.5">
            <FileCode className="w-3.5 h-3.5 text-[#c88d2e]" />
            Conditional Formatting
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs">
            <label className="flex items-center gap-2">
              <input type="checkbox" className="rounded border-[#1f1c17]/20" />
              <span className="text-[#7a7568]">Formula color</span>
            </label>
            <label className="flex items-center gap-2">
              <input type="checkbox" className="rounded border-[#1f1c17]/20" />
              <span className="text-[#7a7568]">Result color</span>
            </label>
            <label className="flex items-center gap-2">
              <input type="checkbox" className="rounded border-[#1f1c17]/20" />
              <span className="text-[#7a7568]">Icon set</span>
            </label>
            <label className="flex items-center gap-2">
              <input type="checkbox" defaultChecked className="rounded border-[#1f1c17]/20" />
              <span className="text-[#7a7568]">Data bars</span>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}
