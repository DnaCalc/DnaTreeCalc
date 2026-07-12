import { useState } from 'react';
import { Link } from 'react-router';
import { ArrowLeft, Search, Settings, HelpCircle } from 'lucide-react';
import { WorkspaceRail } from '../components/WorkspaceRail';
import { ExploreContextBar } from '../components/ExploreContextBar';
import { FormulaEditorPanel } from '../components/FormulaEditorPanel';
import { ResultPanel } from '../components/ResultPanel';
import { CompletionHelpPanel } from '../components/CompletionHelpPanel';
import { ExploreDrawer } from '../components/ExploreDrawer';

type DrawerType = 'formatting' | 'settings' | null;

export function FocusedExploreMode() {
  const [drawerOpen, setDrawerOpen] = useState<DrawerType>(null);

  const handleOpenDrawer = (drawer: DrawerType) => {
    setDrawerOpen(drawer);
  };

  const handleCloseDrawer = () => {
    setDrawerOpen(null);
  };

  const handleEvaluate = () => {
    // Evaluation logic
    console.log('Evaluating formula...');
  };

  return (
    <div className="h-screen flex flex-col bg-[#faf7f1]">
      {/* Global Top Bar */}
      <header className="h-14 border-b border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-4 shadow-sm z-20">
        <div className="flex items-center gap-4">
          <Link to="/" className="flex items-center gap-2 text-[#1e4d4a] hover:text-[#2d6864] transition-colors">
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm font-medium">Back</span>
          </Link>
          <div className="h-6 w-px bg-[#1f1c17]/10" />
          <h1 className="text-lg font-semibold text-[#1f1c17]">DNA OneCalc</h1>
          <span className="text-sm text-[#7a7568]">Focused Explore Mode</span>
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
        {/* Workspace Rail - Left */}
        <WorkspaceRail />

        {/* Main Content Area */}
        <main className="flex-1 flex flex-col overflow-hidden">
          {/* Formula Space Context Bar */}
          <ExploreContextBar onOpenDrawer={handleOpenDrawer} />

          {/* Content Area with 3-Column Layout */}
          <div className="flex-1 flex overflow-hidden relative">
            {/* Left: Formula Editor (40%) */}
            <div className={`flex flex-col border-r border-[#1f1c17]/10 bg-white transition-all duration-300 ${
              drawerOpen ? 'w-[38%]' : 'w-[40%]'
            }`}>
              <FormulaEditorPanel onEvaluate={handleEvaluate} />
            </div>

            {/* Center: Result + Array Preview (35%) */}
            <div className={`flex flex-col border-r border-[#1f1c17]/10 overflow-y-auto transition-all duration-300 ${
              drawerOpen ? 'w-[27%]' : 'w-[35%]'
            }`}>
              <ResultPanel onOpenFormatting={() => handleOpenDrawer('formatting')} />
            </div>

            {/* Right: Completion + Help (25%) */}
            <div className={`flex flex-col transition-all duration-300 ${
              drawerOpen ? 'w-[0%] overflow-hidden' : 'w-[25%]'
            }`}>
              <CompletionHelpPanel />
            </div>

            {/* Secondary Drawer (slides in from right) */}
            {drawerOpen && (
              <div className="w-[35%] border-l border-[#1f1c17]/10">
                <ExploreDrawer type={drawerOpen} onClose={handleCloseDrawer} />
              </div>
            )}
          </div>
        </main>
      </div>

      {/* Status Footer */}
      <footer className="h-8 border-t border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-6 text-xs z-20">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-[#1e4d4a]" />
            <span className="font-medium text-[#1f1c17]">Ready</span>
          </div>
          <div className="text-[#7a7568] font-mono">
            <span className="text-[#1e4d4a] font-semibold">OC-H0</span> • OxFml v0.12.4 • OxFunc v0.8.2
          </div>
        </div>
        <div className="flex items-center gap-6">
          <div className="text-[#7a7568]">
            Mode: <span className="text-[#1e4d4a] font-medium">Explore</span>
          </div>
          <div className="text-[#7a7568]">
            DNA OneCalc v0.1.0
          </div>
        </div>
      </footer>
    </div>
  );
}