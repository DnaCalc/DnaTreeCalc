import { useState } from 'react';
import { FileText, Dot, ChevronDown, Clock, Zap, Shield } from 'lucide-react';

type Mode = 'explore' | 'inspect' | 'workbench';

interface FormulaSpaceContextBarProps {
  currentMode: Mode;
  onModeChange: (mode: Mode) => void;
}

export function FormulaSpaceContextBar({ currentMode, onModeChange }: FormulaSpaceContextBarProps) {
  const [showPolicy, setShowPolicy] = useState(false);

  return (
    <div className="border-b border-[#1f1c17]/10 bg-gradient-to-r from-[#f7f3ea] to-[#ede7da]">
      {/* Main Context Bar */}
      <div className="h-12 flex items-center justify-between px-6">
        {/* Left: Active Formula Space Info */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4 text-[#1e4d4a]" />
            <span className="text-sm font-semibold text-[#1f1c17]">LET Formula Analysis</span>
            <Dot className="w-3 h-3 text-[#c88d2e]" />
          </div>
          
          {/* Scenario Policy Controls */}
          <div className="relative">
            <button
              onClick={() => setShowPolicy(!showPolicy)}
              className="flex items-center gap-2 px-3 py-1 bg-white border border-[#1f1c17]/15 rounded hover:bg-[#faf7f1] transition-colors"
            >
              <Clock className="w-3.5 h-3.5 text-[#c88d2e]" />
              <span className="text-xs text-[#1f1c17] font-medium">Deterministic</span>
              <ChevronDown className="w-3 h-3 text-[#7a7568]" />
            </button>
            
            {showPolicy && (
              <div className="absolute top-full left-0 mt-1 w-64 bg-white border border-[#1f1c17]/15 rounded-lg shadow-lg z-50">
                <div className="p-3">
                  <div className="text-xs font-semibold text-[#1f1c17] mb-2">Scenario Policy</div>
                  <div className="space-y-2">
                    <label className="flex items-start gap-2 cursor-pointer">
                      <input 
                        type="radio" 
                        name="policy" 
                        defaultChecked
                        className="mt-0.5"
                      />
                      <div>
                        <div className="text-xs font-medium text-[#1f1c17]">Deterministic</div>
                        <div className="text-xs text-[#7a7568]">
                          Freeze NOW(), TODAY(), RAND() at scenario creation time
                        </div>
                      </div>
                    </label>
                    <label className="flex items-start gap-2 cursor-pointer">
                      <input 
                        type="radio" 
                        name="policy"
                        className="mt-0.5"
                      />
                      <div>
                        <div className="text-xs font-medium text-[#1f1c17]">Real-time</div>
                        <div className="text-xs text-[#7a7568]">
                          Allow NOW(), TODAY() to update; freeze RAND()
                        </div>
                      </div>
                    </label>
                    <label className="flex items-start gap-2 cursor-pointer">
                      <input 
                        type="radio" 
                        name="policy"
                        className="mt-0.5"
                      />
                      <div>
                        <div className="text-xs font-medium text-[#1f1c17]">Full Random</div>
                        <div className="text-xs text-[#7a7568]">
                          Allow all volatile functions to update freely
                        </div>
                      </div>
                    </label>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Center: Mode Switcher */}
        <div className="flex items-center gap-1 bg-white border border-[#1f1c17]/15 rounded-lg p-1">
          <button
            onClick={() => onModeChange('explore')}
            className={`px-4 py-1.5 text-sm font-medium rounded transition-colors ${
              currentMode === 'explore'
                ? 'bg-[#1e4d4a] text-white shadow-sm'
                : 'text-[#7a7568] hover:text-[#1f1c17] hover:bg-[#f7f3ea]'
            }`}
          >
            Explore
          </button>
          <button
            onClick={() => onModeChange('inspect')}
            className={`px-4 py-1.5 text-sm font-medium rounded transition-colors ${
              currentMode === 'inspect'
                ? 'bg-[#3e5238] text-white shadow-sm'
                : 'text-[#7a7568] hover:text-[#1f1c17] hover:bg-[#f7f3ea]'
            }`}
          >
            Inspect
          </button>
          <button
            onClick={() => onModeChange('workbench')}
            className={`px-4 py-1.5 text-sm font-medium rounded transition-colors ${
              currentMode === 'workbench'
                ? 'bg-[#b84532] text-white shadow-sm'
                : 'text-[#7a7568] hover:text-[#1f1c17] hover:bg-[#f7f3ea]'
            }`}
          >
            Workbench
          </button>
        </div>

        {/* Right: Compact Host Truth */}
        <div className="flex items-center gap-4 text-xs">
          <div className="flex items-center gap-2 text-[#7a7568]">
            <Zap className="w-3.5 h-3.5" />
            <span className="font-mono">1.2ms</span>
          </div>
          <div className="flex items-center gap-2">
            <Shield className="w-3.5 h-3.5 text-[#1e4d4a]" />
            <span className="font-mono font-semibold text-[#1e4d4a]">OC-H0</span>
          </div>
          <div className="px-2 py-1 bg-[#1e4d4a]/10 text-[#1e4d4a] rounded text-[10px] font-medium">
            517 functions
          </div>
        </div>
      </div>
    </div>
  );
}
