import { useState } from 'react';
import { ChevronDown, ChevronRight, Edit2, Copy } from 'lucide-react';

export function DenseResultPanel() {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['display']));

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
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-[#1f1c17]">Result</h3>
        <div className="flex items-center gap-2 text-xs">
          <div className="w-1.5 h-1.5 rounded-full bg-[#1e4d4a]" />
          <span className="text-[#1e4d4a] font-medium">Evaluated</span>
        </div>
      </div>

      {/* Effective Display - Expanded by Default */}
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('display')}
          className="w-full flex items-center justify-between px-3 py-2 hover:bg-[#f7f3ea] transition-colors text-left"
        >
          <span className="text-xs font-semibold text-[#1f1c17]">Effective Display</span>
          {expandedSections.has('display') ? (
            <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
          )}
        </button>
        {expandedSections.has('display') && (
          <div className="px-3 py-3 border-t border-[#1f1c17]/10">
            <div className="flex items-end gap-3 mb-3">
              <div className="font-mono text-4xl text-[#1e4d4a] font-semibold">6</div>
              <div className="text-xs text-[#7a7568] pb-2">Number • Scalar</div>
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                <span className="text-[#7a7568]">Format</span>
                <button className="flex items-center gap-1 text-[#1e4d4a] hover:text-[#2d6864] font-mono font-medium">
                  none
                  <Edit2 className="w-3 h-3" />
                </button>
              </div>
              <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                <span className="text-[#7a7568]">Source</span>
                <span className="font-mono text-[#1f1c17] text-xs">none</span>
              </div>
              <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                <span className="text-[#7a7568]">Plain</span>
                <span className="font-mono text-[#1f1c17] text-xs">simple</span>
              </div>
              <div className="flex items-center justify-between py-1.5 px-2 bg-[#f7f3ea] rounded">
                <span className="text-[#7a7568]">Style</span>
                <span className="font-mono text-[#1f1c17] text-xs">default</span>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Worksheet Value - Collapsed by Default */}
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('worksheet')}
          className="w-full flex items-center justify-between px-3 py-2 hover:bg-[#f7f3ea] transition-colors text-left"
        >
          <span className="text-xs font-semibold text-[#1f1c17]">Worksheet Value</span>
          {expandedSections.has('worksheet') ? (
            <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
          )}
        </button>
        {expandedSections.has('worksheet') && (
          <div className="px-3 py-2 border-t border-[#1f1c17]/10 font-mono text-xs space-y-1">
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">worksheet_value:</span>
              <span className="text-[#1f1c17]">Number(6)</span>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">payload_summary:</span>
              <span className="text-[#1f1c17]">Number</span>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">returned_surface:</span>
              <span className="text-[#c88d2e]">PrimaryValue</span>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">presentation_hint:</span>
              <span className="text-[#7a7568]">none</span>
            </div>
          </div>
        )}
      </div>

      {/* Host State - Collapsed by Default */}
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('host')}
          className="w-full flex items-center justify-between px-3 py-2 hover:bg-[#f7f3ea] transition-colors text-left"
        >
          <span className="text-xs font-semibold text-[#1f1c17]">Host State</span>
          {expandedSections.has('host') ? (
            <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
          )}
        </button>
        {expandedSections.has('host') && (
          <div className="px-3 py-2 border-t border-[#1f1c17]/10 font-mono text-xs space-y-1">
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">host_style_state:</span>
              <span className="text-[#7a7568]">none</span>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">effective_display:</span>
              <span className="text-[#7a7568]">none</span>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-[#7a7568]">commit_decision:</span>
              <span className="text-[#1e4d4a]">accepted</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
