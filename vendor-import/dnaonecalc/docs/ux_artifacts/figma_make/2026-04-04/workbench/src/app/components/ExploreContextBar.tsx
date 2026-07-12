import { useState } from 'react';
import { FileText, Dot, ChevronDown, Clock, Zap, Shield, Palette, Sliders } from 'lucide-react';

interface ExploreContextBarProps {
  onOpenDrawer: (drawer: 'formatting' | 'settings') => void;
}

export function ExploreContextBar({ onOpenDrawer }: ExploreContextBarProps) {
  const [showPolicy, setShowPolicy] = useState(false);

  return (
    <div className="border-b border-[#1f1c17]/10 bg-gradient-to-r from-[#f7f3ea] to-[#ede7da]">
      <div className="h-12 flex items-center justify-between px-6">
        {/* Left: Active Formula Space */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4 text-[#1e4d4a]" />
            <span className="text-sm font-semibold text-[#1f1c17]">LET Formula Analysis</span>
            <Dot className="w-3 h-3 text-[#c88d2e]" />
          </div>
          
          {/* Scenario Policy */}
          <div className="relative">
            <button
              onClick={() => setShowPolicy(!showPolicy)}
              className="flex items-center gap-2 px-3 py-1.5 bg-white border border-[#1f1c17]/15 rounded-lg hover:bg-[#faf7f1] transition-colors"
            >
              <Clock className="w-3.5 h-3.5 text-[#c88d2e]" />
              <span className="text-xs text-[#1f1c17] font-medium">Deterministic</span>
              <ChevronDown className="w-3 h-3 text-[#7a7568]" />
            </button>
            
            {showPolicy && (
              <div className="absolute top-full left-0 mt-1 w-72 bg-white border border-[#1f1c17]/15 rounded-lg shadow-lg z-50">
                <div className="p-3">
                  <div className="text-xs font-semibold text-[#1f1c17] mb-3">Scenario Policy</div>
                  <div className="space-y-2.5">
                    <label className="flex items-start gap-2.5 cursor-pointer p-2 hover:bg-[#f7f3ea] rounded transition-colors">
                      <input 
                        type="radio" 
                        name="policy" 
                        defaultChecked
                        className="mt-0.5 text-[#1e4d4a] focus:ring-[#1e4d4a]"
                      />
                      <div className="flex-1">
                        <div className="text-xs font-medium text-[#1f1c17] mb-0.5">Deterministic</div>
                        <div className="text-xs text-[#7a7568] leading-relaxed">
                          Freeze NOW(), TODAY(), RAND() at scenario creation
                        </div>
                      </div>
                    </label>
                    <label className="flex items-start gap-2.5 cursor-pointer p-2 hover:bg-[#f7f3ea] rounded transition-colors">
                      <input 
                        type="radio" 
                        name="policy"
                        className="mt-0.5 text-[#1e4d4a] focus:ring-[#1e4d4a]"
                      />
                      <div className="flex-1">
                        <div className="text-xs font-medium text-[#1f1c17] mb-0.5">Real-time</div>
                        <div className="text-xs text-[#7a7568] leading-relaxed">
                          Allow NOW(), TODAY() to update; freeze RAND()
                        </div>
                      </div>
                    </label>
                    <label className="flex items-start gap-2.5 cursor-pointer p-2 hover:bg-[#f7f3ea] rounded transition-colors">
                      <input 
                        type="radio" 
                        name="policy"
                        className="mt-0.5 text-[#1e4d4a] focus:ring-[#1e4d4a]"
                      />
                      <div className="flex-1">
                        <div className="text-xs font-medium text-[#1f1c17] mb-0.5">Full Random</div>
                        <div className="text-xs text-[#7a7568] leading-relaxed">
                          All volatile functions update freely
                        </div>
                      </div>
                    </label>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Center: Mode Badge */}
        <div className="flex items-center gap-2 px-4 py-1.5 bg-[#1e4d4a]/5 border border-[#1e4d4a]/20 rounded-lg">
          <div className="w-1.5 h-1.5 rounded-full bg-[#1e4d4a]" />
          <span className="text-xs font-medium text-[#1e4d4a]">Explore Mode</span>
        </div>

        {/* Right: Tools & Host Truth */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => onOpenDrawer('formatting')}
            className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-[#1f1c17] border border-[#1f1c17]/15 rounded-lg hover:bg-white transition-colors"
          >
            <Palette className="w-3.5 h-3.5" />
            Formatting
          </button>
          <button
            onClick={() => onOpenDrawer('settings')}
            className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-[#1f1c17] border border-[#1f1c17]/15 rounded-lg hover:bg-white transition-colors"
          >
            <Sliders className="w-3.5 h-3.5" />
            Settings
          </button>
          <div className="h-5 w-px bg-[#1f1c17]/10" />
          <div className="flex items-center gap-3 text-xs">
            <div className="flex items-center gap-1.5 text-[#7a7568]">
              <Zap className="w-3.5 h-3.5" />
              <span className="font-mono">1.2ms</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Shield className="w-3.5 h-3.5 text-[#1e4d4a]" />
              <span className="font-mono font-semibold text-[#1e4d4a]">OC-H0</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
