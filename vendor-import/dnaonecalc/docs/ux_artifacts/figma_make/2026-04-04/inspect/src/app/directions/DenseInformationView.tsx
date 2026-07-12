import { useState } from 'react';
import { Link } from 'react-router';
import { 
  Home, FileText, Clock, Star, Circle, Dot, X, Plus,
  ChevronRight, ArrowLeft, Search, Settings, HelpCircle,
  List, BookOpen
} from 'lucide-react';
import { DenseSystemBar } from '../components/DenseSystemBar';
import { DenseFormulaEditor } from '../components/DenseFormulaEditor';
import { DenseResultPanel } from '../components/DenseResultPanel';
import { DenseInspectorDrawer } from '../components/DenseInspectorDrawer';
import { FormulaWalkInspector } from '../components/FormulaWalkInspector';

type ViewMode = 'explorer' | 'workbench';

export function DenseInformationView() {
  const [viewMode, setViewMode] = useState<ViewMode>('explorer');
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [capabilityOpen, setCapabilityOpen] = useState(false);
  const [completionsOpen, setCompletionsOpen] = useState(true);

  return (
    <div className="h-screen flex flex-col bg-[#faf7f1]">
      {/* Top Bar */}
      <header className="h-12 border-b border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-4">
        <div className="flex items-center gap-4">
          <Link to="/" className="flex items-center gap-2 text-[#1e4d4a] hover:text-[#2d6864] transition-colors">
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm font-medium">Back</span>
          </Link>
          <div className="h-6 w-px bg-[#1f1c17]/10" />
          <h1 className="text-base font-semibold text-[#1f1c17]">DNA OneCalc</h1>
          <span className="text-sm text-[#7a7568]">Dense Information Mode</span>
        </div>
        <div className="flex items-center gap-3">
          <button className="p-1.5 hover:bg-[#f7f3ea] rounded transition-colors">
            <Search className="w-4 h-4 text-[#7a7568]" />
          </button>
          <button className="p-1.5 hover:bg-[#f7f3ea] rounded transition-colors">
            <HelpCircle className="w-4 h-4 text-[#7a7568]" />
          </button>
          <button className="p-1.5 hover:bg-[#f7f3ea] rounded transition-colors">
            <Settings className="w-4 h-4 text-[#7a7568]" />
          </button>
        </div>
      </header>

      {/* Dense System Bar */}
      <DenseSystemBar
        onToggleInspector={() => setInspectorOpen(!inspectorOpen)}
        onToggleCapability={() => setCapabilityOpen(!capabilityOpen)}
        inspectorOpen={inspectorOpen}
        capabilityOpen={capabilityOpen}
      />

      <div className="flex-1 flex overflow-hidden">
        {/* Left Rail - Compact */}
        <aside className="w-56 border-r border-[#1f1c17]/10 bg-[#ede7da] flex flex-col text-xs">
          <div className="p-3 border-b border-[#1f1c17]/10">
            <button className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-[#1e4d4a] text-white rounded hover:bg-[#2d6864] transition-all shadow-sm">
              <Plus className="w-3.5 h-3.5" />
              <span className="text-xs font-medium">New Formula Space</span>
            </button>
          </div>

          <nav className="flex-1 overflow-y-auto p-3">
            <div className="mb-4">
              <div className="text-[10px] font-semibold text-[#7a7568] uppercase tracking-wider mb-2 px-2">
                Quick Access
              </div>
              <div className="space-y-0.5">
                <button className="w-full flex items-center gap-2 px-2 py-1.5 text-xs text-[#1f1c17] hover:bg-[#f7f3ea] rounded transition-colors">
                  <Home className="w-3.5 h-3.5 text-[#1e4d4a]" />
                  <span>Overview</span>
                </button>
                <button className="w-full flex items-center gap-2 px-2 py-1.5 text-xs text-[#1f1c17] hover:bg-[#f7f3ea] rounded transition-colors">
                  <Clock className="w-3.5 h-3.5 text-[#c88d2e]" />
                  <span>Recent</span>
                </button>
                <button className="w-full flex items-center gap-2 px-2 py-1.5 text-xs text-[#1f1c17] hover:bg-[#f7f3ea] rounded transition-colors">
                  <Star className="w-3.5 h-3.5 text-[#b84532]" />
                  <span>Pinned</span>
                </button>
              </div>
            </div>

            <div>
              <div className="text-[10px] font-semibold text-[#7a7568] uppercase tracking-wider mb-2 px-2">
                Formula Spaces
              </div>
              <div className="space-y-0.5">
                <button className="w-full flex items-center gap-2 px-2 py-1.5 text-xs bg-[#f7f3ea] text-[#1f1c17] rounded border-l-3 border-[#1e4d4a]">
                  <FileText className="w-3.5 h-3.5 text-[#1e4d4a]" />
                  <span className="flex-1 text-left truncate font-medium">SUM Example</span>
                  <Dot className="w-3 h-3 text-[#c88d2e]" />
                </button>
                <button className="w-full flex items-center gap-2 px-2 py-1.5 text-xs text-[#7a7568] hover:bg-[#f7f3ea] rounded transition-colors">
                  <FileText className="w-3.5 h-3.5" />
                  <span className="flex-1 text-left truncate">FILTER Examples</span>
                </button>
                <button className="w-full flex items-center gap-2 px-2 py-1.5 text-xs text-[#7a7568] hover:bg-[#f7f3ea] rounded transition-colors">
                  <FileText className="w-3.5 h-3.5" />
                  <span className="flex-1 text-left truncate">Array Operations</span>
                </button>
              </div>
            </div>
          </nav>

          <div className="p-3 border-t border-[#1f1c17]/10">
            <div className="bg-gradient-to-br from-[#1e4d4a]/10 to-[#3e5238]/10 border border-[#1e4d4a]/30 rounded p-2">
              <div className="flex items-center gap-1.5 mb-1.5">
                <div className="w-1.5 h-1.5 rounded-full bg-[#1e4d4a]" />
                <div className="text-[10px] font-semibold text-[#1f1c17]">Ready</div>
              </div>
              <div className="text-[10px] text-[#7a7568] space-y-0.5 font-mono">
                <div>OC-H0 • Full</div>
                <div>517 functions</div>
              </div>
            </div>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 flex flex-col overflow-hidden">
          {/* Tab Bar - Compact */}
          <div className="h-10 border-b border-[#1f1c17]/10 bg-[#f7f3ea] flex items-center px-2 gap-1">
            <div className="flex items-center gap-2 px-2 py-1.5 bg-white border-l-3 border-[#1e4d4a] rounded shadow-sm">
              <FileText className="w-3.5 h-3.5 text-[#1e4d4a]" />
              <span className="text-xs text-[#1f1c17] font-medium">SUM Example</span>
              <Dot className="w-3 h-3 text-[#c88d2e] ml-1" />
              <button className="ml-1 p-0.5 hover:bg-[#e8e2d4] rounded transition-colors">
                <X className="w-3 h-3 text-[#7a7568]" />
              </button>
            </div>
            <div className="flex items-center gap-2 px-2 py-1.5 hover:bg-[#e8e2d4] rounded transition-colors cursor-pointer">
              <FileText className="w-3.5 h-3.5 text-[#7a7568]" />
              <span className="text-xs text-[#7a7568]">FILTER Examples</span>
            </div>
            <button className="p-1.5 hover:bg-[#e8e2d4] rounded transition-colors">
              <Plus className="w-3 h-3 text-[#7a7568]" />
            </button>
          </div>

          {/* View Switcher - Compact */}
          <div className="h-9 border-b border-[#1f1c17]/10 bg-gradient-to-r from-[#faf7f1] to-[#f7f3ea] flex items-center px-4 gap-4">
            <button
              onClick={() => setViewMode('explorer')}
              className={`text-xs font-medium pb-2 border-b-2 transition-colors ${
                viewMode === 'explorer'
                  ? 'border-[#1e4d4a] text-[#1e4d4a]'
                  : 'border-transparent text-[#7a7568] hover:text-[#1f1c17]'
              }`}
            >
              Formula Explorer
            </button>
            <button
              onClick={() => setViewMode('workbench')}
              className={`text-xs font-medium pb-2 border-b-2 transition-colors ${
                viewMode === 'workbench'
                  ? 'border-[#b84532] text-[#b84532]'
                  : 'border-transparent text-[#7a7568] hover:text-[#1f1c17]'
              }`}
            >
              Evidence Workbench
            </button>
          </div>

          {/* Content Area */}
          <div className="flex-1 overflow-hidden flex">
            <div className={`flex-1 overflow-y-auto transition-all duration-300 ${inspectorOpen ? 'mr-[420px]' : ''}`}>
              {viewMode === 'explorer' && (
                <div className="p-4">
                  <div className="grid grid-cols-4 gap-4">
                    {/* Column 1: Formula + Result */}
                    <div className="col-span-1 space-y-4">
                      <DenseFormulaEditor onEvaluate={() => setInspectorOpen(!inspectorOpen)} />
                      <DenseResultPanel />
                    </div>

                    {/* Column 2: Completions */}
                    <div className="col-span-1">
                      <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
                        <div className="px-3 py-2 bg-[#f7f3ea] border-b border-[#1f1c17]/10 flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <List className="w-4 h-4 text-[#c88d2e]" />
                            <span className="text-xs font-semibold text-[#1f1c17]">Completions (517)</span>
                          </div>
                          <button onClick={() => setCompletionsOpen(!completionsOpen)}>
                            <ChevronRight className={`w-3.5 h-3.5 text-[#7a7568] transition-transform ${completionsOpen ? 'rotate-90' : ''}`} />
                          </button>
                        </div>
                        {completionsOpen && (
                          <div className="max-h-[500px] overflow-y-auto">
                            <div className="divide-y divide-[#1f1c17]/5">
                              {['ABS', 'ACCRINT', 'ACCRINTM', 'ACOS', 'ACOSH', 'ACOT', 'AND', 'AVERAGE', 'AVERAGEIF', 'CHOOSE', 'CLEAN', 'CODE', 'COLUMN', 'COLUMNS', 'CONCAT', 'COUNT', 'COUNTA', 'COUNTBLANK', 'COUNTIF', 'DATE', 'DAY', 'EVEN', 'EXACT', 'EXP', 'FILTER', 'FIND', 'FLOOR', 'IF', 'IFERROR', 'INDEX', 'INT', 'ISBLANK', 'ISERROR', 'ISEVEN', 'ISNUMBER', 'ISODD', 'ISTEXT', 'LEFT', 'LEN', 'LET', 'LOWER', 'MATCH', 'MAX', 'MID', 'MIN', 'MOD', 'MONTH', 'NOT', 'NOW', 'ODD', 'OR', 'POWER', 'PROPER', 'RAND', 'REPLACE', 'REPT', 'RIGHT', 'ROUND', 'ROW', 'ROWS', 'SEARCH', 'SEQUENCE', 'SIGN', 'SIN', 'SORT', 'SQRT', 'SUBSTITUTE', 'SUM', 'SUMIF', 'TEXT', 'TODAY', 'TRIM', 'UPPER', 'VALUE', 'VLOOKUP', 'WEEKDAY', 'XLOOKUP', 'YEAR'].map((func) => (
                                <button
                                  key={func}
                                  className="w-full px-3 py-1.5 text-left hover:bg-[#f7f3ea] transition-colors flex items-center gap-2"
                                >
                                  <span className="text-[10px] text-[#7a7568]">Function</span>
                                  <span className="text-xs font-mono text-[#1e4d4a] font-semibold">{func}</span>
                                </button>
                              ))}
                            </div>
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Column 3: Current Help */}
                    <div className="col-span-1">
                      <div className="bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border-2 border-[#c88d2e]/20 rounded-lg p-4 space-y-3">
                        <div className="flex items-center gap-2">
                          <BookOpen className="w-4 h-4 text-[#c88d2e]" />
                          <h3 className="text-xs font-semibold text-[#1f1c17]">Current Help</h3>
                        </div>
                        
                        <div>
                          <div className="font-mono text-sm text-[#1e4d4a] font-semibold mb-1">SUM</div>
                          <div className="font-mono text-xs text-[#7a7568] mb-2">SUM(...)</div>
                          <div className="text-xs text-[#7a7568] leading-relaxed">
                            Adds all the numbers in a range of cells.
                          </div>
                        </div>

                        <div className="pt-2 border-t border-[#1f1c17]/10">
                          <div className="text-[10px] font-medium text-[#7a7568] mb-2">Active Arguments</div>
                          <div className="space-y-1.5 text-xs">
                            <div>
                              <span className="font-mono text-[#1e4d4a] font-semibold">num1</span>
                              <span className="text-[#7a7568] ml-1">– First number</span>
                            </div>
                            <div>
                              <span className="font-mono text-[#c88d2e] font-semibold">num2</span>
                              <span className="text-[#7a7568] ml-1">– Second number</span>
                            </div>
                            <div>
                              <span className="font-mono text-[#b84532] font-semibold">num3</span>
                              <span className="text-[#7a7568] ml-1">– Third number</span>
                            </div>
                          </div>
                        </div>

                        <div className="pt-2 border-t border-[#1f1c17]/10 text-xs space-y-1">
                          <div className="flex items-center justify-between">
                            <span className="text-[#7a7568]">Category</span>
                            <span className="text-[#1f1c17] font-medium">Math</span>
                          </div>
                          <div className="flex items-center justify-between">
                            <span className="text-[#7a7568]">Status</span>
                            <span className="px-2 py-0.5 bg-[#1e4d4a] text-white text-[10px] font-medium rounded">
                              Supported
                            </span>
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Column 4: Formula Walk */}
                    <div className="col-span-1">
                      <div className="bg-white border-2 border-[#3e5238]/20 rounded-lg p-3">
                        <div className="flex items-center justify-between mb-3">
                          <h3 className="text-xs font-semibold text-[#1f1c17] flex items-center gap-2">
                            <span className="w-2 h-2 rounded-full bg-[#3e5238]" />
                            Formula Walk
                          </h3>
                          <button
                            onClick={() => setInspectorOpen(true)}
                            className="text-[10px] text-[#1e4d4a] hover:text-[#2d6864] font-medium"
                          >
                            Full →
                          </button>
                        </div>
                        <FormulaWalkInspector />
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Inspector Drawer */}
            {inspectorOpen && <DenseInspectorDrawer onClose={() => setInspectorOpen(false)} />}
          </div>
        </main>
      </div>

      {/* Compact Status Footer */}
      <footer className="h-7 border-t border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-4 text-[10px] font-mono">
        <div className="flex items-center gap-4 text-[#7a7568]">
          <span className="text-[#1e4d4a] font-semibold">Ready</span>
          <span>OC-H0</span>
          <span>OxFml v0.12.4</span>
          <span>OxFunc v0.8.2</span>
        </div>
        <div className="flex items-center gap-4 text-[#7a7568]">
          <span>1.2ms</span>
          <span>DNA OneCalc v0.1.0</span>
        </div>
      </footer>
    </div>
  );
}
