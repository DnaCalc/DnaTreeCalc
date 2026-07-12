import { CheckCircle2, XCircle, AlertTriangle, TrendingUp, GitBranch, Circle, ArrowRight } from 'lucide-react';

export function WorkbenchMainPanel() {
  return (
    <div className="flex flex-col h-full bg-white">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <h2 className="text-sm font-semibold text-[#1f1c17] mb-1">Evidence Workbench</h2>
        <div className="text-xs text-[#7a7568]">
          Comparison outcome, replay lineage, and observation envelope
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {/* Comparison Outcome */}
        <section>
          <div className="flex items-center gap-2 mb-4">
            <div className="w-1 h-6 bg-[#b84532] rounded-full" />
            <h3 className="text-base font-semibold text-[#1f1c17]">Comparison Outcome</h3>
          </div>

          <div className="grid grid-cols-3 gap-4 mb-4">
            {/* Matched */}
            <div className="bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10 border-2 border-[#1e4d4a]/30 rounded-xl p-4">
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle2 className="w-5 h-5 text-[#1e4d4a]" />
                <span className="text-xs font-semibold text-[#1f1c17]">Matched</span>
              </div>
              <div className="text-3xl font-mono font-semibold text-[#1e4d4a] mb-1">4</div>
              <div className="text-xs text-[#7a7568]">
                Dimensions matched across runs
              </div>
            </div>

            {/* Differed */}
            <div className="bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border-2 border-[#c88d2e]/30 rounded-xl p-4">
              <div className="flex items-center gap-2 mb-2">
                <XCircle className="w-5 h-5 text-[#c88d2e]" />
                <span className="text-xs font-semibold text-[#1f1c17]">Differed</span>
              </div>
              <div className="text-3xl font-mono font-semibold text-[#c88d2e] mb-1">1</div>
              <div className="text-xs text-[#7a7568]">
                Dimensions with variance
              </div>
            </div>

            {/* Blocked */}
            <div className="bg-gradient-to-br from-[#b84532]/5 to-[#b84532]/10 border-2 border-[#b84532]/30 rounded-xl p-4">
              <div className="flex items-center gap-2 mb-2">
                <AlertTriangle className="w-5 h-5 text-[#b84532]" />
                <span className="text-xs font-semibold text-[#1f1c17]">Blocked</span>
              </div>
              <div className="text-3xl font-mono font-semibold text-[#b84532] mb-1">0</div>
              <div className="text-xs text-[#7a7568]">
                Dimensions unavailable
              </div>
            </div>
          </div>

          {/* Outcome Detail */}
          <div className="bg-[#faf7f1] border border-[#1f1c17]/10 rounded-lg p-4">
            <div className="text-sm font-semibold text-[#1f1c17] mb-3">Outcome Detail</div>
            <div className="space-y-2.5">
              <div className="flex items-center gap-3 p-2.5 bg-white rounded-lg">
                <CheckCircle2 className="w-4 h-4 text-[#1e4d4a] flex-shrink-0" />
                <div className="flex-1">
                  <div className="text-sm text-[#1f1c17] font-medium">Result value</div>
                  <div className="text-xs text-[#7a7568]">Consistent: 30 across all runs</div>
                </div>
              </div>
              <div className="flex items-center gap-3 p-2.5 bg-white rounded-lg">
                <CheckCircle2 className="w-4 h-4 text-[#1e4d4a] flex-shrink-0" />
                <div className="flex-1">
                  <div className="text-sm text-[#1f1c17] font-medium">Type stability</div>
                  <div className="text-xs text-[#7a7568]">Number across all runs</div>
                </div>
              </div>
              <div className="flex items-center gap-3 p-2.5 bg-white rounded-lg">
                <CheckCircle2 className="w-4 h-4 text-[#1e4d4a] flex-shrink-0" />
                <div className="flex-1">
                  <div className="text-sm text-[#1f1c17] font-medium">Array shapes</div>
                  <div className="text-xs text-[#7a7568]">Intermediate arrays consistent</div>
                </div>
              </div>
              <div className="flex items-center gap-3 p-2.5 bg-white rounded-lg">
                <CheckCircle2 className="w-4 h-4 text-[#1e4d4a] flex-shrink-0" />
                <div className="flex-1">
                  <div className="text-sm text-[#1f1c17] font-medium">Evaluation path</div>
                  <div className="text-xs text-[#7a7568]">Same semantic tree across runs</div>
                </div>
              </div>
              <div className="flex items-center gap-3 p-2.5 bg-white rounded-lg border-2 border-[#c88d2e]/30">
                <XCircle className="w-4 h-4 text-[#c88d2e] flex-shrink-0" />
                <div className="flex-1">
                  <div className="text-sm text-[#1f1c17] font-medium">Timing variance</div>
                  <div className="text-xs text-[#7a7568]">0.8ms–1.5ms • Acceptable range</div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Replay Lineage */}
        <section>
          <div className="flex items-center gap-2 mb-4">
            <div className="w-1 h-6 bg-[#b84532] rounded-full" />
            <h3 className="text-base font-semibold text-[#1f1c17]">Replay Lineage</h3>
          </div>

          <div className="bg-[#faf7f1] border border-[#1f1c17]/10 rounded-lg p-4">
            <div className="text-sm font-semibold text-[#1f1c17] mb-4">Timeline of Runs</div>
            
            {/* Timeline */}
            <div className="relative">
              {/* Run 1 */}
              <div className="relative pb-6">
                <div className="absolute left-0 top-2 w-4 h-4 rounded-full bg-[#3e5238] border-2 border-white shadow-md z-10" />
                <div className="absolute left-[7px] top-6 bottom-0 w-0.5 bg-[#3e5238]/20" />
                <div className="ml-8 bg-white border border-[#3e5238]/20 rounded-lg p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-semibold text-[#3e5238]">Run #1</span>
                      <span className="text-xs text-[#7a7568]">Initial</span>
                    </div>
                    <span className="text-xs text-[#7a7568]">Apr 4, 14:32</span>
                  </div>
                  <div className="grid grid-cols-3 gap-2 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Result</span>
                      <span className="font-mono text-[#1f1c17]">30</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Time</span>
                      <span className="font-mono text-[#1f1c17]">1.2ms</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Policy</span>
                      <span className="font-mono text-[#1f1c17]">Det</span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Run 2 */}
              <div className="relative pb-6">
                <div className="absolute left-0 top-2 w-4 h-4 rounded-full bg-[#1e4d4a] border-2 border-white shadow-md z-10" />
                <div className="absolute left-[7px] top-6 bottom-0 w-0.5 bg-[#1e4d4a]/20" />
                <div className="ml-8 bg-white border border-[#1e4d4a]/20 rounded-lg p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-semibold text-[#1e4d4a]">Run #2</span>
                      <span className="text-xs text-[#7a7568]">Replay</span>
                    </div>
                    <span className="text-xs text-[#7a7568]">Apr 4, 14:35</span>
                  </div>
                  <div className="grid grid-cols-3 gap-2 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Result</span>
                      <span className="font-mono text-[#1f1c17]">30</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Time</span>
                      <span className="font-mono text-[#1f1c17]">0.9ms</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Policy</span>
                      <span className="font-mono text-[#1f1c17]">Det</span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Run 3 */}
              <div className="relative pb-6">
                <div className="absolute left-0 top-2 w-4 h-4 rounded-full bg-[#1e4d4a] border-2 border-white shadow-md z-10" />
                <div className="absolute left-[7px] top-6 bottom-0 w-0.5 bg-[#1e4d4a]/20" />
                <div className="ml-8 bg-white border border-[#1e4d4a]/20 rounded-lg p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-semibold text-[#1e4d4a]">Run #3</span>
                      <span className="text-xs text-[#7a7568]">Replay</span>
                    </div>
                    <span className="text-xs text-[#7a7568]">Apr 4, 14:38</span>
                  </div>
                  <div className="grid grid-cols-3 gap-2 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Result</span>
                      <span className="font-mono text-[#1f1c17]">30</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Time</span>
                      <span className="font-mono text-[#1f1c17]">1.1ms</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Policy</span>
                      <span className="font-mono text-[#1f1c17]">Det</span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Run 4 */}
              <div className="relative pb-6">
                <div className="absolute left-0 top-2 w-4 h-4 rounded-full bg-[#1e4d4a] border-2 border-white shadow-md z-10" />
                <div className="absolute left-[7px] top-6 bottom-0 w-0.5 bg-[#1e4d4a]/20" />
                <div className="ml-8 bg-white border border-[#1e4d4a]/20 rounded-lg p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-semibold text-[#1e4d4a]">Run #4</span>
                      <span className="text-xs text-[#7a7568]">Replay</span>
                    </div>
                    <span className="text-xs text-[#7a7568]">Apr 4, 14:42</span>
                  </div>
                  <div className="grid grid-cols-3 gap-2 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Result</span>
                      <span className="font-mono text-[#1f1c17]">30</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Time</span>
                      <span className="font-mono text-[#1f1c17]">1.5ms</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Policy</span>
                      <span className="font-mono text-[#1f1c17]">Det</span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Run 5 - Latest */}
              <div className="relative">
                <div className="absolute left-0 top-2 w-4 h-4 rounded-full bg-[#b84532] border-2 border-white shadow-lg z-10 animate-pulse" />
                <div className="ml-8 bg-gradient-to-br from-[#b84532]/5 to-[#b84532]/10 border-2 border-[#b84532]/30 rounded-lg p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-semibold text-[#b84532]">Run #5</span>
                      <span className="px-2 py-0.5 bg-[#b84532] text-white rounded text-[10px] font-semibold">LATEST</span>
                    </div>
                    <span className="text-xs text-[#7a7568]">Apr 4, 14:45</span>
                  </div>
                  <div className="grid grid-cols-3 gap-2 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Result</span>
                      <span className="font-mono text-[#1f1c17]">30</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Time</span>
                      <span className="font-mono text-[#1f1c17]">1.2ms</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-[#7a7568]">Policy</span>
                      <span className="font-mono text-[#1f1c17]">Det</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Observation Envelope */}
        <section>
          <div className="flex items-center gap-2 mb-4">
            <div className="w-1 h-6 bg-[#b84532] rounded-full" />
            <h3 className="text-base font-semibold text-[#1f1c17]">Observation Envelope</h3>
          </div>

          <div className="bg-[#faf7f1] border border-[#1f1c17]/10 rounded-lg p-4">
            <div className="text-sm font-semibold text-[#1f1c17] mb-3">Captured State</div>
            <div className="grid grid-cols-2 gap-3">
              <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
                <div className="text-xs text-[#7a7568] mb-1">Formula Hash</div>
                <div className="font-mono text-xs text-[#1f1c17]">a4f2c8e9d3b1</div>
              </div>
              <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
                <div className="text-xs text-[#7a7568] mb-1">Host Profile</div>
                <div className="font-mono text-xs text-[#1f1c17]">OC-H0</div>
              </div>
              <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
                <div className="text-xs text-[#7a7568] mb-1">OxFml Version</div>
                <div className="font-mono text-xs text-[#1f1c17]">v0.12.4</div>
              </div>
              <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
                <div className="text-xs text-[#7a7568] mb-1">Scenario Policy</div>
                <div className="font-mono text-xs text-[#1f1c17]">Deterministic</div>
              </div>
              <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
                <div className="text-xs text-[#7a7568] mb-1">Frozen Timestamp</div>
                <div className="font-mono text-xs text-[#1f1c17]">2026-04-04 14:32</div>
              </div>
              <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
                <div className="text-xs text-[#7a7568] mb-1">Run Count</div>
                <div className="font-mono text-xs text-[#1f1c17]">5</div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
