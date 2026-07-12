import { useState } from 'react';
import { Link } from 'react-router';
import { ArrowLeft, Search, Settings, HelpCircle } from 'lucide-react';
import { WorkspaceRail } from '../components/WorkspaceRail';
import { InspectContextBar } from '../components/InspectContextBar';
import { SourceFormulaPanel } from '../components/SourceFormulaPanel';
import { FormulaWalkPanel } from '../components/FormulaWalkPanel';
import { InspectionSummaryPanel } from '../components/InspectionSummaryPanel';
import { InspectDrawer } from '../components/InspectDrawer';

type DrawerType = 'provenance' | 'context' | 'node' | null;

export function FocusedInspectMode() {
  const [drawerOpen, setDrawerOpen] = useState<DrawerType>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  const handleOpenDrawer = (drawer: DrawerType) => {
    setDrawerOpen(drawer);
  };

  const handleCloseDrawer = () => {
    setDrawerOpen(null);
  };

  const handleSelectNode = (nodeId: string) => {
    setSelectedNode(nodeId);
    setDrawerOpen('node');
  };

  const handleBackToExplore = () => {
    // Navigate to Explore mode
    console.log('Navigating to Explore mode...');
  };

  return (
    <div className="h-screen flex flex-col bg-[#faf7f1]">
      {/* Global Top Bar */}
      <header className="h-14 border-b border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-4 shadow-sm z-20">
        <div className="flex items-center gap-4">
          <Link to="/" className="flex items-center gap-2 text-[#3e5238] hover:text-[#566b4f] transition-colors">
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm font-medium">Back</span>
          </Link>
          <div className="h-6 w-px bg-[#1f1c17]/10" />
          <h1 className="text-lg font-semibold text-[#1f1c17]">DNA OneCalc</h1>
          <span className="text-sm text-[#7a7568]">Focused Inspect Mode</span>
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
          <InspectContextBar onOpenDrawer={handleOpenDrawer} />

          {/* Content Area with 3-Column Layout */}
          <div className="flex-1 flex overflow-hidden relative">
            {/* Left: Source Formula + Result (30%) */}
            <div className={`flex flex-col border-r border-[#1f1c17]/10 transition-all duration-300 ${
              drawerOpen ? 'w-[28%]' : 'w-[30%]'
            }`}>
              <SourceFormulaPanel onBackToExplore={handleBackToExplore} />
            </div>

            {/* Center: Formula Walk (45% or expand when drawer closed) */}
            <div className={`flex flex-col border-r border-[#1f1c17]/10 bg-white transition-all duration-300 ${
              drawerOpen ? 'w-[35%]' : 'w-[45%]'
            }`}>
              <FormulaWalkPanel onSelectNode={handleSelectNode} />
            </div>

            {/* Right: Inspection Summary (25% or hidden when drawer open) */}
            <div className={`flex flex-col transition-all duration-300 ${
              drawerOpen ? 'w-[0%] overflow-hidden' : 'w-[25%]'
            }`}>
              <InspectionSummaryPanel />
            </div>

            {/* Secondary Drawer (slides in from right) */}
            {drawerOpen && (
              <div className="w-[37%] border-l border-[#1f1c17]/10">
                <InspectDrawer type={drawerOpen} onClose={handleCloseDrawer} />
              </div>
            )}
          </div>
        </main>
      </div>

      {/* Status Footer */}
      <footer className="h-8 border-t border-[#1f1c17]/10 bg-[#ede7da] flex items-center justify-between px-6 text-xs z-20">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-[#3e5238]" />
            <span className="font-medium text-[#1f1c17]">Inspection Active</span>
          </div>
          <div className="text-[#7a7568] font-mono">
            <span className="text-[#3e5238] font-semibold">OC-H0</span> • OxFml v0.12.4 • OxFunc v0.8.2
          </div>
        </div>
        <div className="flex items-center gap-6">
          <div className="text-[#7a7568]">
            Mode: <span className="text-[#3e5238] font-medium">Inspect</span>
          </div>
          <div className="text-[#7a7568]">
            DNA OneCalc v0.1.0
          </div>
        </div>
      </footer>
    </div>
  );
}
