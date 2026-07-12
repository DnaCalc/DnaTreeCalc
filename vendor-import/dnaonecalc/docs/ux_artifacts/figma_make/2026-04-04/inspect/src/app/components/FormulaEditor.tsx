import { useState } from 'react';

interface FormulaEditorProps {
  variant?: 'default' | 'focused' | 'compact';
  showLineNumbers?: boolean;
  className?: string;
}

export function FormulaEditor({ 
  variant = 'default', 
  showLineNumbers = true,
  className = '' 
}: FormulaEditorProps) {
  const [formula, setFormula] = useState(
    `=LET(\n  values, SEQUENCE(10, 1, 1, 1),\n  filtered, FILTER(values, MOD(values, 2) = 0),\n  SUM(filtered)\n)`
  );

  const lines = formula.split('\n');

  // Improved syntax tokenization
  const tokenizeLine = (line: string) => {
    const tokens: Array<{ text: string; type: string }> = [];
    let currentToken = '';
    let currentType = 'default';

    for (let i = 0; i < line.length; i++) {
      const char = line[i];
      
      // Check for function names (uppercase sequences)
      if (/[A-Z]/.test(char)) {
        if (currentType !== 'function') {
          if (currentToken) tokens.push({ text: currentToken, type: currentType });
          currentToken = char;
          currentType = 'function';
        } else {
          currentToken += char;
        }
      }
      // Check for numbers
      else if (/[0-9]/.test(char)) {
        if (currentType !== 'number') {
          if (currentToken) tokens.push({ text: currentToken, type: currentType });
          currentToken = char;
          currentType = 'number';
        } else {
          currentToken += char;
        }
      }
      // Check for operators
      else if (/[=(),]/.test(char)) {
        if (currentToken) tokens.push({ text: currentToken, type: currentType });
        tokens.push({ text: char, type: 'operator' });
        currentToken = '';
        currentType = 'default';
      }
      // Default characters
      else {
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
    <div className={`relative font-mono ${className}`}>
      <div className="flex relative z-10">
        {showLineNumbers && (
          <div className="select-none pr-4 text-right text-[#9a9a9a] text-sm border-r border-[#2a2a2a]/10">
            {lines.map((_, i) => (
              <div key={i} className="leading-6">
                {i + 1}
              </div>
            ))}
          </div>
        )}
        <div className="flex-1 pl-4">
          <textarea
            value={formula}
            onChange={(e) => setFormula(e.target.value)}
            className="w-full bg-transparent text-transparent caret-[#2a2a2a] text-sm leading-6 resize-none focus:outline-none relative z-20"
            rows={lines.length}
            spellCheck={false}
            style={{ caretColor: 'var(--foreground)' }}
          />
        </div>
      </div>
      
      {/* Syntax highlighting overlay */}
      <div className="absolute inset-0 pointer-events-none flex z-0">
        {showLineNumbers && <div className="pr-4 text-right text-sm border-r border-transparent">
          {lines.map((_, i) => (
            <div key={i} className="leading-6 opacity-0">{i + 1}</div>
          ))}
        </div>}
        <div className="flex-1 pl-4">
          {lines.map((line, i) => (
            <div key={i} className="text-sm leading-6">
              {tokenizeLine(line).map((token, j) => {
                let className = 'text-[#2a2a2a]';
                if (token.type === 'function') className = 'text-[#2d5f5d] font-semibold';
                if (token.type === 'number') className = 'text-[#d69f4c]';
                if (token.type === 'operator') className = 'text-[#c65d47]';
                
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
  );
}