import { Download, Archive, TrendingUp, Send, AlertCircle, ArrowRight, Lock } from 'lucide-react';

export function WorkbenchActionsPanel() {
  return (
    <div className="flex flex-col h-full bg-[#faf7f1] border-l border-[#1f1c17]/10">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <h2 className="text-sm font-semibold text-[#1f1c17]">Actions & Next Steps</h2>
      </div>

      {/* Primary Actions */}
      <div className="p-4 border-b border-[#1f1c17]/10">
        <h3 className="text-xs font-semibold text-[#1f1c17] mb-3">Primary Actions</h3>
        <div className="space-y-2">
          <button className="w-full flex items-center gap-3 px-4 py-3 bg-[#b84532] text-white rounded-lg hover:bg-[#8a3424] transition-colors shadow-sm">
            <Send className="w-4 h-4" />
            <span className="flex-1 text-left font-medium text-sm">Handoff Evidence</span>
            <ArrowRight className="w-4 h-4" />
          </button>
          
          <button className="w-full flex items-center gap-3 px-4 py-3 bg-[#3e5238] text-white rounded-lg hover:bg-[#566b4f] transition-colors shadow-sm">
            <Archive className="w-4 h-4" />
            <span className="flex-1 text-left font-medium text-sm">Retain Bundle</span>
            <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Secondary Actions */}
      <div className="p-4 border-b border-[#1f1c17]/10">
        <h3 className="text-xs font-semibold text-[#1f1c17] mb-3">Secondary Actions</h3>
        <div className="space-y-2">
          <button className="w-full flex items-center gap-2 px-3 py-2.5 bg-white border border-[#1f1c17]/15 rounded-lg hover:bg-[#f7f3ea] transition-colors text-sm">
            <Download className="w-3.5 h-3.5 text-[#7a7568]" />
            <span className="flex-1 text-left text-[#1f1c17]">Export as JSON</span>
          </button>
          
          <button className="w-full flex items-center gap-2 px-3 py-2.5 bg-white border border-[#1f1c17]/15 rounded-lg hover:bg-[#f7f3ea] transition-colors text-sm">
            <TrendingUp className="w-3.5 h-3.5 text-[#7a7568]" />
            <span className="flex-1 text-left text-[#1f1c17]">Widen Observation</span>
          </button>
        </div>
      </div>

      {/* Blocked Dimensions */}
      <div className="flex-1 overflow-y-auto p-4">
        <h3 className="text-xs font-semibold text-[#1f1c17] mb-3">Blocked Dimensions</h3>
        <div className="bg-gradient-to-br from-[#7a7568]/5 to-[#7a7568]/10 border border-[#7a7568]/20 rounded-lg p-3 mb-4">
          <div className="flex items-start gap-2">
            <AlertCircle className="w-4 h-4 text-[#7a7568] flex-shrink-0 mt-0.5" />
            <div className="flex-1 text-xs text-[#7a7568] leading-relaxed">
              No dimensions are currently blocked. All comparison points are observable.
            </div>
          </div>
        </div>

        {/* Example of blocked dimension (commented) */}
        {/* 
        <div className="space-y-2">
          <div className="bg-white border border-[#b84532]/30 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-2">
              <Lock className="w-3.5 h-3.5 text-[#b84532]" />
              <span className="text-xs font-semibold text-[#1f1c17]">External API Response</span>
            </div>
            <div className="text-xs text-[#7a7568] mb-2">
              Cannot compare: host provides opaque result
            </div>
            <div className="text-xs text-[#b84532]">
              Reason: Third-party function does not expose internal state
            </div>
          </div>
        </div>
        */}
      </div>

      {/* Next Action Recommendation */}
      <div className="p-4 border-t border-[#1f1c17]/10 bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10">
        <div className="flex items-start gap-2 mb-3">
          <div className="w-1.5 h-1.5 rounded-full bg-[#1e4d4a] flex-shrink-0 mt-1.5" />
          <h3 className="text-xs font-semibold text-[#1f1c17]">Recommended Next Step</h3>
        </div>
        <div className="bg-white border border-[#1e4d4a]/20 rounded-lg p-3 mb-3">
          <div className="text-sm font-medium text-[#1f1c17] mb-1">
            Evidence bundle is complete and reliable
          </div>
          <div className="text-xs text-[#7a7568] leading-relaxed mb-3">
            5 consistent runs with 98% reliability. Consider handing off this evidence for review or retaining for future reference.
          </div>
          <div className="flex items-center gap-2 text-xs text-[#1e4d4a] font-medium">
            <ArrowRight className="w-3 h-3" />
            <span>Click "Handoff Evidence" to proceed</span>
          </div>
        </div>
      </div>
    </div>
  );
}
