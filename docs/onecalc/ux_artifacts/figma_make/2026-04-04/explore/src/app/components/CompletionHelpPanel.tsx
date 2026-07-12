import { useState } from 'react';
import { Search, BookOpen, Code, ChevronRight } from 'lucide-react';

export function CompletionHelpPanel() {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedFunction, setSelectedFunction] = useState('LET');

  const functions = [
    'ABS', 'ACCRINT', 'ACOS', 'AND', 'AVERAGE', 'CHOOSE', 'CLEAN', 'CODE', 
    'COLUMN', 'CONCAT', 'COUNT', 'COUNTA', 'COUNTIF', 'DATE', 'DAY', 'EVEN', 
    'FILTER', 'FIND', 'IF', 'IFERROR', 'INDEX', 'INT', 'ISBLANK', 'LET', 
    'LOWER', 'MATCH', 'MAX', 'MID', 'MIN', 'MOD', 'NOT', 'NOW', 'OR', 
    'RAND', 'ROUND', 'SEARCH', 'SEQUENCE', 'SUM', 'SUMIF', 'TEXT', 'TODAY', 
    'TRIM', 'UPPER', 'VLOOKUP', 'XLOOKUP'
  ];

  const filteredFunctions = searchQuery
    ? functions.filter(f => f.toLowerCase().includes(searchQuery.toLowerCase()))
    : functions;

  return (
    <div className="flex flex-col h-full border-l border-[#1f1c17]/10 bg-[#faf7f1]">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <h3 className="text-sm font-semibold text-[#1f1c17] mb-3">Function Reference</h3>
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-[#7a7568]" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search functions..."
            className="w-full pl-9 pr-3 py-2 text-sm bg-white border border-[#1f1c17]/15 rounded-lg focus:outline-none focus:ring-2 focus:ring-[#1e4d4a] placeholder:text-[#7a7568]"
          />
        </div>
        <div className="text-xs text-[#7a7568] mt-2">
          {filteredFunctions.length} of 517 functions
        </div>
      </div>

      {/* Split: Completions + Help */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Completion List */}
        <div className="flex-1 overflow-y-auto border-b border-[#1f1c17]/10">
          <div className="p-2">
            <div className="space-y-0.5">
              {filteredFunctions.slice(0, 20).map((func) => (
                <button
                  key={func}
                  onClick={() => setSelectedFunction(func)}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-lg transition-colors text-left ${
                    selectedFunction === func
                      ? 'bg-[#1e4d4a] text-white'
                      : 'hover:bg-white'
                  }`}
                >
                  <span className={`text-sm font-mono font-semibold ${
                    selectedFunction === func ? 'text-white' : 'text-[#1e4d4a]'
                  }`}>
                    {func}
                  </span>
                  <ChevronRight className={`w-3.5 h-3.5 ${
                    selectedFunction === func ? 'text-white' : 'text-[#7a7568]'
                  }`} />
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Current Help */}
        <div className="h-80 overflow-y-auto bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border-t-2 border-[#c88d2e]/30">
          <div className="p-4">
            <div className="flex items-center gap-2 mb-4">
              <BookOpen className="w-4 h-4 text-[#c88d2e]" />
              <h4 className="text-sm font-semibold text-[#1f1c17]">Current Help</h4>
            </div>

            <div className="space-y-4">
              {/* Function Header */}
              <div>
                <div className="font-mono text-lg text-[#1e4d4a] font-semibold mb-1">
                  {selectedFunction}
                </div>
                <div className="font-mono text-xs text-[#7a7568] bg-white px-3 py-2 rounded border border-[#1f1c17]/15">
                  {selectedFunction === 'LET' && 'LET(name1, value1, [name2, value2, ...], calculation)'}
                  {selectedFunction === 'SUM' && 'SUM(number1, [number2], ...)'}
                  {selectedFunction === 'FILTER' && 'FILTER(array, include, [if_empty])'}
                  {selectedFunction === 'SEQUENCE' && 'SEQUENCE(rows, [columns], [start], [step])'}
                  {selectedFunction !== 'LET' && selectedFunction !== 'SUM' && selectedFunction !== 'FILTER' && selectedFunction !== 'SEQUENCE' && `${selectedFunction}(...)`}
                </div>
              </div>

              {/* Description */}
              <div className="text-sm text-[#1f1c17] leading-relaxed">
                {selectedFunction === 'LET' && 'Assigns names to calculation results to allow storing intermediate calculations, values, and names inside a formula.'}
                {selectedFunction === 'SUM' && 'Adds all the numbers in a range of cells or provided as arguments.'}
                {selectedFunction === 'FILTER' && 'Filters a range of data based on criteria you define.'}
                {selectedFunction === 'SEQUENCE' && 'Generates a list of sequential numbers in an array.'}
                {selectedFunction !== 'LET' && selectedFunction !== 'SUM' && selectedFunction !== 'FILTER' && selectedFunction !== 'SEQUENCE' && `Function ${selectedFunction} documentation.`}
              </div>

              {/* Arguments */}
              {selectedFunction === 'LET' && (
                <div>
                  <div className="text-xs font-semibold text-[#1f1c17] mb-2">Arguments</div>
                  <div className="space-y-2 text-sm">
                    <div className="bg-white p-2.5 rounded border border-[#1f1c17]/15">
                      <div className="font-mono text-[#1e4d4a] font-semibold mb-0.5">name1</div>
                      <div className="text-xs text-[#7a7568]">Required. The name to assign.</div>
                    </div>
                    <div className="bg-white p-2.5 rounded border border-[#1f1c17]/15">
                      <div className="font-mono text-[#c88d2e] font-semibold mb-0.5">value1</div>
                      <div className="text-xs text-[#7a7568]">Required. The value to assign to name1.</div>
                    </div>
                    <div className="bg-white p-2.5 rounded border border-[#1f1c17]/15">
                      <div className="font-mono text-[#b84532] font-semibold mb-0.5">calculation</div>
                      <div className="text-xs text-[#7a7568]">Required. The final calculation using assigned names.</div>
                    </div>
                  </div>
                </div>
              )}

              {/* Metadata */}
              <div className="pt-3 border-t border-[#1f1c17]/10">
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div className="flex items-center justify-between py-1">
                    <span className="text-[#7a7568]">Category</span>
                    <span className="text-[#1f1c17] font-medium">
                      {selectedFunction === 'LET' || selectedFunction === 'IF' ? 'Logical' : 
                       selectedFunction === 'SUM' || selectedFunction === 'AVERAGE' ? 'Math' : 'Lookup'}
                    </span>
                  </div>
                  <div className="flex items-center justify-between py-1">
                    <span className="text-[#7a7568]">Status</span>
                    <span className="px-2 py-0.5 bg-[#1e4d4a] text-white rounded text-[10px] font-medium">
                      Supported
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
