import { useState } from 'react';
import { Link } from 'react-router';
import {
  ArrowLeft, Plus, X, Play, Code, Settings, Search, 
  Grid3x3, PanelLeftClose, PanelRightClose, FileText,
  Activity, Database, Layers, FlaskConical, Target,
  CheckCircle2, BarChart3, GitCompare, Clock, Maximize2
} from 'lucide-react';
import { FormulaEditor } from '../components/FormulaEditor';
import { ResultPanel } from '../components/ResultPanel';

type PanelConfig = {
  editor: boolean;
  result: boolean;
  help: boolean;
  xray: boolean;
  compare: boolean;
  trace: boolean;
};

export function ModularEvidenceCockpit() {
  const [leftRailOpen, setLeftRailOpen] = useState(true);
  const [rightPanelOpen, setRightPanelOpen] = useState(true);
  const [panels, setPanels] = useState<PanelConfig>({
    editor: true,
    result: true,
    help: true,
    xray: true,
    compare: true,
    trace: true,
  });

  return (
    <div className="h-screen flex flex-col bg-[#1a1816] text-[#f5f1e8]">
      {/* Command Bar */}
      <header className="h-12 border-b border-[#f5f1e8]/10 bg-[#222018] flex items-center justify-between px-4">
        <div className="flex items-center gap-3">
          <Link to="/" className="flex items-center gap-2 text-[#a75842] hover:text-[#c77862] transition-colors">
            <ArrowLeft className="w-4 h-4" />
          </Link>
          <div className="h-4 w-px bg-[#f5f1e8]/10" />
          <Grid3x3 className="w-4 h-4 text-[#a75842]" />
          <span className="text-sm font-semibold">DNA OneCalc</span>
          <span className="text-xs text-[#b0a99a]">Modular Evidence Cockpit</span>
        </div>
        
        <div className="flex items-center gap-2">
          <button className="px-3 py-1 text-xs bg-[#3a3530] hover:bg-[#4a4540] rounded flex items-center gap-1.5 transition-colors">
            <Search className="w-3.5 h-3.5" />
            Quick Find
          </button>
          <div className="h-4 w-px bg-[#f5f1e8]/10" />
          <button className="p-1.5 hover:bg-[#3a3530] rounded transition-colors">
            <Settings className="w-4 h-4" />
          </button>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Collapsible Left Rail */}
        {leftRailOpen && (
          <aside className="w-56 border-r border-[#f5f1e8]/10 bg-[#222018] flex flex-col">
            <div className="p-3 border-b border-[#f5f1e8]/10 flex items-center justify-between">
              <span className="text-xs font-semibold uppercase tracking-wider text-[#b0a99a]">Workspace</span>
              <button
                onClick={() => setLeftRailOpen(false)}
                className="p-1 hover:bg-[#3a3530] rounded transition-colors"
              >
                <PanelLeftClose className="w-3.5 h-3.5" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-3 space-y-4">
              <div>
                <div className="text-xs text-[#b0a99a] mb-2 px-2">Active Spaces</div>
                <div className="space-y-1">
                  <div className="flex items-center gap-2 px-2 py-1.5 bg-[#a75842]/20 border border-[#a75842]/30 rounded text-sm">
                    <div className="w-1.5 h-1.5 rounded-full bg-[#a75842]" />
                    <FileText className="w-3.5 h-3.5" />
                    <span className="flex-1 truncate text-xs">Formula #247</span>
                    <div className="w-1.5 h-1.5 rounded-full bg-[#d69f4c]" />
                  </div>
                  <div className="flex items-center gap-2 px-2 py-1.5 hover:bg-[#3a3530] rounded text-sm cursor-pointer transition-colors">
                    <div className="w-1.5 h-1.5 rounded-full bg-[#b0a99a]" />
                    <FileText className="w-3.5 h-3.5 text-[#b0a99a]" />
                    <span className="flex-1 truncate text-xs text-[#b0a99a]">Array Ops</span>
                  </div>
                  <div className="flex items-center gap-2 px-2 py-1.5 hover:bg-[#3a3530] rounded text-sm cursor-pointer transition-colors">
                    <div className="w-1.5 h-1.5 rounded-full bg-[#b0a99a]" />
                    <FileText className="w-3.5 h-3.5 text-[#b0a99a]" />
                    <span className="flex-1 truncate text-xs text-[#b0a99a]">FILTER Tests</span>
                  </div>
                </div>
              </div>

              <div>
                <div className="text-xs text-[#b0a99a] mb-2 px-2">Evidence</div>
                <div className="space-y-1 text-xs">
                  <div className="flex items-center justify-between px-2 py-1.5 hover:bg-[#3a3530] rounded cursor-pointer transition-colors">
                    <span className="text-[#b0a99a]">Retained</span>
                    <span className="text-[#f5f1e8]">12</span>
                  </div>
                  <div className="flex items-center justify-between px-2 py-1.5 hover:bg-[#3a3530] rounded cursor-pointer transition-colors">
                    <span className="text-[#b0a99a]">Compared</span>
                    <span className="text-[#f5f1e8]">8</span>
                  </div>
                  <div className="flex items-center justify-between px-2 py-1.5 hover:bg-[#3a3530] rounded cursor-pointer transition-colors">
                    <span className="text-[#b0a99a]">Witnesses</span>
                    <span className="text-[#f5f1e8]">3</span>
                  </div>
                </div>
              </div>

              <div>
                <div className="text-xs text-[#b0a99a] mb-2 px-2">Layout</div>
                <div className="grid grid-cols-2 gap-2">
                  {Object.entries(panels).map(([key, value]) => (
                    <button
                      key={key}
                      onClick={() => setPanels({ ...panels, [key]: !value })}
                      className={`px-2 py-1.5 text-xs rounded transition-colors ${
                        value
                          ? 'bg-[#a75842] text-white'
                          : 'bg-[#3a3530] text-[#b0a99a] hover:bg-[#4a4540]'
                      }`}
                    >
                      {key.charAt(0).toUpperCase() + key.slice(1)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="p-3 border-t border-[#f5f1e8]/10">
              <button className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-[#a75842] hover:bg-[#c77862] text-white rounded transition-colors text-sm">
                <Plus className="w-4 h-4" />
                New Space
              </button>
            </div>
          </aside>
        )}

        {!leftRailOpen && (
          <button
            onClick={() => setLeftRailOpen(true)}
            className="w-12 border-r border-[#f5f1e8]/10 bg-[#222018] flex flex-col items-center py-4 gap-4"
          >
            <PanelLeftClose className="w-4 h-4 rotate-180" />
          </button>
        )}

        {/* Modular Panel Grid */}
        <main className="flex-1 overflow-hidden flex flex-col">
          {/* Tabs */}
          <div className="h-10 border-b border-[#f5f1e8]/10 bg-[#252220] flex items-center px-3 gap-1">
            <div className="flex items-center gap-2 px-3 py-1.5 bg-[#3a3530] border border-[#a75842]/30 rounded">
              <FileText className="w-3.5 h-3.5 text-[#a75842]" />
              <span className="text-xs">Formula #247</span>
              <div className="w-1.5 h-1.5 rounded-full bg-[#d69f4c]" />
              <X className="w-3 h-3 ml-1 text-[#b0a99a] hover:text-[#f5f1e8] cursor-pointer" />
            </div>
            <button className="p-1.5 hover:bg-[#3a3530] rounded transition-colors">
              <Plus className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* Panel Grid */}
          <div className="flex-1 overflow-auto p-3">
            <div className="grid grid-cols-12 gap-3 h-full">
              {/* Editor Panel */}
              {panels.editor && (
                <div className="col-span-6 row-span-2 bg-[#252220] border border-[#f5f1e8]/10 rounded-lg flex flex-col overflow-hidden">
                  <div className="px-4 py-3 border-b border-[#f5f1e8]/10 flex items-center justify-between bg-[#222018]">
                    <div className="flex items-center gap-2">
                      <Code className="w-4 h-4 text-[#a75842]" />
                      <span className="text-sm font-semibold">Formula Editor</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button className="px-2 py-1 text-xs bg-[#a75842] hover:bg-[#c77862] text-white rounded flex items-center gap-1.5 transition-colors">
                        <Play className="w-3 h-3" />
                        Run
                      </button>
                      <button className="p-1 hover:bg-[#3a3530] rounded transition-colors">
                        <Maximize2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                  <div className="flex-1 overflow-auto p-4">
                    <div className="bg-[#1a1816] border border-[#f5f1e8]/5 rounded p-3">
                      <FormulaEditor />
                    </div>
                  </div>
                </div>
              )}

              {/* Result Panel */}
              {panels.result && (
                <div className="col-span-3 row-span-1 bg-[#252220] border border-[#f5f1e8]/10 rounded-lg flex flex-col overflow-hidden">
                  <div className="px-4 py-3 border-b border-[#f5f1e8]/10 flex items-center justify-between bg-[#222018]">
                    <div className="flex items-center gap-2">
                      <Target className="w-4 h-4 text-[#2d5f5d]" />
                      <span className="text-sm font-semibold">Result</span>
                    </div>
                    <CheckCircle2 className="w-4 h-4 text-[#2d5f5d]" />
                  </div>
                  <div className="flex-1 overflow-auto p-4">
                    <div className="bg-[#1a1816] border border-[#2d5f5d]/20 rounded-lg p-6 text-center">
                      <div className="font-mono text-4xl text-[#2d5f5d] mb-2">30</div>
                      <div className="text-xs text-[#b0a99a]">Number • Scalar</div>
                    </div>
                  </div>
                </div>
              )}

              {/* Help Panel */}
              {panels.help && (
                <div className="col-span-3 row-span-1 bg-[#252220] border border-[#f5f1e8]/10 rounded-lg flex flex-col overflow-hidden">
                  <div className="px-4 py-3 border-b border-[#f5f1e8]/10 flex items-center gap-2 bg-[#222018]">
                    <Database className="w-4 h-4 text-[#d69f4c]" />
                    <span className="text-sm font-semibold">Function Info</span>
                  </div>
                  <div className="flex-1 overflow-auto p-4">
                    <div className="text-xs space-y-2">
                      <div>
                        <div className="text-[#b0a99a]">Function</div>
                        <div className="font-mono text-[#d69f4c]">LET</div>
                      </div>
                      <div>
                        <div className="text-[#b0a99a]">Category</div>
                        <div>Logical</div>
                      </div>
                      <div>
                        <div className="text-[#b0a99a]">Status</div>
                        <div className="text-[#2d5f5d]">Supported</div>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* X-Ray Panel */}
              {panels.xray && (
                <div className="col-span-4 row-span-2 bg-[#252220] border border-[#f5f1e8]/10 rounded-lg flex flex-col overflow-hidden">
                  <div className="px-4 py-3 border-b border-[#f5f1e8]/10 flex items-center gap-2 bg-[#222018]">
                    <Activity className="w-4 h-4 text-[#c65d47]" />
                    <span className="text-sm font-semibold">X-Ray Inspector</span>
                  </div>
                  <div className="flex-1 overflow-auto p-4 space-y-3">
                    <div className="bg-[#1a1816] border border-[#f5f1e8]/5 rounded p-3">
                      <div className="text-xs font-semibold text-[#c65d47] mb-2">Parse</div>
                      <div className="font-mono text-xs text-[#b0a99a] space-y-1">
                        <div>Status: Valid</div>
                        <div>Tokens: 24</div>
                        <div>Functions: 4</div>
                      </div>
                    </div>
                    <div className="bg-[#1a1816] border border-[#f5f1e8]/5 rounded p-3">
                      <div className="text-xs font-semibold text-[#c65d47] mb-2">Bind</div>
                      <div className="font-mono text-xs text-[#b0a99a] space-y-1">
                        <div>Variables: 2</div>
                        <div>References: 0</div>
                      </div>
                    </div>
                    <div className="bg-[#1a1816] border border-[#f5f1e8]/5 rounded p-3">
                      <div className="text-xs font-semibold text-[#c65d47] mb-2">Eval</div>
                      <div className="font-mono text-xs text-[#b0a99a] space-y-1">
                        <div>Steps: 7</div>
                        <div>Duration: 1.2ms</div>
                      </div>
                    </div>
                    <div className="bg-[#1a1816] border border-[#f5f1e8]/5 rounded p-3">
                      <div className="text-xs font-semibold text-[#c65d47] mb-2">Provenance</div>
                      <div className="font-mono text-xs text-[#b0a99a] space-y-1">
                        <div>Profile: H1</div>
                        <div>Floor: Full</div>
                        <div>Platform: Win/Web</div>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* Compare Panel */}
              {panels.compare && (
                <div className="col-span-4 row-span-1 bg-[#252220] border border-[#f5f1e8]/10 rounded-lg flex flex-col overflow-hidden">
                  <div className="px-4 py-3 border-b border-[#f5f1e8]/10 flex items-center justify-between bg-[#222018]">
                    <div className="flex items-center gap-2">
                      <GitCompare className="w-4 h-4 text-[#5a6f4d]" />
                      <span className="text-sm font-semibold">Comparison</span>
                    </div>
                    <div className="px-2 py-0.5 bg-[#2d5f5d] text-white text-xs rounded">Match</div>
                  </div>
                  <div className="flex-1 overflow-auto p-4">
                    <div className="grid grid-cols-2 gap-3">
                      <div className="bg-[#1a1816] border border-[#2d5f5d]/20 rounded p-3">
                        <div className="text-xs text-[#b0a99a] mb-2">DNA</div>
                        <div className="font-mono text-2xl text-[#2d5f5d]">30</div>
                      </div>
                      <div className="bg-[#1a1816] border border-[#c65d47]/20 rounded p-3">
                        <div className="text-xs text-[#b0a99a] mb-2">Excel</div>
                        <div className="font-mono text-2xl text-[#c65d47]">30</div>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* Trace Panel */}
              {panels.trace && (
                <div className="col-span-4 row-span-1 bg-[#252220] border border-[#f5f1e8]/10 rounded-lg flex flex-col overflow-hidden">
                  <div className="px-4 py-3 border-b border-[#f5f1e8]/10 flex items-center gap-2 bg-[#222018]">
                    <Layers className="w-4 h-4 text-[#7a8f6d]" />
                    <span className="text-sm font-semibold">Eval Trace</span>
                  </div>
                  <div className="flex-1 overflow-auto p-4">
                    <div className="space-y-2 font-mono text-xs">
                      <div className="flex items-center gap-2">
                        <div className="w-1.5 h-1.5 rounded-full bg-[#2d5f5d]" />
                        <span className="text-[#b0a99a]">SEQUENCE → [1..10]</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <div className="w-1.5 h-1.5 rounded-full bg-[#2d5f5d]" />
                        <span className="text-[#b0a99a]">FILTER → [2,4,6,8,10]</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <div className="w-1.5 h-1.5 rounded-full bg-[#d69f4c]" />
                        <span className="text-[#b0a99a]">SUM → 30</span>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Status Bar */}
          <div className="h-8 border-t border-[#f5f1e8]/10 bg-[#222018] flex items-center justify-between px-4 text-xs">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="w-3.5 h-3.5 text-[#2d5f5d]" />
                <span className="text-[#b0a99a]">Ready</span>
              </div>
              <div className="text-[#b0a99a]">H1-Standard</div>
              <div className="text-[#b0a99a]">OxFml v0.12.4</div>
            </div>
            <div className="flex items-center gap-4">
              <div className="text-[#b0a99a]">{Object.values(panels).filter(Boolean).length} panels active</div>
              <div className="flex items-center gap-1">
                <Clock className="w-3.5 h-3.5" />
                <span className="text-[#b0a99a]">1.2ms</span>
              </div>
            </div>
          </div>
        </main>

        {/* Right Panel (Evidence Workbench) */}
        {rightPanelOpen && (
          <aside className="w-72 border-l border-[#f5f1e8]/10 bg-[#222018] flex flex-col overflow-hidden">
            <div className="px-4 py-3 border-b border-[#f5f1e8]/10 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <FlaskConical className="w-4 h-4 text-[#a75842]" />
                <span className="text-sm font-semibold">Evidence Workbench</span>
              </div>
              <button
                onClick={() => setRightPanelOpen(false)}
                className="p-1 hover:bg-[#3a3530] rounded transition-colors"
              >
                <PanelRightClose className="w-3.5 h-3.5" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              <div className="bg-[#252220] border border-[#f5f1e8]/10 rounded-lg p-4">
                <div className="text-xs font-semibold text-[#a75842] mb-3">Scenario Info</div>
                <div className="space-y-2 text-xs">
                  <div className="flex justify-between">
                    <span className="text-[#b0a99a]">ID</span>
                    <span className="font-mono">scn_247</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-[#b0a99a]">Created</span>
                    <span>Apr 3, 14:32</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-[#b0a99a]">Runs</span>
                    <span>1</span>
                  </div>
                </div>
              </div>

              <div className="bg-[#252220] border border-[#f5f1e8]/10 rounded-lg p-4">
                <div className="text-xs font-semibold text-[#a75842] mb-3">Evidence Bundle</div>
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-xs">
                    <CheckCircle2 className="w-3.5 h-3.5 text-[#2d5f5d]" />
                    <span>Run captured</span>
                  </div>
                  <div className="flex items-center gap-2 text-xs">
                    <CheckCircle2 className="w-3.5 h-3.5 text-[#2d5f5d]" />
                    <span>Observation recorded</span>
                  </div>
                  <div className="flex items-center gap-2 text-xs">
                    <CheckCircle2 className="w-3.5 h-3.5 text-[#2d5f5d]" />
                    <span>Comparison complete</span>
                  </div>
                </div>
              </div>

              <div className="bg-[#252220] border border-[#f5f1e8]/10 rounded-lg p-4">
                <div className="text-xs font-semibold text-[#a75842] mb-3">Actions</div>
                <div className="space-y-2">
                  <button className="w-full px-3 py-2 bg-[#a75842] hover:bg-[#c77862] text-white text-xs rounded transition-colors">
                    Retain Evidence
                  </button>
                  <button className="w-full px-3 py-2 bg-[#3a3530] hover:bg-[#4a4540] text-xs rounded transition-colors">
                    Export Bundle
                  </button>
                  <button className="w-full px-3 py-2 bg-[#3a3530] hover:bg-[#4a4540] text-xs rounded transition-colors">
                    Generate Handoff
                  </button>
                </div>
              </div>

              <div className="bg-[#252220] border border-[#f5f1e8]/10 rounded-lg p-4">
                <div className="text-xs font-semibold text-[#a75842] mb-3">Capability</div>
                <div className="space-y-1 text-xs">
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-[#2d5f5d]" />
                    <span>All modes available</span>
                  </div>
                  <div className="text-[#b0a99a] mt-2">
                    Profile: H1-Standard<br />
                    Floor: Full
                  </div>
                </div>
              </div>
            </div>
          </aside>
        )}

        {!rightPanelOpen && (
          <button
            onClick={() => setRightPanelOpen(true)}
            className="w-12 border-l border-[#f5f1e8]/10 bg-[#222018] flex flex-col items-center py-4"
          >
            <PanelRightClose className="w-4 h-4 rotate-180" />
          </button>
        )}
      </div>
    </div>
  );
}
