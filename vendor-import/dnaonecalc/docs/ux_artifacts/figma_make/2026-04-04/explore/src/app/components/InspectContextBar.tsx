import { FileText, Dot, Eye, Shield, Zap, ChevronRight } from 'lucide-react';

interface InspectContextBarProps {
  onOpenDrawer: (drawer: 'provenance' | 'context') => void;
}

export function InspectContextBar({ onOpenDrawer }: InspectContextBarProps) {
  return (
    <div className="border-b border-[#1f1c17]/10 bg-gradient-to-r from-[#f7f3ea] to-[#ede7da]">
      <div className="h-12 flex items-center justify-between px-6">
        {/* Left: Active Formula Space */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4 text-[#3e5238]" />
            <span className="text-sm font-semibold text-[#1f1c17]">LET Formula Analysis</span>
            <Dot className="w-3 h-3 text-[#c88d2e]" />
          </div>
          
          {/* Scenario Policy (Read-only indicator) */}
          <div className="flex items-center gap-2 px-3 py-1.5 bg-white/60 border border-[#1f1c17]/10 rounded-lg">
            <span className="text-xs text-[#7a7568]">Policy:</span>
            <span className="text-xs text-[#1f1c17] font-medium">Deterministic</span>
          </div>
        </div>

        {/* Center: Mode Badge */}
        <div className="flex items-center gap-2 px-4 py-1.5 bg-[#3e5238]/5 border border-[#3e5238]/20 rounded-lg">
          <Eye className="w-3.5 h-3.5 text-[#3e5238]" />
          <span className="text-xs font-medium text-[#3e5238]">Inspect Mode</span>
        </div>

        {/* Right: Context & Host Truth */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => onOpenDrawer('provenance')}
            className="flex items-center gap-1.5 text-xs text-[#3e5238] hover:text-[#566b4f] font-medium"
          >
            Provenance
            <ChevronRight className="w-3 h-3" />
          </button>
          <button
            onClick={() => onOpenDrawer('context')}
            className="flex items-center gap-1.5 text-xs text-[#3e5238] hover:text-[#566b4f] font-medium"
          >
            Host Context
            <ChevronRight className="w-3 h-3" />
          </button>
          <div className="h-5 w-px bg-[#1f1c17]/10" />
          <div className="flex items-center gap-3 text-xs">
            <div className="flex items-center gap-1.5 text-[#7a7568]">
              <Zap className="w-3.5 h-3.5" />
              <span className="font-mono">1.2ms</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Shield className="w-3.5 h-3.5 text-[#3e5238]" />
              <span className="font-mono font-semibold text-[#3e5238]">OC-H0</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
