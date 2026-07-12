import { useState } from 'react';
import { Link } from 'react-router';
import { 
  Home, FileText, Clock, Star, Circle, Dot, X, Plus,
  ChevronRight, Play, Search, Settings, HelpCircle,
  Code, Activity, FlaskConical, ArrowLeft
} from 'lucide-react';
import { FormulaEditor } from '../components/FormulaEditor';
import { ResultPanel } from '../components/ResultPanel';

type ViewMode = 'explorer' | 'xray' | 'workbench';

export function WarmEditorialWorkbench() {
  const [viewMode, setViewMode] = useState<ViewMode>('explorer');
  const [xrayOpen, setXrayOpen] = useState(false);

  return (
    <div className="h-screen flex flex-col bg-[#faf8f3]">
      {/* Top Bar */}
      <header className="h-14 border-b border-[#2a2a2a]/10 bg-[#efebe1] flex items-center justify-between px-4">
        <div className="flex items-center gap-4">
          <Link to="/" className="flex items-center gap-2 text-[#2d5f5d] hover:text-[#3d7f7d] transition-colors">
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm font-medium">Back</span>
          </Link>
          <div className="h-6 w-px bg-[#2a2a2a]/10" />
          <h1 className="text-lg font-semibold text-[#2a2a2a]">DNA OneCalc</h1>
          <span className="text-sm text-[#6a6a6a]">Warm Editorial Workbench</span>
        </div>
        <div className="flex items-center gap-3">
          <button className="p-2 hover:bg-[#f5f1e8] rounded transition-colors">
            <Search className="w-4 h-4 text-[#6a6a6a]" />
          </button>
          <button className="p-2 hover:bg-[#f5f1e8] rounded transition-colors">
            <HelpCircle className="w-4 h-4 text-[#6a6a6a]" />
          </button>
          <button className="p-2 hover:bg-[#f5f1e8] rounded transition-colors">
            <Settings className="w-4 h-4 text-[#6a6a6a]" />
          </button>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Left Rail */}
        <aside className="w-60 border-r border-[#2a2a2a]/10 bg-[#efebe1] flex flex-col">
          <div className="p-4 border-b border-[#2a2a2a]/10">
            <button className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-[#2d5f5d] text-white rounded-lg hover:bg-[#3d7f7d] transition-colors">
              <Plus className="w-4 h-4" />
              <span className="text-sm font-medium">New Formula</span>
            </button>
          </div>

          <nav className="flex-1 overflow-y-auto p-3">
            <div className="mb-6">
              <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2 px-2">
                Workspace
              </div>
              <div className="space-y-1">
                <button className="w-full flex items-center gap-3 px-3 py-2 text-sm text-[#2a2a2a] hover:bg-[#f5f1e8] rounded-lg transition-colors">
                  <Home className="w-4 h-4" />
                  <span>Overview</span>
                </button>
                <button className="w-full flex items-center gap-3 px-3 py-2 text-sm text-[#2a2a2a] hover:bg-[#f5f1e8] rounded-lg transition-colors">
                  <Clock className="w-4 h-4" />
                  <span>Recent</span>
                </button>
                <button className="w-full flex items-center gap-3 px-3 py-2 text-sm text-[#2a2a2a] hover:bg-[#f5f1e8] rounded-lg transition-colors">
                  <Star className="w-4 h-4" />
                  <span>Pinned</span>
                </button>
              </div>
            </div>

            <div>
              <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2 px-2">
                Active Spaces
              </div>
              <div className="space-y-1">
                <button className="w-full flex items-center gap-2 px-3 py-2 text-sm bg-[#f5f1e8] text-[#2a2a2a] rounded-lg">
                  <Circle className="w-2 h-2 fill-[#2d5f5d] text-[#2d5f5d]" />
                  <FileText className="w-4 h-4" />
                  <span className="flex-1 text-left truncate">LET Formula Analysis</span>
                  <Dot className="w-3 h-3 text-[#d69f4c]" />
                </button>
                <button className="w-full flex items-center gap-2 px-3 py-2 text-sm text-[#6a6a6a] hover:bg-[#f5f1e8] rounded-lg transition-colors">
                  <Circle className="w-2 h-2" />
                  <FileText className="w-4 h-4" />
                  <span className="flex-1 text-left truncate">FILTER Examples</span>
                </button>
                <button className="w-full flex items-center gap-2 px-3 py-2 text-sm text-[#6a6a6a] hover:bg-[#f5f1e8] rounded-lg transition-colors">
                  <Circle className="w-2 h-2" />
                  <FileText className="w-4 h-4" />
                  <span className="flex-1 text-left truncate">Array Operations</span>
                </button>
              </div>
            </div>
          </nav>

          <div className="p-3 border-t border-[#2a2a2a]/10">
            <div className="bg-[#f5f1e8] rounded-lg p-3">
              <div className="text-xs font-medium text-[#2a2a2a] mb-1">Capability Status</div>
              <div className="text-xs text-[#6a6a6a]">All modes available</div>
            </div>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 flex flex-col overflow-hidden">
          {/* Tab Bar */}
          <div className="h-12 border-b border-[#2a2a2a]/10 bg-[#f5f1e8] flex items-center px-2 gap-1">
            <div className="flex items-center gap-1 px-3 py-1.5 bg-[#faf8f3] border border-[#2a2a2a]/10 rounded-lg">
              <FileText className="w-4 h-4 text-[#2d5f5d]" />
              <span className="text-sm text-[#2a2a2a] ml-1">LET Formula Analysis</span>
              <Dot className="w-3 h-3 text-[#d69f4c] ml-1" />
              <button className="ml-2 p-0.5 hover:bg-[#e8e4da] rounded transition-colors">
                <X className="w-3 h-3 text-[#6a6a6a]" />
              </button>
            </div>
            <div className="flex items-center gap-1 px-3 py-1.5 hover:bg-[#e8e4da] rounded-lg transition-colors cursor-pointer">
              <FileText className="w-4 h-4 text-[#6a6a6a]" />
              <span className="text-sm text-[#6a6a6a] ml-1">FILTER Examples</span>
            </div>
            <button className="p-2 hover:bg-[#e8e4da] rounded transition-colors">
              <Plus className="w-3 h-3 text-[#6a6a6a]" />
            </button>
          </div>

          {/* Mode Switcher */}
          <div className="h-10 border-b border-[#2a2a2a]/10 bg-[#faf8f3] flex items-center px-6 gap-6">
            <button
              onClick={() => setViewMode('explorer')}
              className={`text-sm font-medium pb-2 border-b-2 transition-colors ${
                viewMode === 'explorer'
                  ? 'border-[#2d5f5d] text-[#2d5f5d]'
                  : 'border-transparent text-[#6a6a6a] hover:text-[#2a2a2a]'
              }`}
            >
              Formula Explorer
            </button>
            <button
              onClick={() => {
                setViewMode('xray');
                setXrayOpen(true);
              }}
              className={`text-sm font-medium pb-2 border-b-2 transition-colors ${
                viewMode === 'xray'
                  ? 'border-[#2d5f5d] text-[#2d5f5d]'
                  : 'border-transparent text-[#6a6a6a] hover:text-[#2a2a2a]'
              }`}
            >
              Semantic X-Ray
            </button>
            <button
              onClick={() => setViewMode('workbench')}
              className={`text-sm font-medium pb-2 border-b-2 transition-colors ${
                viewMode === 'workbench'
                  ? 'border-[#2d5f5d] text-[#2d5f5d]'
                  : 'border-transparent text-[#6a6a6a] hover:text-[#2a2a2a]'
              }`}
            >
              Twin Oracle Workbench
            </button>
          </div>

          {/* Content Area */}
          <div className="flex-1 overflow-hidden flex">
            <div className={`flex-1 overflow-y-auto transition-all duration-300 ${xrayOpen ? 'mr-96' : ''}`}>
              {viewMode === 'explorer' && (
                <div className="p-6 max-w-5xl mx-auto">
                  <div className="mb-6">
                    <div className="flex items-center justify-between mb-3">
                      <h2 className="text-xl font-semibold text-[#2a2a2a]">Formula Editor</h2>
                      <div className="flex items-center gap-2">
                        <button className="px-3 py-1.5 text-sm bg-[#2d5f5d] text-white rounded-lg hover:bg-[#3d7f7d] transition-colors flex items-center gap-2">
                          <Play className="w-3.5 h-3.5" />
                          Evaluate
                        </button>
                        <button
                          onClick={() => setXrayOpen(!xrayOpen)}
                          className="px-3 py-1.5 text-sm border border-[#2a2a2a]/20 text-[#2a2a2a] rounded-lg hover:bg-[#f5f1e8] transition-colors flex items-center gap-2"
                        >
                          <Code className="w-3.5 h-3.5" />
                          X-Ray
                        </button>
                      </div>
                    </div>
                    <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-4">
                      <FormulaEditor />
                    </div>
                  </div>

                  <ResultPanel variant="detailed" className="mb-6" />

                  <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                    <h3 className="font-medium text-[#2a2a2a] mb-4">Function Help</h3>
                    <div className="space-y-3">
                      <div>
                        <div className="font-mono text-sm text-[#2d5f5d] mb-1">LET(name1, value1, [name2, value2, ...], calculation)</div>
                        <div className="text-sm text-[#6a6a6a]">Assigns names to calculation results to allow storing intermediate calculations.</div>
                      </div>
                      <div className="grid grid-cols-2 gap-4 pt-3 border-t border-[#2a2a2a]/10">
                        <div>
                          <div className="text-xs font-medium text-[#6a6a6a] mb-1">Category</div>
                          <div className="text-sm text-[#2a2a2a]">Logical</div>
                        </div>
                        <div>
                          <div className="text-xs font-medium text-[#6a6a6a] mb-1">Status</div>
                          <div className="text-sm text-[#2d5f5d]">Supported</div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {viewMode === 'xray' && (
                <div className="p-6 max-w-5xl mx-auto">
                  <h2 className="text-xl font-semibold text-[#2a2a2a] mb-6">Live Semantic X-Ray</h2>
                  
                  <div className="space-y-4">
                    <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                      <h3 className="font-medium text-[#2a2a2a] mb-3">Parse Tree</h3>
                      <div className="font-mono text-xs text-[#2a2a2a] space-y-1">
                        <div>FunctionCall: LET</div>
                        <div className="pl-4">├─ Argument: values</div>
                        <div className="pl-4">├─ FunctionCall: SEQUENCE</div>
                        <div className="pl-8">└─ Arguments: (10, 1, 1, 1)</div>
                        <div className="pl-4">├─ Argument: filtered</div>
                        <div className="pl-4">└─ Body: SUM(filtered)</div>
                      </div>
                    </div>

                    <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                      <h3 className="font-medium text-[#2a2a2a] mb-3">Evaluation Trace</h3>
                      <div className="space-y-2 text-sm">
                        <div className="flex items-center gap-3">
                          <div className="w-1.5 h-1.5 rounded-full bg-[#2d5f5d]" />
                          <span className="text-[#6a6a6a]">Bind values →</span>
                          <span className="text-[#2a2a2a]">SEQUENCE(10,1,1,1) = [1,2,3..10]</span>
                        </div>
                        <div className="flex items-center gap-3">
                          <div className="w-1.5 h-1.5 rounded-full bg-[#2d5f5d]" />
                          <span className="text-[#6a6a6a]">Bind filtered →</span>
                          <span className="text-[#2a2a2a]">FILTER([1..10], MOD=0) = [2,4,6,8,10]</span>
                        </div>
                        <div className="flex items-center gap-3">
                          <div className="w-1.5 h-1.5 rounded-full bg-[#d69f4c]" />
                          <span className="text-[#6a6a6a]">Evaluate SUM →</span>
                          <span className="text-[#2a2a2a]">30</span>
                        </div>
                      </div>
                    </div>

                    <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                      <h3 className="font-medium text-[#2a2a2a] mb-3">Provenance</h3>
                      <div className="grid grid-cols-2 gap-4 text-sm">
                        <div>
                          <div className="text-[#6a6a6a] mb-1">Host Profile</div>
                          <div className="text-[#2a2a2a] font-mono">H1-Standard</div>
                        </div>
                        <div>
                          <div className="text-[#6a6a6a] mb-1">Function Surface</div>
                          <div className="text-[#2a2a2a]">Supported</div>
                        </div>
                        <div>
                          <div className="text-[#6a6a6a] mb-1">Capability Floor</div>
                          <div className="text-[#2a2a2a]">Full</div>
                        </div>
                        <div>
                          <div className="text-[#6a6a6a] mb-1">Platform</div>
                          <div className="text-[#2a2a2a]">Windows/Browser</div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {viewMode === 'workbench' && (
                <div className="p-6 max-w-5xl mx-auto">
                  <h2 className="text-xl font-semibold text-[#2a2a2a] mb-6">Twin Oracle Workbench</h2>
                  
                  <div className="space-y-4">
                    <div className="bg-[#2d5f5d]/5 border border-[#2d5f5d]/20 rounded-lg p-6">
                      <div className="flex items-center gap-3 mb-4">
                        <div className="w-10 h-10 rounded-lg bg-[#2d5f5d] flex items-center justify-center">
                          <FlaskConical className="w-5 h-5 text-white" />
                        </div>
                        <div>
                          <h3 className="font-medium text-[#2a2a2a]">Comparison Result</h3>
                          <p className="text-sm text-[#6a6a6a]">DNA vs Excel Observed</p>
                        </div>
                        <div className="ml-auto px-3 py-1 bg-[#2d5f5d] text-white text-sm rounded-full">
                          Match
                        </div>
                      </div>
                      
                      <div className="grid grid-cols-2 gap-6">
                        <div>
                          <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2">DNA OneCalc</div>
                          <div className="bg-white/50 rounded-lg p-4">
                            <div className="font-mono text-2xl text-[#2a2a2a]">30</div>
                            <div className="text-sm text-[#6a6a6a] mt-1">Number</div>
                          </div>
                        </div>
                        <div>
                          <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2">Excel Observed</div>
                          <div className="bg-white/50 rounded-lg p-4">
                            <div className="font-mono text-2xl text-[#2a2a2a]">30</div>
                            <div className="text-sm text-[#6a6a6a] mt-1">Number</div>
                          </div>
                        </div>
                      </div>
                    </div>

                    <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                      <h3 className="font-medium text-[#2a2a2a] mb-4">Evidence Lineage</h3>
                      <div className="space-y-3">
                        <div className="flex items-center gap-3">
                          <ChevronRight className="w-4 h-4 text-[#6a6a6a]" />
                          <div className="flex-1">
                            <div className="text-sm text-[#2a2a2a]">Scenario: LET Formula Analysis</div>
                            <div className="text-xs text-[#6a6a6a]">Created 2026-04-03 14:32</div>
                          </div>
                        </div>
                        <div className="flex items-center gap-3 pl-7">
                          <ChevronRight className="w-4 h-4 text-[#6a6a6a]" />
                          <div className="flex-1">
                            <div className="text-sm text-[#2a2a2a]">Run #247</div>
                            <div className="text-xs text-[#6a6a6a]">OxFml v0.12.4 • OxFunc v0.8.2</div>
                          </div>
                        </div>
                        <div className="flex items-center gap-3 pl-14">
                          <ChevronRight className="w-4 h-4 text-[#6a6a6a]" />
                          <div className="flex-1">
                            <div className="text-sm text-[#2a2a2a]">Observation via OxXlPlay</div>
                            <div className="text-xs text-[#6a6a6a]">Excel 365 • Windows 11</div>
                          </div>
                        </div>
                      </div>
                    </div>

                    <div className="flex gap-3">
                      <button className="flex-1 px-4 py-3 bg-[#2d5f5d] text-white rounded-lg hover:bg-[#3d7f7d] transition-colors">
                        Retain as Evidence
                      </button>
                      <button className="flex-1 px-4 py-3 border border-[#2a2a2a]/20 text-[#2a2a2a] rounded-lg hover:bg-[#f5f1e8] transition-colors">
                        Export Handoff Packet
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* X-Ray Drawer */}
            {xrayOpen && (
              <aside className="w-96 border-l border-[#2a2a2a]/10 bg-[#f5f1e8] overflow-y-auto">
                <div className="sticky top-0 bg-[#f5f1e8] border-b border-[#2a2a2a]/10 p-4 flex items-center justify-between">
                  <h3 className="font-semibold text-[#2a2a2a]">X-Ray Inspector</h3>
                  <button
                    onClick={() => setXrayOpen(false)}
                    className="p-1.5 hover:bg-[#e8e4da] rounded transition-colors"
                  >
                    <X className="w-4 h-4 text-[#6a6a6a]" />
                  </button>
                </div>
                
                <div className="p-4 space-y-4">
                  <div>
                    <h4 className="text-sm font-medium text-[#2a2a2a] mb-2">Parse Context</h4>
                    <div className="bg-white/50 rounded-lg p-3 text-xs font-mono text-[#2a2a2a]">
                      Status: Valid<br />
                      Tokens: 24<br />
                      Functions: 4
                    </div>
                  </div>

                  <div>
                    <h4 className="text-sm font-medium text-[#2a2a2a] mb-2">Bind Context</h4>
                    <div className="bg-white/50 rounded-lg p-3 text-xs space-y-1">
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Variables</span>
                        <span className="text-[#2a2a2a]">2</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">References</span>
                        <span className="text-[#2a2a2a]">0</span>
                      </div>
                    </div>
                  </div>

                  <div>
                    <h4 className="text-sm font-medium text-[#2a2a2a] mb-2">Eval Context</h4>
                    <div className="bg-white/50 rounded-lg p-3 text-xs space-y-1">
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Steps</span>
                        <span className="text-[#2a2a2a]">7</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Duration</span>
                        <span className="text-[#2a2a2a]">1.2ms</span>
                      </div>
                    </div>
                  </div>

                  <div>
                    <h4 className="text-sm font-medium text-[#2a2a2a] mb-2">Capability</h4>
                    <div className="bg-white/50 rounded-lg p-3 text-xs space-y-1">
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Profile</span>
                        <span className="text-[#2a2a2a]">H1</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Floor</span>
                        <span className="text-[#2d5f5d]">Full</span>
                      </div>
                    </div>
                  </div>
                </div>
              </aside>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
