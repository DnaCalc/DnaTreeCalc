import { useState } from 'react';
import { ChevronDown, ChevronRight, X, Copy, AlertCircle, Cpu, Database, Layers } from 'lucide-react';

interface DenseInspectorDrawerProps {
  onClose: () => void;
}

export function DenseInspectorDrawer({ onClose }: DenseInspectorDrawerProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(['diagnostics', 'capability'])
  );

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
    <aside className="w-[420px] border-l border-[#1f1c17]/10 bg-[#f7f3ea] overflow-y-auto flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-0 bg-[#ede7da] border-b border-[#1f1c17]/10 p-3 z-10">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-[#1f1c17]">Inspector</h3>
          <button
            onClick={onClose}
            className="p-1 hover:bg-[#f7f3ea] rounded transition-colors"
          >
            <X className="w-4 h-4 text-[#7a7568]" />
          </button>
        </div>
        <div className="text-xs text-[#7a7568] mt-1">
          Supporting dock • Collapse with Esc
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-3 space-y-2">
        {/* Diagnostics Section */}
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
          <button
            onClick={() => toggleSection('diagnostics')}
            className="w-full flex items-center justify-between px-3 py-2 hover:bg-[#f7f3ea] transition-colors text-left"
          >
            <div className="flex items-center gap-2">
              <AlertCircle className="w-4 h-4 text-[#c88d2e]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Diagnostics</span>
            </div>
            {expandedSections.has('diagnostics') ? (
              <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
            )}
          </button>
          {expandedSections.has('diagnostics') && (
            <div className="px-3 py-2 border-t border-[#1f1c17]/10">
              <div className="space-y-1.5 font-mono text-xs">
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">probe_parse_diagnostic_count:</span>
                  <span className="text-[#1f1c17]">0</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">edit_packet_diagnostic_count:</span>
                  <span className="text-[#1f1c17]">0</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">text_start_pos:</span>
                  <span className="text-[#1f1c17]">0</span>
                </div>
                <div className="py-1">
                  <div className="text-[#7a7568] mb-1">packet_kinds:</div>
                  <div className="pl-2 text-[#1e4d4a] space-y-0.5">
                    <div>formula_edit</div>
                    <div>edit_accept_recalc</div>
                    <div>replay_capture</div>
                  </div>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">buffer_len:</span>
                  <span className="text-[#1f1c17]">11</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">cursor_index:</span>
                  <span className="text-[#1f1c17]">11</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">selection:</span>
                  <span className="text-[#1f1c17]">11..11</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">selected_text:</span>
                  <span className="text-[#7a7568]">""</span>
                </div>
                <div className="py-1">
                  <div className="text-[#7a7568] mb-1">edit_formula_token:</div>
                  <div className="pl-2 text-[#1f1c17] break-all">
                    e5f9254907f3ea74
                  </div>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">edit_diagnostic_count:</span>
                  <span className="text-[#1f1c17]">0</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">text_change_range:</span>
                  <span className="text-[#7a7568]">None</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Capability Center Section */}
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
          <button
            onClick={() => toggleSection('capability')}
            className="w-full flex items-center justify-between px-3 py-2 hover:bg-[#f7f3ea] transition-colors text-left"
          >
            <div className="flex items-center gap-2">
              <Cpu className="w-4 h-4 text-[#1e4d4a]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Capability Center</span>
            </div>
            {expandedSections.has('capability') ? (
              <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
            )}
          </button>
          {expandedSections.has('capability') && (
            <div className="px-3 py-2 border-t border-[#1f1c17]/10">
              <div className="space-y-1.5 font-mono text-xs">
                <div className="py-1">
                  <div className="text-[#7a7568] mb-1">snapshot_id:</div>
                  <div className="pl-2 text-[#1f1c17] break-all text-[10px]">
                    capability-snapshot-oc-h0-edit-accept-recalc-t75244163946
                  </div>
                </div>
                <div className="py-1">
                  <div className="text-[#7a7568] mb-1">packet_kinds:</div>
                  <div className="pl-2 text-[#1e4d4a] space-y-0.5">
                    <div>formula_edit</div>
                    <div>edit_accept_recalc</div>
                  </div>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">runtime_platform:</span>
                  <span className="text-[#1f1c17]">windows</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">runtime_class:</span>
                  <span className="text-[#1f1c17]">desktop_native_only</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">capability_floor:</span>
                  <span className="text-[#1e4d4a] font-semibold">OC-H0</span>
                </div>
                <div className="py-1">
                  <div className="text-[#7a7568] mb-1">seam_pin_set_id:</div>
                  <div className="pl-2 text-[#1f1c17]">
                    onecalc1us-05:capability
                  </div>
                </div>
                <div className="py-1">
                  <div className="text-[#7a7568] mb-1">function_surface_policy:</div>
                  <div className="pl-2 text-[#1f1c17]">
                    onecalc1:admitted
                  </div>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">execution:</span>
                  <span className="text-[#1e4d4a]">supported=517</span>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">preview:</span>
                  <span className="text-[#1f1c17]">0</span>
                </div>
                <div className="py-1">
                  <div className="text-[#7a7568] mb-1">experimentals:</div>
                  <div className="pl-2 text-[#7a7568]">
                    0; deferred=17
                  </div>
                </div>
                <div className="flex items-start justify-between py-1">
                  <span className="text-[#7a7568]">catalog_only:</span>
                  <span className="text-[#1f1c17]">0</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Dependency Ledger Section */}
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg overflow-hidden">
          <button
            onClick={() => toggleSection('dependency')}
            className="w-full flex items-center justify-between px-3 py-2 hover:bg-[#f7f3ea] transition-colors text-left"
          >
            <div className="flex items-center gap-2">
              <Layers className="w-4 h-4 text-[#3e5238]" />
              <span className="text-xs font-semibold text-[#1f1c17]">Dependency Ledger</span>
            </div>
            {expandedSections.has('dependency') ? (
              <ChevronDown className="w-3.5 h-3.5 text-[#7a7568]" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-[#7a7568]" />
            )}
          </button>
          {expandedSections.has('dependency') && (
            <div className="px-3 py-2 border-t border-[#1f1c17]/10">
              <div className="space-y-1 font-mono text-xs text-[#7a7568]">
                <div>dnaonecalc-host</div>
                <div>oxfml_core</div>
                <div>oxfunc_core</div>
                <div>oxreplay_abstractions</div>
                <div>oxreplay_core</div>
              </div>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
