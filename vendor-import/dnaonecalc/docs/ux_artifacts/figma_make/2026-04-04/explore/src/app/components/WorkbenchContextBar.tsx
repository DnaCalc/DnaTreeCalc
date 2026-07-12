import { FileText, Dot, Briefcase, Shield, Clock, ChevronRight, Calendar } from 'lucide-react';

interface WorkbenchContextBarProps {
  onOpenDrawer: (drawer: 'witness' | 'handoff') => void;
}

export function WorkbenchContextBar({ onOpenDrawer }: WorkbenchContextBarProps) {
  return (
    <div className="border-b border-[#1f1c17]/10 bg-gradient-to-r from-[#f7f3ea] to-[#ede7da]">
      <div className="h-12 flex items-center justify-between px-6">
        {/* Left: Active Formula Space */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4 text-[#b84532]" />
            <span className="text-sm font-semibold text-[#1f1c17]">LET Formula Analysis</span>
            <Dot className="w-3 h-3 text-[#c88d2e]" />
          </div>
          
          {/* Scenario Timestamp */}
          <div className="flex items-center gap-2 px-3 py-1.5 bg-white/60 border border-[#1f1c17]/10 rounded-lg">
            <Calendar className="w-3.5 h-3.5 text-[#7a7568]" />
            <span className="text-xs text-[#7a7568]">Created:</span>
            <span className="text-xs text-[#1f1c17] font-medium">Apr 4, 2026 14:32</span>
          </div>
        </div>

        {/* Center: Mode Badge */}
        <div className="flex items-center gap-2 px-4 py-1.5 bg-[#b84532]/5 border border-[#b84532]/20 rounded-lg">
          <Briefcase className="w-3.5 h-3.5 text-[#b84532]" />
          <span className="text-xs font-medium text-[#b84532]">Workbench Mode</span>
        </div>

        {/* Right: Evidence & Host Truth */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => onOpenDrawer('witness')}
            className="flex items-center gap-1.5 text-xs text-[#b84532] hover:text-[#8a3424] font-medium"
          >
            Witness Chain
            <ChevronRight className="w-3 h-3" />
          </button>
          <button
            onClick={() => onOpenDrawer('handoff')}
            className="flex items-center gap-1.5 text-xs text-[#b84532] hover:text-[#8a3424] font-medium"
          >
            Handoff History
            <ChevronRight className="w-3 h-3" />
          </button>
          <div className="h-5 w-px bg-[#1f1c17]/10" />
          <div className="flex items-center gap-3 text-xs">
            <div className="flex items-center gap-1.5 text-[#7a7568]">
              <Clock className="w-3.5 h-3.5" />
              <span className="font-mono">5 runs</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Shield className="w-3.5 h-3.5 text-[#b84532]" />
              <span className="font-mono font-semibold text-[#b84532]">OC-H0</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
