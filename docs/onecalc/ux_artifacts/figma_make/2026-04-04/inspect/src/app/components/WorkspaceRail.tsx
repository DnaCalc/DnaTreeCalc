import { useState } from 'react';
import { 
  Home, FileText, Clock, Star, Plus, Circle, Dot,
  ChevronDown, ChevronRight, Settings, Cpu, Package, Shield
} from 'lucide-react';

export function WorkspaceRail() {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['spaces']));

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expandedSections);
    if (newExpanded.has(section)) {
      newExpanded.delete(section);
    } else {
      newExpanded.add(section);
    }
    setExpandedSections(newExpanded);
  };

  return (
    <aside className="w-64 border-r border-[#1f1c17]/10 bg-[#ede7da] flex flex-col">
      {/* New Formula Space */}
      <div className="p-4 border-b border-[#1f1c17]/10">
        <button className="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-[#1e4d4a] text-white rounded-lg hover:bg-[#2d6864] transition-all shadow-sm">
          <Plus className="w-4 h-4" />
          <span className="text-sm font-medium">New Formula Space</span>
        </button>
      </div>

      {/* Workspace Navigation */}
      <nav className="flex-1 overflow-y-auto p-4">
        {/* Quick Access */}
        <div className="mb-6">
          <div className="text-xs font-semibold text-[#7a7568] uppercase tracking-wider mb-3 px-2">
            Workspace
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

        {/* Formula Spaces - Collapsible */}
        <div className="mb-6">
          <button
            onClick={() => toggleSection('spaces')}
            className="w-full flex items-center justify-between px-2 mb-2"
          >
            <div className="text-xs font-semibold text-[#7a7568] uppercase tracking-wider">
              Formula Spaces (3)
            </div>
            {expandedSections.has('spaces') ? (
              <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
            )}
          </button>
          {expandedSections.has('spaces') && (
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
          )}
        </div>

        {/* Extensions - Collapsible */}
        <div className="mb-6">
          <button
            onClick={() => toggleSection('extensions')}
            className="w-full flex items-center justify-between px-2 mb-2"
          >
            <div className="text-xs font-semibold text-[#7a7568] uppercase tracking-wider">
              Extensions
            </div>
            {expandedSections.has('extensions') ? (
              <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
            )}
          </button>
          {expandedSections.has('extensions') && (
            <div className="space-y-1">
              <button className="w-full flex items-center gap-2 px-3 py-2 text-sm text-[#1f1c17] hover:bg-[#f7f3ea] rounded-lg transition-colors">
                <Package className="w-4 h-4 text-[#3e5238]" />
                <span className="flex-1 text-left">Extension Manager</span>
              </button>
              <div className="px-3 py-2 text-xs text-[#7a7568]">
                <div className="flex items-center gap-2 mb-1">
                  <Circle className="w-2 h-2 fill-[#1e4d4a] text-[#1e4d4a]" />
                  <span>OxXlPlay v0.4.1</span>
                </div>
                <div className="flex items-center gap-2">
                  <Circle className="w-2 h-2 fill-[#7a7568] text-[#7a7568]" />
                  <span className="text-[#7a7568]">No other extensions</span>
                </div>
              </div>
            </div>
          )}
        </div>
      </nav>

      {/* Environment Truth Footer */}
      <div className="p-4 border-t border-[#1f1c17]/10 space-y-3">
        {/* Capability Summary */}
        <div className="bg-gradient-to-br from-[#1e4d4a]/10 to-[#3e5238]/10 border border-[#1e4d4a]/30 rounded-lg p-3">
          <button 
            onClick={() => toggleSection('capability')}
            className="w-full flex items-center gap-2 mb-2"
          >
            <Cpu className="w-4 h-4 text-[#1e4d4a]" />
            <span className="text-xs font-semibold text-[#1f1c17]">Host Profile</span>
            {expandedSections.has('capability') ? (
              <ChevronDown className="w-3 h-3 text-[#7a7568] ml-auto" />
            ) : (
              <ChevronRight className="w-3 h-3 text-[#7a7568] ml-auto" />
            )}
          </button>
          {expandedSections.has('capability') ? (
            <div className="text-xs space-y-1.5 font-mono">
              <div className="flex items-center justify-between">
                <span className="text-[#7a7568]">Floor:</span>
                <span className="text-[#1e4d4a] font-semibold">OC-H0</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[#7a7568]">Functions:</span>
                <span className="text-[#1f1c17]">517 supported</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[#7a7568]">Platform:</span>
                <span className="text-[#1f1c17]">Windows</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[#7a7568]">Runtime:</span>
                <span className="text-[#1f1c17]">Native</span>
              </div>
            </div>
          ) : (
            <div className="text-xs text-[#7a7568] space-y-0.5">
              <div className="flex items-center gap-2">
                <span className="text-[#3e5238]">Floor:</span>
                <span className="font-mono text-[#1e4d4a] font-semibold">OC-H0</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[#3e5238]">Platform:</span>
                <span className="font-mono text-[#1f1c17]">Windows • Native</span>
              </div>
            </div>
          )}
        </div>

        {/* Platform Gates */}
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
          <div className="flex items-center gap-2 mb-2">
            <Shield className="w-4 h-4 text-[#c88d2e]" />
            <span className="text-xs font-semibold text-[#1f1c17]">Platform Gates</span>
          </div>
          <div className="text-xs space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-[#7a7568]">Browser:</span>
              <span className="px-2 py-0.5 bg-[#7a7568]/10 text-[#7a7568] rounded text-[10px]">Blocked</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[#7a7568]">Native:</span>
              <span className="px-2 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] rounded text-[10px]">Admitted</span>
            </div>
          </div>
        </div>
      </div>
    </aside>
  );
}
