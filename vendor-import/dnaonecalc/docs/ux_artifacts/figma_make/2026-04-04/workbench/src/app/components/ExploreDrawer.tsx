import { X, Palette, Sliders, Flag, AlertCircle } from 'lucide-react';

type DrawerType = 'formatting' | 'settings' | null;

interface ExploreDrawerProps {
  type: DrawerType;
  onClose: () => void;
}

export function ExploreDrawer({ type, onClose }: ExploreDrawerProps) {
  if (!type) return null;

  return (
    <aside className="w-[360px] border-l border-[#1f1c17]/10 bg-[#f7f3ea] overflow-y-auto flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-0 bg-[#ede7da] border-b border-[#1f1c17]/10 p-4 z-10">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {type === 'formatting' && <Palette className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'settings' && <Sliders className="w-5 h-5 text-[#3e5238]" />}
            <h3 className="text-sm font-semibold text-[#1f1c17]">
              {type === 'formatting' ? 'Formatting' : 'Scenario Settings'}
            </h3>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-[#f7f3ea] rounded transition-colors"
          >
            <X className="w-4 h-4 text-[#7a7568]" />
          </button>
        </div>
        <div className="text-xs text-[#7a7568] mt-1">
          {type === 'formatting' ? 'Edit display formatting and conditional rules' : 'Configure scenario-affecting host flags'}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-4">
        {type === 'formatting' && <FormattingContent />}
        {type === 'settings' && <SettingsContent />}
      </div>
    </aside>
  );
}

function FormattingContent() {
  return (
    <div className="space-y-6">
      {/* Number Format */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Number Format</h4>
        <div className="space-y-3">
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Format Code</label>
            <input
              type="text"
              defaultValue="General"
              className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded-lg focus:outline-none focus:ring-2 focus:ring-[#1e4d4a] bg-white"
            />
          </div>
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Decimal Places</label>
            <select className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded-lg focus:outline-none focus:ring-2 focus:ring-[#1e4d4a] bg-white">
              <option>Auto</option>
              <option>0</option>
              <option>1</option>
              <option>2</option>
              <option>3</option>
              <option>4</option>
            </select>
          </div>
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Thousands Separator</label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input type="checkbox" className="rounded border-[#1f1c17]/20 text-[#1e4d4a] focus:ring-[#1e4d4a]" />
              <span className="text-sm text-[#1f1c17]">Use comma separator</span>
            </label>
          </div>
        </div>
      </div>

      {/* Conditional Formatting */}
      <div className="pt-6 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Conditional Formatting</h4>
        <div className="space-y-2.5">
          <label className="flex items-center gap-2.5 text-sm cursor-pointer p-2.5 hover:bg-white rounded-lg transition-colors">
            <input type="checkbox" className="rounded border-[#1f1c17]/20 text-[#1e4d4a] focus:ring-[#1e4d4a]" />
            <div className="flex-1">
              <div className="text-[#1f1c17] font-medium">Result color by value</div>
              <div className="text-xs text-[#7a7568]">Apply color based on numeric value</div>
            </div>
          </label>
          <label className="flex items-center gap-2.5 text-sm cursor-pointer p-2.5 hover:bg-white rounded-lg transition-colors">
            <input type="checkbox" className="rounded border-[#1f1c17]/20 text-[#1e4d4a] focus:ring-[#1e4d4a]" />
            <div className="flex-1">
              <div className="text-[#1f1c17] font-medium">Icon set</div>
              <div className="text-xs text-[#7a7568]">Show icons based on value ranges</div>
            </div>
          </label>
          <label className="flex items-center gap-2.5 text-sm cursor-pointer p-2.5 hover:bg-white rounded-lg transition-colors">
            <input type="checkbox" defaultChecked className="rounded border-[#1f1c17]/20 text-[#1e4d4a] focus:ring-[#1e4d4a]" />
            <div className="flex-1">
              <div className="text-[#1f1c17] font-medium">Data bars</div>
              <div className="text-xs text-[#7a7568]">Display value as horizontal bar</div>
            </div>
          </label>
          <label className="flex items-center gap-2.5 text-sm cursor-pointer p-2.5 hover:bg-white rounded-lg transition-colors">
            <input type="checkbox" className="rounded border-[#1f1c17]/20 text-[#1e4d4a] focus:ring-[#1e4d4a]" />
            <div className="flex-1">
              <div className="text-[#1f1c17] font-medium">Color scale</div>
              <div className="text-xs text-[#7a7568]">Two or three color gradient</div>
            </div>
          </label>
        </div>
      </div>

      {/* Style Presets */}
      <div className="pt-6 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Style Presets</h4>
        <div className="grid grid-cols-2 gap-2">
          <button className="px-3 py-2 text-xs font-medium border-2 border-[#1e4d4a] text-[#1e4d4a] rounded-lg hover:bg-[#1e4d4a]/5 transition-colors">
            Default
          </button>
          <button className="px-3 py-2 text-xs font-medium border border-[#1f1c17]/15 text-[#1f1c17] rounded-lg hover:bg-white transition-colors">
            Currency
          </button>
          <button className="px-3 py-2 text-xs font-medium border border-[#1f1c17]/15 text-[#1f1c17] rounded-lg hover:bg-white transition-colors">
            Percentage
          </button>
          <button className="px-3 py-2 text-xs font-medium border border-[#1f1c17]/15 text-[#1f1c17] rounded-lg hover:bg-white transition-colors">
            Scientific
          </button>
        </div>
      </div>
    </div>
  );
}

function SettingsContent() {
  return (
    <div className="space-y-6">
      {/* Scenario-Affecting Host Flags */}
      <div>
        <div className="flex items-center gap-2 mb-3">
          <Flag className="w-4 h-4 text-[#3e5238]" />
          <h4 className="text-sm font-semibold text-[#1f1c17]">Scenario Flags</h4>
        </div>
        <div className="bg-gradient-to-br from-[#3e5238]/5 to-[#3e5238]/10 border border-[#3e5238]/20 rounded-lg p-4">
          <div className="space-y-3">
            <label className="flex items-start gap-2.5 text-sm cursor-pointer">
              <input type="checkbox" className="mt-0.5 rounded border-[#1f1c17]/20 text-[#3e5238] focus:ring-[#3e5238]" />
              <div className="flex-1">
                <div className="text-[#1f1c17] font-medium">Allow volatile functions</div>
                <div className="text-xs text-[#7a7568] mt-0.5">
                  Permit NOW(), TODAY(), RAND() to update
                </div>
              </div>
            </label>
            <label className="flex items-start gap-2.5 text-sm cursor-pointer">
              <input type="checkbox" defaultChecked className="mt-0.5 rounded border-[#1f1c17]/20 text-[#3e5238] focus:ring-[#3e5238]" />
              <div className="flex-1">
                <div className="text-[#1f1c17] font-medium">Freeze intermediate arrays</div>
                <div className="text-xs text-[#7a7568] mt-0.5">
                  Cache array results for inspection
                </div>
              </div>
            </label>
            <label className="flex items-start gap-2.5 text-sm cursor-pointer">
              <input type="checkbox" defaultChecked className="mt-0.5 rounded border-[#1f1c17]/20 text-[#3e5238] focus:ring-[#3e5238]" />
              <div className="flex-1">
                <div className="text-[#1f1c17] font-medium">Enable result caching</div>
                <div className="text-xs text-[#7a7568] mt-0.5">
                  Cache results for identical formulas
                </div>
              </div>
            </label>
            <label className="flex items-start gap-2.5 text-sm cursor-pointer">
              <input type="checkbox" className="mt-0.5 rounded border-[#1f1c17]/20 text-[#3e5238] focus:ring-[#3e5238]" />
              <div className="flex-1">
                <div className="text-[#1f1c17] font-medium">Strict evaluation mode</div>
                <div className="text-xs text-[#7a7568] mt-0.5">
                  Enforce stricter type checking
                </div>
              </div>
            </label>
          </div>
        </div>
      </div>

      {/* Evaluation Settings */}
      <div className="pt-6 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Evaluation Settings</h4>
        <div className="space-y-3">
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Max Iterations</label>
            <input
              type="number"
              defaultValue="1000"
              className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded-lg focus:outline-none focus:ring-2 focus:ring-[#3e5238] bg-white"
            />
          </div>
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Timeout (ms)</label>
            <input
              type="number"
              defaultValue="5000"
              className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded-lg focus:outline-none focus:ring-2 focus:ring-[#3e5238] bg-white"
            />
          </div>
        </div>
      </div>

      {/* Warning */}
      <div className="pt-6 border-t border-[#1f1c17]/10">
        <div className="bg-[#c88d2e]/5 border border-[#c88d2e]/20 rounded-lg p-3 flex items-start gap-2">
          <AlertCircle className="w-4 h-4 text-[#c88d2e] flex-shrink-0 mt-0.5" />
          <div className="text-xs text-[#7a7568] leading-relaxed">
            Scenario flags affect evaluation behavior. Changes apply to the current formula space only.
          </div>
        </div>
      </div>
    </div>
  );
}
