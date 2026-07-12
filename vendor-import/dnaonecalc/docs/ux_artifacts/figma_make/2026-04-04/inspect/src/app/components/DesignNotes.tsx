export function DesignNotes() {
  return (
    <div className="max-w-4xl mx-auto p-8 space-y-8">
      <div>
        <h2 className="text-2xl font-semibold text-[#2a2a2a] mb-4">Design System Notes</h2>
        <p className="text-[#6a6a6a] leading-relaxed">
          DNA OneCalc uses a warm, editorial palette inspired by parchment, sand, smoke, and ink,
          with accent colors from deep teal, terracotta, amber, moss, and muted rust families.
        </p>
      </div>

      <div className="grid grid-cols-5 gap-4">
        <div className="space-y-2">
          <div className="w-full h-20 rounded-lg bg-[#2d5f5d]" />
          <div className="text-sm text-[#2a2a2a]">Deep Teal</div>
          <div className="text-xs text-[#6a6a6a] font-mono">#2d5f5d</div>
        </div>
        <div className="space-y-2">
          <div className="w-full h-20 rounded-lg bg-[#c65d47]" />
          <div className="text-sm text-[#2a2a2a]">Terracotta</div>
          <div className="text-xs text-[#6a6a6a] font-mono">#c65d47</div>
        </div>
        <div className="space-y-2">
          <div className="w-full h-20 rounded-lg bg-[#d69f4c]" />
          <div className="text-sm text-[#2a2a2a]">Amber</div>
          <div className="text-xs text-[#6a6a6a] font-mono">#d69f4c</div>
        </div>
        <div className="space-y-2">
          <div className="w-full h-20 rounded-lg bg-[#5a6f4d]" />
          <div className="text-sm text-[#2a2a2a]">Moss</div>
          <div className="text-xs text-[#6a6a6a] font-mono">#5a6f4d</div>
        </div>
        <div className="space-y-2">
          <div className="w-full h-20 rounded-lg bg-[#a75842]" />
          <div className="text-sm text-[#2a2a2a]">Muted Rust</div>
          <div className="text-xs text-[#6a6a6a] font-mono">#a75842</div>
        </div>
      </div>

      <div className="space-y-4">
        <h3 className="text-xl font-semibold text-[#2a2a2a]">Core Design Principles</h3>
        
        <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
          <h4 className="font-medium text-[#2a2a2a] mb-2">Explorer First</h4>
          <p className="text-sm text-[#6a6a6a]">
            The primary task is authoring and understanding one formula quickly and comfortably.
            The formula editor is a first-class product surface, not an afterthought.
          </p>
        </div>

        <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
          <h4 className="font-medium text-[#2a2a2a] mb-2">Support Surfaces Stay Subordinate</h4>
          <p className="text-sm text-[#6a6a6a]">
            X-Ray, capability, replay, and comparison surfaces must support the main task
            rather than obscure it. The current result must remain visible while support surfaces are open.
          </p>
        </div>

        <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
          <h4 className="font-medium text-[#2a2a2a] mb-2">Evidence, Not Theater</h4>
          <p className="text-sm text-[#6a6a6a]">
            Reliability, lossiness, blocked dimensions, and capability limits must be visible and plain.
            The workbench should feel like an evidence workbench, not an error panel.
          </p>
        </div>

        <div className="bg-[#f5f1e8] border border-[#2a2a2a]/10 rounded-lg p-6">
          <h4 className="font-medium text-[#2a2a2a] mb-2">One Scenario, Many Perspectives</h4>
          <p className="text-sm text-[#6a6a6a]">
            Explorer, X-Ray, compare, witness, and handoff should feel like coordinated views
            over the same scenario, not unrelated screens.
          </p>
        </div>
      </div>
    </div>
  );
}
