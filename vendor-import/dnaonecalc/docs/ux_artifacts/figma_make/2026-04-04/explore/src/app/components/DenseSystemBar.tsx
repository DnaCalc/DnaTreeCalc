import { useState } from 'react';
import { ChevronDown, Eye, EyeOff, Code, Layers, Settings } from 'lucide-react';

interface DenseSystemBarProps {
  onToggleInspector: () => void;
  onToggleCapability: () => void;
  inspectorOpen: boolean;
  capabilityOpen: boolean;
}

export function DenseSystemBar({ 
  onToggleInspector, 
  onToggleCapability,
  inspectorOpen,
  capabilityOpen 
}: DenseSystemBarProps) {
  const [showDetails, setShowDetails] = useState(false);

  return (
    <div className="border-b border-[#1f1c17]/10 bg-gradient-to-r from-[#ede7da] to-[#f7f3ea]">
      {/* Main System Bar */}
      <div className="h-9 flex items-center justify-between px-4 text-xs">
        <div className="flex items-center gap-4">
          {/* Host Profile */}
          <div className="flex items-center gap-2">
            <span className="text-[#7a7568]">Host:</span>
            <span className="font-mono font-semibold text-[#1e4d4a]">OC-H0</span>
          </div>

          {/* Packet Kinds */}
          <div className="flex items-center gap-2">
            <span className="text-[#7a7568]">Packets:</span>
            <div className="flex items-center gap-1">
              <span className="px-1.5 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] rounded font-mono text-[10px]">
                formula_edit
              </span>
              <span className="px-1.5 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] rounded font-mono text-[10px]">
                edit_accept_recalc
              </span>
              <span className="px-1.5 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] rounded font-mono text-[10px]">
                replay_capture
              </span>
            </div>
          </div>

          {/* Function Policy */}
          <div className="flex items-center gap-2">
            <span className="text-[#7a7568]">Functions:</span>
            <span className="font-mono text-[#1f1c17]">supported=517</span>
          </div>

          {/* Toggle Details */}
          <button
            onClick={() => setShowDetails(!showDetails)}
            className="flex items-center gap-1 text-[#7a7568] hover:text-[#1f1c17] transition-colors"
          >
            <ChevronDown className={`w-3.5 h-3.5 transition-transform ${showDetails ? 'rotate-180' : ''}`} />
            <span>{showDetails ? 'Less' : 'More'}</span>
          </button>
        </div>

        <div className="flex items-center gap-3">
          {/* Conditional Formatting Status */}
          <div className="px-2 py-1 bg-[#c88d2e]/10 text-[#c88d2e] rounded text-[10px] font-medium">
            Conditional Formatting: ON
          </div>

          {/* Toggle Buttons */}
          <button
            onClick={onToggleInspector}
            className={`px-2 py-1 text-[10px] font-medium rounded transition-colors ${
              inspectorOpen
                ? 'bg-[#1e4d4a] text-white'
                : 'bg-white text-[#7a7568] hover:bg-[#f7f3ea]'
            }`}
          >
            {inspectorOpen ? 'Hide' : 'Show'} Inspector
          </button>
          
          <button
            onClick={onToggleCapability}
            className={`px-2 py-1 text-[10px] font-medium rounded transition-colors ${
              capabilityOpen
                ? 'bg-[#1e4d4a] text-white'
                : 'bg-white text-[#7a7568] hover:bg-[#f7f3ea]'
            }`}
          >
            {capabilityOpen ? 'Hide' : 'Show'} Capability
          </button>
        </div>
      </div>

      {/* Expanded Details */}
      {showDetails && (
        <div className="border-t border-[#1f1c17]/10 bg-[#f7f3ea] px-4 py-2 space-y-1.5 text-xs font-mono">
          <div className="flex items-center gap-2">
            <span className="text-[#7a7568] w-32">Runtime Platform:</span>
            <span className="text-[#1f1c17]">Desktop native host only; browser and secondary hosts not admitted yet.</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-[#7a7568] w-32">Conditional Format:</span>
            <span className="text-[#1f1c17]">
              fill_color=F6, color=blue, colorfont_color=blod, italic, underline, simple_border, 
              number_format_code=local, icon_set=html, icon_set_html, range_priority_graph, 
              stop_if_true_graph, &book_global_scope
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-[#7a7568] w-32">Blocked Data:</span>
            <span className="text-[#1f1c17]">
              barstwo_color_scale, three_color_scale, rich_icon_set, html, range_priority_graph, 
              stop_if_true_graph, &book_global_scope
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-[#7a7568] w-32">Capability Eval:</span>
            <span className="text-[#1f1c17]">all snapshot = 17 capability + fill story</span>
          </div>
        </div>
      )}
    </div>
  );
}
