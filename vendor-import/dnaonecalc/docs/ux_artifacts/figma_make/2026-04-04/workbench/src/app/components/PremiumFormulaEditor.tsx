import { useState } from 'react';
import { Play, BookOpen, AlertCircle } from 'lucide-react';

interface PremiumFormulaEditorProps {
  onEvaluate?: () => void;
}

export function PremiumFormulaEditor({ onEvaluate }: PremiumFormulaEditorProps) {
  const [formula, setFormula] = useState(
    `=LET(\n  values, SEQUENCE(10, 1, 1, 1),\n  filtered, FILTER(values, MOD(values, 2) = 0),\n  SUM(filtered)\n)`
  );

  const lines = formula.split('\n');

  const tokenizeLine = (line: string) => {
    const tokens: Array<{ text: string; type: string }> = [];
    let currentToken = '';
    let currentType = 'default';

    for (let i = 0; i < line.length; i++) {
      const char = line[i];
      
      if (/[A-Z]/.test(char)) {
        if (currentType !== 'function') {
          if (currentToken) tokens.push({ text: currentToken, type: currentType });
          currentToken = char;
          currentType = 'function';
        } else {
          currentToken += char;
        }
      } else if (/[0-9]/.test(char)) {
        if (currentType !== 'number') {
          if (currentToken) tokens.push({ text: currentToken, type: currentType });
          currentToken = char;
          currentType = 'number';
        } else {
          currentToken += char;
        }
      } else if (/[=(),]/.test(char)) {
        if (currentToken) tokens.push({ text: currentToken, type: currentType });
        tokens.push({ text: char, type: 'operator' });
        currentToken = '';
        currentType = 'default';
      } else {
        if (currentType !== 'default') {
          if (currentToken) tokens.push({ text: currentToken, type: currentType });
          currentToken = char;
          currentType = 'default';
        } else {
          currentToken += char;
        }
      }
    }

    if (currentToken) tokens.push({ text: currentToken, type: currentType });
    return tokens;
  };

  return (
    <div className="space-y-4">
      {/* Editor Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-semibold text-[#1f1c17]">Formula Editor</h2>
          <div className="px-2 py-1 bg-[#1e4d4a]/10 text-[#1e4d4a] text-xs font-medium rounded">
            Active
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button className="px-4 py-2 text-sm font-medium text-[#1f1c17] hover:bg-[#e8e2d4] rounded-lg transition-colors flex items-center gap-2">
            <BookOpen className="w-4 h-4" />
            Help
          </button>
          <button 
            onClick={onEvaluate}
            className="px-4 py-2 text-sm font-medium bg-[#1e4d4a] text-white rounded-lg hover:bg-[#2d6864] transition-colors flex items-center gap-2 shadow-sm"
          >
            <Play className="w-4 h-4" />
            Evaluate
          </button>
        </div>
      </div>

      {/* Editor Surface */}
      <div className="bg-white border-2 border-[#1f1c17]/15 rounded-xl overflow-hidden shadow-sm">
        {/* Editor Toolbar */}
        <div className="h-10 bg-[#f7f3ea] border-b border-[#1f1c17]/10 flex items-center justify-between px-4">
          <div className="flex items-center gap-3 text-xs text-[#7a7568]">
            <span className="font-medium">LET Formula Analysis</span>
            <span>•</span>
            <span>4 lines</span>
          </div>
          <div className="flex items-center gap-2 text-xs">
            <span className="text-[#7a7568]">Profile:</span>
            <span className="font-mono text-[#1e4d4a]">H1-Standard</span>
          </div>
        </div>

        {/* Editor Content */}
        <div className="relative font-mono text-[15px] leading-7">
          <div className="flex relative z-10">
            {/* Line Numbers */}
            <div className="select-none w-12 py-4 text-right text-[#7a7568] bg-[#faf7f1] border-r border-[#1f1c17]/10">
              {lines.map((_, i) => (
                <div key={i} className="px-3">
                  {i + 1}
                </div>
              ))}
            </div>

            {/* Editor Input */}
            <div className="flex-1 py-4 px-4">
              <textarea
                value={formula}
                onChange={(e) => setFormula(e.target.value)}
                className="w-full bg-transparent text-transparent caret-[#1f1c17] resize-none focus:outline-none relative z-20"
                rows={lines.length}
                spellCheck={false}
                style={{ caretColor: '#1f1c17' }}
              />
            </div>
          </div>
          
          {/* Syntax Highlighting Overlay */}
          <div className="absolute inset-0 pointer-events-none flex z-0">
            <div className="w-12" />
            <div className="flex-1 py-4 px-4">
              {lines.map((line, i) => (
                <div key={i} className="leading-7">
                  {tokenizeLine(line).map((token, j) => {
                    let className = 'text-[#1f1c17]';
                    if (token.type === 'function') className = 'text-[#1e4d4a] font-semibold';
                    if (token.type === 'number') className = 'text-[#c88d2e]';
                    if (token.type === 'operator') className = 'text-[#b84532]';
                    
                    return (
                      <span key={j} className={className}>
                        {token.text}
                      </span>
                    );
                  })}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Diagnostics (when present) */}
      <div className="bg-[#1e4d4a]/5 border border-[#1e4d4a]/20 rounded-lg p-4">
        <div className="flex items-start gap-3">
          <div className="w-5 h-5 rounded bg-[#1e4d4a] flex items-center justify-center flex-shrink-0 mt-0.5">
            <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
            </svg>
          </div>
          <div className="flex-1">
            <div className="text-sm font-medium text-[#1f1c17] mb-1">No errors detected</div>
            <div className="text-sm text-[#7a7568]">Formula is valid and ready to evaluate</div>
          </div>
        </div>
      </div>
    </div>
  );
}
