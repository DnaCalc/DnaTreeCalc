import { X, Users, Send, CheckCircle2, Clock, Package } from 'lucide-react';

type DrawerType = 'witness' | 'handoff' | null;

interface WorkbenchDrawerProps {
  type: DrawerType;
  onClose: () => void;
}

export function WorkbenchDrawer({ type, onClose }: WorkbenchDrawerProps) {
  if (!type) return null;

  return (
    <aside className="w-[380px] border-l border-[#1f1c17]/10 bg-[#f7f3ea] overflow-y-auto flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-0 bg-[#ede7da] border-b border-[#1f1c17]/10 p-4 z-10">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {type === 'witness' && <Users className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'handoff' && <Send className="w-5 h-5 text-[#b84532]" />}
            <h3 className="text-sm font-semibold text-[#1f1c17]">
              {type === 'witness' && 'Witness Chain'}
              {type === 'handoff' && 'Handoff History'}
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
          {type === 'witness' && 'Verification points for this evidence bundle'}
          {type === 'handoff' && 'History of evidence transfers and reviews'}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-4">
        {type === 'witness' && <WitnessContent />}
        {type === 'handoff' && <HandoffContent />}
      </div>
    </aside>
  );
}

function WitnessContent() {
  return (
    <div className="space-y-4">
      {/* Witness Chain */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Verification Points</h4>
        <div className="space-y-3">
          {/* Witness 1 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#1e4d4a] border-2 border-white shadow" />
            <div className="absolute left-[5px] top-5 bottom-0 w-0.5 bg-[#1e4d4a]/20" />
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
                <span className="text-xs font-semibold text-[#1f1c17]">Initial Evaluation</span>
              </div>
              <div className="text-xs text-[#7a7568] mb-2">
                Formula parsed and evaluated successfully
              </div>
              <div className="flex items-center justify-between text-[10px]">
                <span className="text-[#7a7568]">Run #1</span>
                <span className="text-[#7a7568]">Apr 4, 14:32</span>
              </div>
            </div>
          </div>

          {/* Witness 2 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#1e4d4a] border-2 border-white shadow" />
            <div className="absolute left-[5px] top-5 bottom-0 w-0.5 bg-[#1e4d4a]/20" />
            <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
                <span className="text-xs font-semibold text-[#1f1c17]">Replay Consistency</span>
              </div>
              <div className="text-xs text-[#7a7568] mb-2">
                Runs #2-4 matched initial result
              </div>
              <div className="flex items-center justify-between text-[10px]">
                <span className="text-[#7a7568]">3 runs</span>
                <span className="text-[#7a7568]">Apr 4, 14:35-42</span>
              </div>
            </div>
          </div>

          {/* Witness 3 */}
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#c88d2e] border-2 border-white shadow" />
            <div className="bg-white border border-[#c88d2e]/30 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-2">
                <Clock className="w-3.5 h-3.5 text-[#c88d2e]" />
                <span className="text-xs font-semibold text-[#1f1c17]">Latest Verification</span>
              </div>
              <div className="text-xs text-[#7a7568] mb-2">
                Run #5 confirmed consistency
              </div>
              <div className="flex items-center justify-between text-[10px]">
                <span className="text-[#7a7568]">Run #5</span>
                <span className="text-[#7a7568]">Apr 4, 14:45</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Witness Metadata */}
      <div className="pt-4 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Witness Metadata</h4>
        <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3 space-y-2 text-xs">
          <div className="flex items-center justify-between">
            <span className="text-[#7a7568]">Total verifications</span>
            <span className="font-mono text-[#1f1c17]">3</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[#7a7568]">Runs witnessed</span>
            <span className="font-mono text-[#1f1c17]">5</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[#7a7568]">Witness ID</span>
            <span className="font-mono text-[#1f1c17]">wit-5c1e8a</span>
          </div>
        </div>
      </div>

      {/* Traceability */}
      <div className="pt-4 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Traceability</h4>
        <div className="bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10 border border-[#1e4d4a]/20 rounded-lg p-3 text-xs text-[#7a7568] leading-relaxed">
          This witness chain provides verification that the formula has been evaluated consistently across multiple runs. Each verification point is traceable to a specific run and timestamp.
        </div>
      </div>
    </div>
  );
}

function HandoffContent() {
  return (
    <div className="space-y-4">
      {/* Handoff History */}
      <div>
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Transfer History</h4>
        <div className="bg-gradient-to-br from-[#7a7568]/5 to-[#7a7568]/10 border border-[#7a7568]/20 rounded-lg p-3 mb-4">
          <div className="text-xs text-[#7a7568] leading-relaxed">
            No handoffs recorded yet. Click "Handoff Evidence" to transfer this bundle for review.
          </div>
        </div>

        {/* Example handoff (commented) */}
        {/* 
        <div className="space-y-3">
          <div className="relative pl-6">
            <div className="absolute left-0 top-2 w-3 h-3 rounded-full bg-[#b84532] border-2 border-white shadow" />
            <div className="absolute left-[5px] top-5 bottom-0 w-0.5 bg-[#b84532]/20" />
            <div className="bg-white border border-[#b84532]/30 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-2">
                <Send className="w-3.5 h-3.5 text-[#b84532]" />
                <span className="text-xs font-semibold text-[#1f1c17]">Handoff to Review Team</span>
              </div>
              <div className="text-xs text-[#7a7568] mb-2">
                Evidence bundle transferred for verification
              </div>
              <div className="grid grid-cols-2 gap-2 text-[10px] mb-2">
                <div>
                  <span className="text-[#7a7568]">Recipient:</span>
                  <span className="font-mono text-[#1f1c17] ml-1">team-qa</span>
                </div>
                <div>
                  <span className="text-[#7a7568]">Status:</span>
                  <span className="font-mono text-[#c88d2e] ml-1">Pending</span>
                </div>
              </div>
              <div className="text-[10px] text-[#7a7568]">Apr 4, 15:00</div>
            </div>
          </div>
        </div>
        */}
      </div>

      {/* Handoff Configuration */}
      <div className="pt-4 border-t border-[#1f1c17]/10">
        <h4 className="text-sm font-semibold text-[#1f1c17] mb-3">Handoff Configuration</h4>
        <div className="space-y-3">
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Recipient</label>
            <select className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded-lg focus:outline-none focus:ring-2 focus:ring-[#b84532] bg-white">
              <option>Select recipient...</option>
              <option>Review Team (team-qa)</option>
              <option>Archive (archive-main)</option>
              <option>External Auditor (ext-audit)</option>
            </select>
          </div>

          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Include</label>
            <div className="space-y-2">
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <input
                  type="checkbox"
                  defaultChecked
                  className="rounded border-[#1f1c17]/20 text-[#b84532] focus:ring-[#b84532]"
                />
                <span className="text-[#1f1c17]">Evidence bundle</span>
              </label>
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <input
                  type="checkbox"
                  defaultChecked
                  className="rounded border-[#1f1c17]/20 text-[#b84532] focus:ring-[#b84532]"
                />
                <span className="text-[#1f1c17]">Replay lineage</span>
              </label>
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <input
                  type="checkbox"
                  defaultChecked
                  className="rounded border-[#1f1c17]/20 text-[#b84532] focus:ring-[#b84532]"
                />
                <span className="text-[#1f1c17]">Witness chain</span>
              </label>
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <input
                  type="checkbox"
                  className="rounded border-[#1f1c17]/20 text-[#b84532] focus:ring-[#b84532]"
                />
                <span className="text-[#1f1c17]">Source formula</span>
              </label>
            </div>
          </div>

          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Notes</label>
            <textarea
              className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded-lg focus:outline-none focus:ring-2 focus:ring-[#b84532] bg-white resize-none"
              rows={3}
              placeholder="Add notes for the recipient..."
            />
          </div>
        </div>
      </div>

      {/* Handoff Action */}
      <div className="pt-4 border-t border-[#1f1c17]/10">
        <button className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-[#b84532] text-white rounded-lg hover:bg-[#8a3424] transition-colors font-medium text-sm shadow-sm">
          <Send className="w-4 h-4" />
          Create Handoff
        </button>
      </div>
    </div>
  );
}
