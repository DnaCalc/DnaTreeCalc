import { useState } from 'react';
import { Link } from 'react-router';
import { 
  Home, FileText, Clock, Star, Circle, Dot, X, Plus,
  ChevronRight, Play, Search, Settings, HelpCircle,
  Code, Activity, FlaskConical, ArrowLeft, Database,
  Target, Layers, GitCompare, Package, Cpu, Zap
} from 'lucide-react';
import { PremiumFormulaEditor } from '../components/PremiumFormulaEditor';
import { FormulaWalkInspector } from '../components/FormulaWalkInspector';

type ViewMode = 'explorer' | 'workbench';

export function RefinedWarmEditorial() {
  const [viewMode, setViewMode] = useState<ViewMode>('explorer');
  const [inspectorOpen, setInspectorOpen] = useState(false);

  return (
    <div className="h-screen flex flex-col bg-[#faf7f1]">
      {/* Top Bar */}
      <header className="h-14 border-b border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-4 shadow-sm">
        <div className="flex items-center gap-4">
          <Link to="/" className="flex items-center gap-2 text-[#1e4d4a] hover:text-[#2d6864] transition-colors">
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm font-medium">Back</span>
          </Link>
          <div className="h-6 w-px bg-[#1f1c17]/10" />
          <h1 className="text-lg font-semibold text-[#1f1c17]">DNA OneCalc</h1>
          <span className="text-sm text-[#7a7568]">Refined Edition</span>
        </div>
        <div className="flex items-center gap-3">
          <button className="p-2 hover:bg-[#f7f3ea] rounded transition-colors">
            <Search className="w-4 h-4 text-[#7a7568]" />
          </button>
          <button className="p-2 hover:bg-[#f7f3ea] rounded transition-colors">
            <HelpCircle className="w-4 h-4 text-[#7a7568]" />
          </button>
          <button className="p-2 hover:bg-[#f7f3ea] rounded transition-colors">
            <Settings className="w-4 h-4 text-[#7a7568]" />
          </button>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Left Rail */}
        <aside className="w-64 border-r border-[#1f1c17]/10 bg-[#ede7da] flex flex-col">
          <div className="p-4 border-b border-[#1f1c17]/10">
            <button className="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-[#1e4d4a] text-white rounded-lg hover:bg-[#2d6864] transition-all shadow-sm hover:shadow">
              <Plus className="w-4 h-4" />
              <span className="text-sm font-medium">New Formula Space</span>
            </button>
          </div>

          <nav className="flex-1 overflow-y-auto p-4">
            <div className="mb-6">
              <div className="text-xs font-semibold text-[#7a7568] uppercase tracking-wider mb-3 px-2">
                Quick Access
              </div>
              <div className="space-y-1">
                <button className="w-full flex items-center gap-3 px-3 py-2 text-sm text-[#1f1c17] hover:bg-[#f7f3ea] rounded-lg transition-colors">
                  <Home className="w-4 h-4 text-[#1e4d4a]" />
                  <span>Overview</span>
                </button>
                <button className="w-full flex items-center gap-3 px-3 py-2 text-sm text-[#1f1c17] hover:bg-[#f7f3ea] rounded-lg transition-colors">
                  <Clock className="w-4 h-4 text-[#c88d2e]" />
                  <span>Recent</span>
                </button>
                <button className="w-full flex items-center gap-3 px-3 py-2 text-sm text-[#1f1c17] hover:bg-[#f7f3ea] rounded-lg transition-colors">
                  <Star className="w-4 h-4 text-[#b84532]" />
                  <span>Pinned</span>
                </button>
              </div>
            </div>

            <div>
              <div className="text-xs font-semibold text-[#7a7568] uppercase tracking-wider mb-3 px-2">
                Formula Spaces
              </div>
              <div className="space-y-1">
                <button className="w-full flex items-center gap-2 px-3 py-2.5 text-sm bg-[#f7f3ea] text-[#1f1c17] rounded-lg border-l-4 border-[#1e4d4a]">
                  <FileText className="w-4 h-4 text-[#1e4d4a]" />
                  <span className="flex-1 text-left truncate font-medium">LET Formula Analysis</span>
                  <Dot className="w-3 h-3 text-[#c88d2e]" />
                </button>
                <button className="w-full flex items-center gap-2 px-3 py-2.5 text-sm text-[#7a7568] hover:bg-[#f7f3ea] rounded-lg transition-colors border-l-4 border-transparent">
                  <FileText className="w-4 h-4" />
                  <span className="flex-1 text-left truncate">FILTER Examples</span>
                </button>
                <button className="w-full flex items-center gap-2 px-3 py-2.5 text-sm text-[#7a7568] hover:bg-[#f7f3ea] rounded-lg transition-colors border-l-4 border-transparent">
                  <FileText className="w-4 h-4" />
                  <span className="flex-1 text-left truncate">Array Operations</span>
                </button>
              </div>
            </div>
          </nav>

          <div className="p-4 border-t border-[#1f1c17]/10">
            <div className="bg-gradient-to-br from-[#1e4d4a]/10 to-[#3e5238]/10 border border-[#1e4d4a]/30 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-2">
                <div className="w-2 h-2 rounded-full bg-[#1e4d4a]" />
                <div className="text-xs font-semibold text-[#1f1c17]">All Modes Available</div>
              </div>
              <div className="text-xs text-[#7a7568] space-y-0.5">
                <div className="flex items-center gap-2">
                  <span className="text-[#3e5238]">Profile:</span>
                  <span className="font-mono text-[#1f1c17]">H1-Standard</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-[#3e5238]">Floor:</span>
                  <span className="font-mono text-[#1f1c17]">Full</span>
                </div>
              </div>
            </div>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 flex flex-col overflow-hidden">
          {/* Tab Bar */}
          <div className="h-12 border-b border-[#1f1c17]/10 bg-[#f7f3ea] flex items-center px-3 gap-1.5">
            <div className="flex items-center gap-2 px-3 py-2 bg-white border-l-4 border-[#1e4d4a] rounded-lg shadow-sm">
              <FileText className="w-4 h-4 text-[#1e4d4a]" />
              <span className="text-sm text-[#1f1c17] font-medium">LET Formula Analysis</span>
              <Dot className="w-3 h-3 text-[#c88d2e] ml-1" />
              <button className="ml-2 p-0.5 hover:bg-[#e8e2d4] rounded transition-colors">
                <X className="w-3.5 h-3.5 text-[#7a7568]" />
              </button>
            </div>
            <div className="flex items-center gap-2 px-3 py-2 hover:bg-[#e8e2d4] rounded-lg transition-colors cursor-pointer">
              <FileText className="w-4 h-4 text-[#7a7568]" />
              <span className="text-sm text-[#7a7568]">FILTER Examples</span>
            </div>
            <button className="p-2 hover:bg-[#e8e2d4] rounded transition-colors">
              <Plus className="w-3.5 h-3.5 text-[#7a7568]" />
            </button>
          </div>

          {/* View Switcher */}
          <div className="h-11 border-b border-[#1f1c17]/10 bg-gradient-to-r from-[#faf7f1] to-[#f7f3ea] flex items-center px-6 gap-6">
            <button
              onClick={() => setViewMode('explorer')}
              className={`text-sm font-medium pb-3 border-b-2 transition-colors ${
                viewMode === 'explorer'
                  ? 'border-[#1e4d4a] text-[#1e4d4a]'
                  : 'border-transparent text-[#7a7568] hover:text-[#1f1c17]'
              }`}
            >
              Formula Explorer
            </button>
            <button
              onClick={() => setViewMode('workbench')}
              className={`text-sm font-medium pb-3 border-b-2 transition-colors ${
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
                <div className="p-6 max-w-7xl mx-auto">
                  <div className="grid grid-cols-3 gap-6">
                    {/* Left Column: Editor + Result */}
                    <div className="col-span-2 space-y-6">
                      {/* Premium Formula Editor */}
                      <PremiumFormulaEditor onEvaluate={() => setInspectorOpen(!inspectorOpen)} />

                      {/* Result Panel */}
                      <div className="space-y-3">
                        <div className="flex items-center justify-between">
                          <h3 className="text-base font-semibold text-[#1f1c17]">Result</h3>
                          <div className="flex items-center gap-2 text-sm text-[#1e4d4a]">
                            <div className="w-2 h-2 rounded-full bg-[#1e4d4a]" />
                            <span className="font-medium">Evaluated</span>
                          </div>
                        </div>
                        
                        <div className="bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10 border-2 border-[#1e4d4a]/20 rounded-xl p-8">
                          <div className="font-mono text-5xl text-[#1e4d4a] mb-3 font-semibold">
                            30
                          </div>
                          <div className="text-sm text-[#7a7568] font-medium">
                            Number • Scalar
                          </div>
                        </div>

                        <div className="grid grid-cols-2 gap-3">
                          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                            <div className="text-xs text-[#7a7568] mb-1">Effective Display</div>
                            <div className="text-lg font-semibold text-[#1f1c17] font-mono">30</div>
                          </div>
                          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                            <div className="text-xs text-[#7a7568] mb-1">Type</div>
                            <div className="text-lg font-semibold text-[#c88d2e] font-mono">Number</div>
                          </div>
                          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                            <div className="text-xs text-[#7a7568] mb-1">Shape</div>
                            <div className="text-lg font-semibold text-[#3e5238] font-mono">Scalar</div>
                          </div>
                          <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                            <div className="text-xs text-[#7a7568] mb-1">Format</div>
                            <div className="text-lg font-semibold text-[#b84532] font-mono">General</div>
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Right Column: Formula Walk + Help */}
                    <div className="space-y-6">
                      {/* Formula Walk - Always Visible */}
                      <div className="bg-white border-2 border-[#3e5238]/20 rounded-xl p-4">
                        <div className="flex items-center justify-between mb-3">
                          <h3 className="text-sm font-semibold text-[#1f1c17] flex items-center gap-2">
                            <Activity className="w-4 h-4 text-[#3e5238]" />
                            Formula Walk
                          </h3>
                          <button
                            onClick={() => setInspectorOpen(true)}
                            className="text-xs text-[#1e4d4a] hover:text-[#2d6864] font-medium"
                          >
                            Full Inspector →
                          </button>
                        </div>
                        <FormulaWalkInspector />
                      </div>

                      {/* Function Help Panel */}
                      <div className="bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border-2 border-[#c88d2e]/20 rounded-xl p-5 space-y-4">
                        <h3 className="text-sm font-semibold text-[#1f1c17] flex items-center gap-2">
                          <HelpCircle className="w-4 h-4 text-[#c88d2e]" />
                          Function Help
                        </h3>
                        
                        <div>
                          <div className="font-mono text-sm text-[#1e4d4a] font-semibold mb-2">
                            LET(name1, value1, [...], calc)
                          </div>
                          <div className="text-sm text-[#7a7568] leading-relaxed">
                            Assigns names to calculation results to allow storing intermediate calculations.
                          </div>
                        </div>
                        
                        <div className="grid grid-cols-2 gap-3 pt-3 border-t border-[#1f1c17]/10">
                          <div className="bg-white/60 rounded-lg p-3">
                            <div className="text-xs font-medium text-[#7a7568] mb-1.5">Category</div>
                            <div className="text-sm text-[#1f1c17] font-medium">Logical</div>
                          </div>
                          <div className="bg-white/60 rounded-lg p-3">
                            <div className="text-xs font-medium text-[#7a7568] mb-1.5">Status</div>
                            <div className="inline-flex items-center gap-1.5 px-2 py-1 bg-[#1e4d4a] text-white text-xs font-medium rounded">
                              <div className="w-1.5 h-1.5 rounded-full bg-white" />
                              Supported
                            </div>
                          </div>
                        </div>

                        <div className="pt-3 border-t border-[#1f1c17]/10">
                          <div className="text-xs font-medium text-[#7a7568] mb-2">Arguments</div>
                          <div className="space-y-2 text-sm">
                            <div className="flex items-start gap-2">
                              <span className="font-mono text-[#1e4d4a] font-semibold">name1</span>
                              <span className="text-[#7a7568]">– Variable name</span>
                            </div>
                            <div className="flex items-start gap-2">
                              <span className="font-mono text-[#c88d2e] font-semibold">value1</span>
                              <span className="text-[#7a7568]">– Value to assign</span>
                            </div>
                            <div className="flex items-start gap-2">
                              <span className="font-mono text-[#b84532] font-semibold">calc</span>
                              <span className="text-[#7a7568]">– Final expression</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {viewMode === 'workbench' && (
                <div className="p-6 max-w-6xl mx-auto">
                  <div className="mb-6">
                    <h2 className="text-xl font-semibold text-[#1f1c17] mb-2">Evidence Workbench</h2>
                    <p className="text-sm text-[#7a7568]">
                      Twin Oracle comparison with replay lineage and evidence bundle management
                    </p>
                  </div>
                  
                  <div className="space-y-6">
                    {/* Comparison Header */}
                    <div className="bg-gradient-to-br from-[#1e4d4a]/5 via-[#1e4d4a]/10 to-[#3e5238]/5 border-2 border-[#1e4d4a]/20 rounded-xl p-6">
                      <div className="flex items-start justify-between mb-5">
                        <div>
                          <h3 className="text-lg font-semibold text-[#1f1c17] mb-1">Twin Oracle Comparison</h3>
                          <p className="text-sm text-[#7a7568]">DNA OneCalc vs Excel Observed Evidence</p>
                        </div>
                        <div className="flex items-center gap-2.5 px-4 py-2 bg-[#1e4d4a] text-white rounded-lg shadow-sm">
                          <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                          </svg>
                          <span className="font-semibold">Match</span>
                        </div>
                      </div>
                      
                      <div className="grid grid-cols-4 gap-4 text-sm">
                        <div className="bg-white/60 rounded-lg p-3">
                          <div className="text-[#7a7568] mb-1">Reliability</div>
                          <div className="text-[#1f1c17] font-semibold">High</div>
                        </div>
                        <div className="bg-white/60 rounded-lg p-3">
                          <div className="text-[#7a7568] mb-1">Envelope</div>
                          <div className="text-[#1f1c17] font-semibold">Full</div>
                        </div>
                        <div className="bg-white/60 rounded-lg p-3">
                          <div className="text-[#7a7568] mb-1">Platform</div>
                          <div className="text-[#1f1c17] font-semibold">Windows</div>
                        </div>
                        <div className="bg-white/60 rounded-lg p-3">
                          <div className="text-[#7a7568] mb-1">Timestamp</div>
                          <div className="text-[#1f1c17] font-mono text-xs">2026-04-03 14:32</div>
                        </div>
                      </div>
                    </div>

                    {/* Evidence Lineage */}
                    <div className="bg-white border border-[#1f1c17]/15 rounded-xl p-6">
                      <h3 className="font-semibold text-[#1f1c17] mb-4 flex items-center gap-2">
                        <GitCompare className="w-5 h-5 text-[#3e5238]" />
                        Evidence Lineage
                      </h3>
                      <div className="space-y-1">
                        <div className="flex items-center gap-3 p-3 bg-[#f7f3ea] rounded-lg">
                          <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                          <div className="flex-1">
                            <div className="text-sm font-medium text-[#1f1c17]">Scenario: LET Formula Analysis</div>
                            <div className="text-xs text-[#7a7568] font-mono">scn_20260403_247 • Created 2026-04-03 14:30</div>
                          </div>
                        </div>
                        <div className="flex items-center gap-3 p-3 pl-9 bg-[#f7f3ea] rounded-lg">
                          <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                          <div className="flex-1">
                            <div className="text-sm font-medium text-[#1f1c17]">DNA Run #247</div>
                            <div className="text-xs text-[#7a7568] font-mono">run_247_001 • OxFml v0.12.4 • OxFunc v0.8.2</div>
                          </div>
                          <div className="px-2 py-1 bg-[#1e4d4a] text-white text-xs font-medium rounded">30</div>
                        </div>
                        <div className="flex items-center gap-3 p-3 pl-9 bg-[#f7f3ea] rounded-lg">
                          <ChevronRight className="w-4 h-4 text-[#7a7568]" />
                          <div className="flex-1">
                            <div className="text-sm font-medium text-[#1f1c17]">Excel Observation</div>
                            <div className="text-xs text-[#7a7568] font-mono">obs_247_xl365 • OxXlPlay v0.4.1 • Excel 365 • Windows 11</div>
                          </div>
                          <div className="px-2 py-1 bg-[#b84532] text-white text-xs font-medium rounded">30</div>
                        </div>
                        <div className="flex items-center gap-3 p-3 pl-14 bg-[#1e4d4a]/5 border border-[#1e4d4a]/20 rounded-lg">
                          <FlaskConical className="w-4 h-4 text-[#1e4d4a]" />
                          <div className="flex-1">
                            <div className="text-sm font-medium text-[#1f1c17]">Comparison Generated</div>
                            <div className="text-xs text-[#7a7568] font-mono">cmp_247_001 • Full envelope • High reliability</div>
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Evidence Bundle */}
                    <div className="bg-white border border-[#1f1c17]/15 rounded-xl p-6">
                      <h3 className="font-semibold text-[#1f1c17] mb-4 flex items-center gap-2">
                        <Package className="w-5 h-5 text-[#c88d2e]" />
                        Evidence Bundle
                      </h3>
                      <div className="grid grid-cols-3 gap-4 mb-5">
                        <div className="bg-[#f7f3ea] rounded-lg p-4">
                          <div className="text-xs text-[#7a7568] mb-1">Scenario ID</div>
                          <div className="font-mono text-sm text-[#1f1c17] font-semibold">scn_247</div>
                        </div>
                        <div className="bg-[#f7f3ea] rounded-lg p-4">
                          <div className="text-xs text-[#7a7568] mb-1">Run ID</div>
                          <div className="font-mono text-sm text-[#1f1c17] font-semibold">run_247_001</div>
                        </div>
                        <div className="bg-[#f7f3ea] rounded-lg p-4">
                          <div className="text-xs text-[#7a7568] mb-1">Comparison ID</div>
                          <div className="font-mono text-sm text-[#1f1c17] font-semibold">cmp_247_xl365</div>
                        </div>
                      </div>
                      <div className="flex gap-3">
                        <button className="flex-1 px-4 py-3 bg-[#1e4d4a] text-white rounded-lg hover:bg-[#2d6864] transition-colors font-medium shadow-sm hover:shadow flex items-center justify-center gap-2">
                          <Package className="w-4 h-4" />
                          Retain as Evidence
                        </button>
                        <button className="flex-1 px-4 py-3 border-2 border-[#1e4d4a]/30 text-[#1e4d4a] rounded-lg hover:bg-[#1e4d4a]/5 transition-colors font-medium flex items-center justify-center gap-2">
                          <ChevronRight className="w-4 h-4" />
                          Export Handoff Packet
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* X-Ray Drawer */}
            {inspectorOpen && (
              <aside className="w-[420px] border-l border-[#1f1c17]/10 bg-[#f7f3ea] overflow-y-auto flex flex-col">
                <div className="sticky top-0 bg-[#ede7da] border-b border-[#1f1c17]/10 p-4 z-10">
                  <div className="flex items-center justify-between mb-4">
                    <h3 className="font-semibold text-[#1f1c17] flex items-center gap-2">
                      <Code className="w-5 h-5 text-[#1e4d4a]" />
                      X-Ray Inspector
                    </h3>
                    <button
                      onClick={() => setInspectorOpen(false)}
                      className="p-1.5 hover:bg-[#f7f3ea] rounded transition-colors"
                    >
                      <X className="w-4 h-4 text-[#7a7568]" />
                    </button>
                  </div>
                  
                  <div className="flex gap-1 p-1 bg-white rounded-lg border border-[#1f1c17]/10">
                    <button
                      onClick={() => setInspectorOpen(false)}
                      className={`flex-1 px-3 py-1.5 text-sm font-medium rounded transition-colors ${
                        !inspectorOpen
                          ? 'bg-[#1e4d4a] text-white'
                          : 'text-[#7a7568] hover:text-[#1f1c17]'
                      }`}
                    >
                      Overview
                    </button>
                    <button
                      onClick={() => setInspectorOpen(true)}
                      className={`flex-1 px-3 py-1.5 text-sm font-medium rounded transition-colors ${
                        inspectorOpen
                          ? 'bg-[#1e4d4a] text-white'
                          : 'text-[#7a7568] hover:text-[#1f1c17]'
                      }`}
                    >
                      Formula Walk
                    </button>
                  </div>
                </div>
                
                <div className="flex-1 p-4">
                  {inspectorOpen ? (
                    <FormulaWalkInspector />
                  ) : (
                    <div className="space-y-4">
                      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Parse Context</h4>
                        <div className="space-y-2 text-sm">
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Status</span>
                            <span className="px-2 py-0.5 bg-[#1e4d4a] text-white text-xs font-medium rounded">Valid</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Tokens</span>
                            <span className="text-[#1f1c17] font-mono">24</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Functions</span>
                            <span className="text-[#1f1c17] font-mono">4</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Depth</span>
                            <span className="text-[#1f1c17] font-mono">3</span>
                          </div>
                        </div>
                      </div>

                      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Bind Context</h4>
                        <div className="space-y-2 text-sm">
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Variables</span>
                            <span className="text-[#1f1c17] font-mono">2</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">References</span>
                            <span className="text-[#1f1c17] font-mono">0</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Scope Depth</span>
                            <span className="text-[#1f1c17] font-mono">1</span>
                          </div>
                        </div>
                      </div>

                      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Eval Context</h4>
                        <div className="space-y-2 text-sm">
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Steps</span>
                            <span className="text-[#1f1c17] font-mono">7</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Duration</span>
                            <span className="text-[#1f1c17] font-mono">1.2ms</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-[#7a7568]">Memory</span>
                            <span className="text-[#1f1c17] font-mono">124 bytes</span>
                          </div>
                        </div>
                      </div>

                      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
                        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Provenance</h4>
                        <div className="space-y-2.5 text-sm">
                          <div>
                            <div className="text-[#7a7568] mb-1">Host Profile</div>
                            <div className="text-[#1f1c17] font-mono">H1-Standard</div>
                          </div>
                          <div>
                            <div className="text-[#7a7568] mb-1">Capability Floor</div>
                            <div className="text-[#1e4d4a] font-medium">Full</div>
                          </div>
                          <div>
                            <div className="text-[#7a7568] mb-1">Platform</div>
                            <div className="text-[#1f1c17]">Windows/Browser</div>
                          </div>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </aside>
            )}
          </div>
        </main>
      </div>

      {/* Status Footer */}
      <footer className="h-8 border-t border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-6 text-xs">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-[#1e4d4a]" />
            <span className="font-medium text-[#1f1c17]">Ready</span>
          </div>
          <div className="flex items-center gap-1.5 text-[#7a7568]">
            <Cpu className="w-3.5 h-3.5" />
            <span>H1-Standard</span>
          </div>
          <div className="flex items-center gap-1.5 text-[#7a7568]">
            <Package className="w-3.5 h-3.5" />
            <span>OxFml v0.12.4</span>
          </div>
          <div className="flex items-center gap-1.5 text-[#7a7568]">
            <Database className="w-3.5 h-3.5" />
            <span>OxFunc v0.8.2</span>
          </div>
        </div>
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-1.5 text-[#7a7568]">
            <Zap className="w-3.5 h-3.5" />
            <span>1.2ms</span>
          </div>
          <div className="text-[#7a7568]">
            DNA OneCalc v0.1.0
          </div>
        </div>
      </footer>
    </div>
  );
}