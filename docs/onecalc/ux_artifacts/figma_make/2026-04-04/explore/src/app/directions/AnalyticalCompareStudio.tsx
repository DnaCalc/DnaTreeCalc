import { useState } from 'react';
import { Link } from 'react-router';
import {
  ArrowLeft, FileText, Plus, X, Circle, Dot, Play, Code,
  GitCompare, Layers, FlaskConical, CheckCircle2, AlertCircle,
  ChevronDown, Database, Activity, Clock, Target
} from 'lucide-react';
import { FormulaEditor } from '../components/FormulaEditor';
import { ResultPanel } from '../components/ResultPanel';

type ViewMode = 'compare' | 'diff' | 'replay';

export function AnalyticalCompareStudio() {
  const [viewMode, setViewMode] = useState<ViewMode>('compare');
  const [xrayOpen, setXrayOpen] = useState(true);

  return (
    <div className="h-screen flex flex-col bg-[#faf8f3]">
      {/* Top Bar with Analytical Theme */}
      <header className="h-16 border-b border-[#c65d47]/20 bg-gradient-to-r from-[#f5f1e8] to-[#e8dcc8] flex items-center justify-between px-6">
        <div className="flex items-center gap-4">
          <Link to="/" className="flex items-center gap-2 text-[#c65d47] hover:text-[#d97d67] transition-colors">
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm font-medium">Back</span>
          </Link>
          <div className="h-8 w-px bg-[#2a2a2a]/10" />
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded bg-gradient-to-br from-[#c65d47] to-[#d69f4c] flex items-center justify-center">
              <Target className="w-5 h-5 text-white" />
            </div>
            <div>
              <h1 className="text-base font-semibold text-[#2a2a2a]">DNA OneCalc</h1>
              <p className="text-xs text-[#6a6a6a]">Analytical Compare Studio</p>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <div className="px-3 py-1.5 bg-white/60 rounded-lg border border-[#2a2a2a]/10">
            <div className="text-xs text-[#6a6a6a]">Active Comparison</div>
            <div className="text-sm font-medium text-[#c65d47]">DNA vs Excel</div>
          </div>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Compact Left Rail */}
        <aside className="w-16 border-r border-[#2a2a2a]/10 bg-[#efebe1] flex flex-col items-center py-4 gap-4">
          <button className="w-10 h-10 rounded-lg bg-[#c65d47] text-white flex items-center justify-center hover:bg-[#d97d67] transition-colors">
            <Plus className="w-5 h-5" />
          </button>
          <div className="w-10 h-px bg-[#2a2a2a]/10" />
          <button className="w-10 h-10 rounded-lg bg-[#f5f1e8] text-[#c65d47] flex items-center justify-center border-2 border-[#c65d47]">
            <FileText className="w-5 h-5" />
          </button>
          <button className="w-10 h-10 rounded-lg hover:bg-[#f5f1e8] text-[#6a6a6a] flex items-center justify-center transition-colors">
            <FileText className="w-5 h-5" />
          </button>
          <button className="w-10 h-10 rounded-lg hover:bg-[#f5f1e8] text-[#6a6a6a] flex items-center justify-center transition-colors">
            <FileText className="w-5 h-5" />
          </button>
        </aside>

        {/* Main Content with Tab Bar */}
        <main className="flex-1 flex flex-col overflow-hidden">
          {/* Integrated Tab and Mode Bar */}
          <div className="border-b border-[#2a2a2a]/10 bg-[#f5f1e8]">
            <div className="flex items-center justify-between px-4 h-12 border-b border-[#2a2a2a]/10">
              <div className="flex items-center gap-2">
                <div className="flex items-center gap-2 px-3 py-1.5 bg-[#faf8f3] rounded-lg">
                  <Circle className="w-2 h-2 fill-[#c65d47] text-[#c65d47]" />
                  <FileText className="w-4 h-4 text-[#c65d47]" />
                  <span className="text-sm text-[#2a2a2a]">Comparison Analysis #247</span>
                  <Dot className="w-3 h-3 text-[#d69f4c]" />
                  <button className="ml-1 p-0.5 hover:bg-[#e8e4da] rounded">
                    <X className="w-3 h-3 text-[#6a6a6a]" />
                  </button>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <button className="px-3 py-1.5 text-sm bg-[#c65d47] text-white rounded-lg hover:bg-[#d97d67] transition-colors flex items-center gap-2">
                  <Play className="w-3.5 h-3.5" />
                  Run Comparison
                </button>
                <button
                  onClick={() => setXrayOpen(!xrayOpen)}
                  className={`px-3 py-1.5 text-sm rounded-lg transition-colors flex items-center gap-2 ${
                    xrayOpen
                      ? 'bg-[#c65d47] text-white'
                      : 'border border-[#2a2a2a]/20 text-[#2a2a2a] hover:bg-white/50'
                  }`}
                >
                  <Code className="w-3.5 h-3.5" />
                  X-Ray
                </button>
              </div>
            </div>
            
            <div className="flex items-center px-4 h-10 gap-1">
              <button
                onClick={() => setViewMode('compare')}
                className={`px-4 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                  viewMode === 'compare'
                    ? 'bg-[#c65d47] text-white'
                    : 'text-[#6a6a6a] hover:bg-white/50 hover:text-[#2a2a2a]'
                }`}
              >
                <GitCompare className="w-4 h-4 inline mr-2" />
                Compare View
              </button>
              <button
                onClick={() => setViewMode('diff')}
                className={`px-4 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                  viewMode === 'diff'
                    ? 'bg-[#c65d47] text-white'
                    : 'text-[#6a6a6a] hover:bg-white/50 hover:text-[#2a2a2a]'
                }`}
              >
                <Layers className="w-4 h-4 inline mr-2" />
                Diff Analysis
              </button>
              <button
                onClick={() => setViewMode('replay')}
                className={`px-4 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                  viewMode === 'replay'
                    ? 'bg-[#c65d47] text-white'
                    : 'text-[#6a6a6a] hover:bg-white/50 hover:text-[#2a2a2a]'
                }`}
              >
                <FlaskConical className="w-4 h-4 inline mr-2" />
                Replay Evidence
              </button>
            </div>
          </div>

          {/* Content Area */}
          <div className="flex-1 overflow-hidden flex">
            <div className={`flex-1 overflow-y-auto transition-all duration-300 ${xrayOpen ? 'mr-80' : ''}`}>
              {viewMode === 'compare' && (
                <div className="p-6">
                  {/* Comparison Header */}
                  <div className="mb-6 bg-gradient-to-r from-[#2d5f5d]/5 to-[#c65d47]/5 border border-[#c65d47]/20 rounded-xl p-6">
                    <div className="flex items-start justify-between mb-4">
                      <div>
                        <h2 className="text-2xl font-semibold text-[#2a2a2a] mb-2">Twin Oracle Comparison</h2>
                        <p className="text-[#6a6a6a]">DNA OneCalc vs Excel Observed Evidence</p>
                      </div>
                      <div className="flex items-center gap-2 px-4 py-2 bg-[#2d5f5d] text-white rounded-lg">
                        <CheckCircle2 className="w-5 h-5" />
                        <span className="font-medium">Match</span>
                      </div>
                    </div>
                    <div className="grid grid-cols-4 gap-4 text-sm">
                      <div>
                        <div className="text-[#6a6a6a] mb-1">Reliability</div>
                        <div className="text-[#2a2a2a] font-medium">High</div>
                      </div>
                      <div>
                        <div className="text-[#6a6a6a] mb-1">Envelope</div>
                        <div className="text-[#2a2a2a] font-medium">Full</div>
                      </div>
                      <div>
                        <div className="text-[#6a6a6a] mb-1">Platform</div>
                        <div className="text-[#2a2a2a] font-medium">Windows</div>
                      </div>
                      <div>
                        <div className="text-[#6a6a6a] mb-1">Run Time</div>
                        <div className="text-[#2a2a2a] font-medium">2026-04-03 14:32</div>
                      </div>
                    </div>
                  </div>

                  {/* Side-by-Side Comparison */}
                  <div className="grid grid-cols-2 gap-6 mb-6">
                    {/* DNA OneCalc Side */}
                    <div className="space-y-4">
                      <div className="flex items-center gap-3 mb-3">
                        <div className="w-10 h-10 rounded-lg bg-[#2d5f5d] flex items-center justify-center">
                          <Activity className="w-5 h-5 text-white" />
                        </div>
                        <div>
                          <h3 className="font-semibold text-[#2a2a2a]">DNA OneCalc</h3>
                          <p className="text-xs text-[#6a6a6a]">OxFml v0.12.4 • OxFunc v0.8.2</p>
                        </div>
                      </div>
                      
                      <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-4">
                        <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2">Formula</div>
                        <div className="font-mono text-sm text-[#2a2a2a] bg-white/50 rounded p-3">
                          =LET(values, SEQUENCE(10,1,1,1), filtered, FILTER(values, MOD(values,2)=0), SUM(filtered))
                        </div>
                      </div>

                      <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-4">
                        <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-3">Result</div>
                        <div className="bg-white/50 rounded-lg p-6 text-center">
                          <div className="font-mono text-4xl text-[#2d5f5d] mb-2">30</div>
                          <div className="text-sm text-[#6a6a6a]">Number • Scalar</div>
                        </div>
                      </div>

                      <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-4">
                        <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2">Type Info</div>
                        <div className="space-y-2 text-sm">
                          <div className="flex justify-between">
                            <span className="text-[#6a6a6a]">Type</span>
                            <span className="text-[#2a2a2a] font-mono">Number</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#6a6a6a]">Shape</span>
                            <span className="text-[#2a2a2a] font-mono">Scalar</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#6a6a6a]">Format</span>
                            <span className="text-[#2a2a2a] font-mono">General</span>
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Excel Observed Side */}
                    <div className="space-y-4">
                      <div className="flex items-center gap-3 mb-3">
                        <div className="w-10 h-10 rounded-lg bg-[#c65d47] flex items-center justify-center">
                          <Database className="w-5 h-5 text-white" />
                        </div>
                        <div>
                          <h3 className="font-semibold text-[#2a2a2a]">Excel Observed</h3>
                          <p className="text-xs text-[#6a6a6a]">Excel 365 • OxXlPlay v0.4.1</p>
                        </div>
                      </div>
                      
                      <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-4">
                        <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2">Formula</div>
                        <div className="font-mono text-sm text-[#2a2a2a] bg-white/50 rounded p-3">
                          =LET(values, SEQUENCE(10,1,1,1), filtered, FILTER(values, MOD(values,2)=0), SUM(filtered))
                        </div>
                      </div>

                      <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-4">
                        <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-3">Result</div>
                        <div className="bg-white/50 rounded-lg p-6 text-center">
                          <div className="font-mono text-4xl text-[#c65d47] mb-2">30</div>
                          <div className="text-sm text-[#6a6a6a]">Number • Scalar</div>
                        </div>
                      </div>

                      <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-4">
                        <div className="text-xs font-medium text-[#6a6a6a] uppercase tracking-wider mb-2">Type Info</div>
                        <div className="space-y-2 text-sm">
                          <div className="flex justify-between">
                            <span className="text-[#6a6a6a]">Type</span>
                            <span className="text-[#2a2a2a] font-mono">Number</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#6a6a6a]">Shape</span>
                            <span className="text-[#2a2a2a] font-mono">Scalar</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#6a6a6a]">Format</span>
                            <span className="text-[#2a2a2a] font-mono">General</span>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Comparison Matrix */}
                  <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                    <h3 className="font-semibold text-[#2a2a2a] mb-4">Comparison Dimensions</h3>
                    <div className="space-y-3">
                      <div className="flex items-center justify-between p-3 bg-white/50 rounded-lg">
                        <div className="flex items-center gap-3">
                          <CheckCircle2 className="w-5 h-5 text-[#2d5f5d]" />
                          <div>
                            <div className="text-sm font-medium text-[#2a2a2a]">Value Agreement</div>
                            <div className="text-xs text-[#6a6a6a]">30 = 30</div>
                          </div>
                        </div>
                        <div className="px-3 py-1 bg-[#2d5f5d] text-white text-xs rounded-full">Match</div>
                      </div>
                      <div className="flex items-center justify-between p-3 bg-white/50 rounded-lg">
                        <div className="flex items-center gap-3">
                          <CheckCircle2 className="w-5 h-5 text-[#2d5f5d]" />
                          <div>
                            <div className="text-sm font-medium text-[#2a2a2a]">Type Agreement</div>
                            <div className="text-xs text-[#6a6a6a]">Number = Number</div>
                          </div>
                        </div>
                        <div className="px-3 py-1 bg-[#2d5f5d] text-white text-xs rounded-full">Match</div>
                      </div>
                      <div className="flex items-center justify-between p-3 bg-white/50 rounded-lg">
                        <div className="flex items-center gap-3">
                          <CheckCircle2 className="w-5 h-5 text-[#2d5f5d]" />
                          <div>
                            <div className="text-sm font-medium text-[#2a2a2a]">Display Agreement</div>
                            <div className="text-xs text-[#6a6a6a]">Formatting matches</div>
                          </div>
                        </div>
                        <div className="px-3 py-1 bg-[#2d5f5d] text-white text-xs rounded-full">Match</div>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {viewMode === 'diff' && (
                <div className="p-6">
                  <h2 className="text-2xl font-semibold text-[#2a2a2a] mb-6">Diff Analysis</h2>
                  
                  <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6 mb-6">
                    <h3 className="font-medium text-[#2a2a2a] mb-4">Semantic Differences</h3>
                    <div className="bg-[#2d5f5d]/5 border border-[#2d5f5d]/20 rounded-lg p-4 text-center">
                      <CheckCircle2 className="w-12 h-12 text-[#2d5f5d] mx-auto mb-3" />
                      <p className="text-lg font-medium text-[#2a2a2a] mb-1">No Differences Detected</p>
                      <p className="text-sm text-[#6a6a6a]">DNA and Excel produce identical results</p>
                    </div>
                  </div>

                  <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                    <h3 className="font-medium text-[#2a2a2a] mb-4">Trace Comparison</h3>
                    <div className="space-y-2 font-mono text-sm">
                      <div className="flex items-center gap-3 p-2">
                        <div className="w-4 h-4 rounded bg-[#2d5f5d]" />
                        <span className="text-[#6a6a6a]">Step 1:</span>
                        <span className="text-[#2a2a2a]">SEQUENCE(10,1,1,1) → [1,2,3..10]</span>
                        <span className="ml-auto text-[#2d5f5d]">✓</span>
                      </div>
                      <div className="flex items-center gap-3 p-2">
                        <div className="w-4 h-4 rounded bg-[#2d5f5d]" />
                        <span className="text-[#6a6a6a]">Step 2:</span>
                        <span className="text-[#2a2a2a]">FILTER with MOD → [2,4,6,8,10]</span>
                        <span className="ml-auto text-[#2d5f5d]">✓</span>
                      </div>
                      <div className="flex items-center gap-3 p-2">
                        <div className="w-4 h-4 rounded bg-[#2d5f5d]" />
                        <span className="text-[#6a6a6a]">Step 3:</span>
                        <span className="text-[#2a2a2a]">SUM → 30</span>
                        <span className="ml-auto text-[#2d5f5d]">✓</span>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {viewMode === 'replay' && (
                <div className="p-6">
                  <h2 className="text-2xl font-semibold text-[#2a2a2a] mb-6">Replay Evidence</h2>
                  
                  <div className="space-y-4">
                    <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                      <h3 className="font-medium text-[#2a2a2a] mb-4">Evidence Bundle</h3>
                      <div className="grid grid-cols-3 gap-4 mb-4">
                        <div className="bg-white/50 rounded-lg p-4">
                          <div className="text-xs text-[#6a6a6a] mb-1">Scenario ID</div>
                          <div className="font-mono text-sm text-[#2a2a2a]">scn_20260403_247</div>
                        </div>
                        <div className="bg-white/50 rounded-lg p-4">
                          <div className="text-xs text-[#6a6a6a] mb-1">Run ID</div>
                          <div className="font-mono text-sm text-[#2a2a2a]">run_247_001</div>
                        </div>
                        <div className="bg-white/50 rounded-lg p-4">
                          <div className="text-xs text-[#6a6a6a] mb-1">Comparison ID</div>
                          <div className="font-mono text-sm text-[#2a2a2a]">cmp_247_xl365</div>
                        </div>
                      </div>
                      <div className="flex gap-3">
                        <button className="flex-1 px-4 py-2 bg-[#c65d47] text-white rounded-lg hover:bg-[#d97d67] transition-colors">
                          Export Evidence Bundle
                        </button>
                        <button className="flex-1 px-4 py-2 border border-[#2a2a2a]/20 text-[#2a2a2a] rounded-lg hover:bg-white/50 transition-colors">
                          Generate Handoff Packet
                        </button>
                      </div>
                    </div>

                    <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
                      <h3 className="font-medium text-[#2a2a2a] mb-4">Replay Lineage</h3>
                      <div className="space-y-3">
                        <div className="flex items-start gap-3">
                          <Clock className="w-5 h-5 text-[#c65d47] mt-0.5" />
                          <div className="flex-1">
                            <div className="text-sm font-medium text-[#2a2a2a]">Initial Run</div>
                            <div className="text-xs text-[#6a6a6a] mt-1">2026-04-03 14:32:15 • DNA OneCalc v0.1.0</div>
                          </div>
                        </div>
                        <div className="flex items-start gap-3 pl-8">
                          <ChevronDown className="w-5 h-5 text-[#6a6a6a] mt-0.5" />
                          <div className="flex-1">
                            <div className="text-sm font-medium text-[#2a2a2a]">Excel Observation</div>
                            <div className="text-xs text-[#6a6a6a] mt-1">2026-04-03 14:32:18 • OxXlPlay v0.4.1 on Windows 11</div>
                          </div>
                        </div>
                        <div className="flex items-start gap-3 pl-8">
                          <ChevronDown className="w-5 h-5 text-[#6a6a6a] mt-0.5" />
                          <div className="flex-1">
                            <div className="text-sm font-medium text-[#2a2a2a]">Comparison Generated</div>
                            <div className="text-xs text-[#6a6a6a] mt-1">2026-04-03 14:32:20 • Full envelope • High reliability</div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* X-Ray Drawer */}
            {xrayOpen && (
              <aside className="w-80 border-l border-[#2a2a2a]/10 bg-[#f5f1e8] overflow-y-auto">
                <div className="sticky top-0 bg-gradient-to-b from-[#f5f1e8] to-[#e8dcc8] border-b border-[#c65d47]/20 p-4">
                  <div className="flex items-center justify-between mb-3">
                    <h3 className="font-semibold text-[#2a2a2a]">Semantic X-Ray</h3>
                    <button
                      onClick={() => setXrayOpen(false)}
                      className="p-1.5 hover:bg-[#e8e4da] rounded transition-colors"
                    >
                      <X className="w-4 h-4 text-[#6a6a6a]" />
                    </button>
                  </div>
                  <p className="text-xs text-[#6a6a6a]">Structured inspection lens over active scenario</p>
                </div>
                
                <div className="p-4 space-y-4">
                  <div className="bg-white/60 rounded-lg p-4 border border-[#c65d47]/10">
                    <h4 className="text-sm font-semibold text-[#2a2a2a] mb-3 flex items-center gap-2">
                      <Code className="w-4 h-4 text-[#c65d47]" />
                      Parse Context
                    </h4>
                    <div className="space-y-2 text-xs">
                      <div className="flex justify-between items-center">
                        <span className="text-[#6a6a6a]">Status</span>
                        <span className="px-2 py-0.5 bg-[#2d5f5d] text-white rounded">Valid</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Tokens</span>
                        <span className="text-[#2a2a2a] font-mono">24</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Functions</span>
                        <span className="text-[#2a2a2a] font-mono">4</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Depth</span>
                        <span className="text-[#2a2a2a] font-mono">3</span>
                      </div>
                    </div>
                  </div>

                  <div className="bg-white/60 rounded-lg p-4 border border-[#c65d47]/10">
                    <h4 className="text-sm font-semibold text-[#2a2a2a] mb-3 flex items-center gap-2">
                      <Layers className="w-4 h-4 text-[#c65d47]" />
                      Bind Context
                    </h4>
                    <div className="space-y-2 text-xs">
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Variables</span>
                        <span className="text-[#2a2a2a] font-mono">2</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">References</span>
                        <span className="text-[#2a2a2a] font-mono">0</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Scope Depth</span>
                        <span className="text-[#2a2a2a] font-mono">1</span>
                      </div>
                    </div>
                  </div>

                  <div className="bg-white/60 rounded-lg p-4 border border-[#c65d47]/10">
                    <h4 className="text-sm font-semibold text-[#2a2a2a] mb-3 flex items-center gap-2">
                      <Activity className="w-4 h-4 text-[#c65d47]" />
                      Eval Context
                    </h4>
                    <div className="space-y-2 text-xs">
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Steps</span>
                        <span className="text-[#2a2a2a] font-mono">7</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Duration</span>
                        <span className="text-[#2a2a2a] font-mono">1.2ms</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-[#6a6a6a]">Memory</span>
                        <span className="text-[#2a2a2a] font-mono">124 bytes</span>
                      </div>
                    </div>
                  </div>

                  <div className="bg-white/60 rounded-lg p-4 border border-[#c65d47]/10">
                    <h4 className="text-sm font-semibold text-[#2a2a2a] mb-3 flex items-center gap-2">
                      <Target className="w-4 h-4 text-[#c65d47]" />
                      Provenance
                    </h4>
                    <div className="space-y-2 text-xs">
                      <div>
                        <div className="text-[#6a6a6a] mb-1">Host Profile</div>
                        <div className="text-[#2a2a2a] font-mono">H1-Standard</div>
                      </div>
                      <div>
                        <div className="text-[#6a6a6a] mb-1">Capability Floor</div>
                        <div className="text-[#2d5f5d]">Full</div>
                      </div>
                      <div>
                        <div className="text-[#6a6a6a] mb-1">Platform</div>
                        <div className="text-[#2a2a2a]">Windows/Browser</div>
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
