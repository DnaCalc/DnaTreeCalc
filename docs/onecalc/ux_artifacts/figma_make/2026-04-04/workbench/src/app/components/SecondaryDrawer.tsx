import { X, List, BookOpen, Palette, Code, Flag, Package, FileCheck } from 'lucide-react';

type DrawerType = 'completions' | 'help' | 'formatting' | 'details' | 'flags' | 'evidence' | 'envelope' | null;

interface SecondaryDrawerProps {
  type: DrawerType;
  onClose: () => void;
}

export function SecondaryDrawer({ type, onClose }: SecondaryDrawerProps) {
  if (!type) return null;

  return (
    <aside className="w-[400px] border-l border-[#1f1c17]/10 bg-[#f7f3ea] overflow-y-auto flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-0 bg-[#ede7da] border-b border-[#1f1c17]/10 p-4 z-10">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {type === 'completions' && <List className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'help' && <BookOpen className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'formatting' && <Palette className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'details' && <Code className="w-5 h-5 text-[#3e5238]" />}
            {type === 'flags' && <Flag className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'evidence' && <Package className="w-5 h-5 text-[#c88d2e]" />}
            {type === 'envelope' && <FileCheck className="w-5 h-5 text-[#b84532]" />}
            <h3 className="text-sm font-semibold text-[#1f1c17]">
              {type === 'completions' && 'Function Completions'}
              {type === 'help' && 'Function Help'}
              {type === 'formatting' && 'Formatting Editor'}
              {type === 'details' && 'Detailed Inspection'}
              {type === 'flags' && 'Function Flags'}
              {type === 'evidence' && 'Evidence Bundle'}
              {type === 'envelope' && 'Observation Envelope'}
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
          Secondary detail drawer • Press Esc to close
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-4">
        {type === 'completions' && <CompletionsContent />}
        {type === 'help' && <HelpContent />}
        {type === 'formatting' && <FormattingContent />}
        {type === 'details' && <DetailsContent />}
        {type === 'flags' && <FlagsContent />}
        {type === 'evidence' && <EvidenceContent />}
        {type === 'envelope' && <EnvelopeContent />}
      </div>
    </aside>
  );
}

function CompletionsContent() {
  const functions = ['ABS', 'ACCRINT', 'ACOS', 'AND', 'AVERAGE', 'CHOOSE', 'CLEAN', 'CODE', 'COLUMN', 'CONCAT', 'COUNT', 'COUNTA', 'COUNTIF', 'DATE', 'DAY', 'EVEN', 'FILTER', 'FIND', 'IF', 'IFERROR', 'INDEX', 'INT', 'ISBLANK', 'LET', 'LOWER', 'MATCH', 'MAX', 'MID', 'MIN', 'MOD', 'NOT', 'NOW', 'OR', 'RAND', 'ROUND', 'SEARCH', 'SEQUENCE', 'SUM', 'SUMIF', 'TEXT', 'TODAY', 'TRIM', 'UPPER', 'VLOOKUP', 'XLOOKUP'];

  return (
    <div className="space-y-2">
      <div className="p-3 bg-white border border-[#1f1c17]/15 rounded-lg">
        <input
          type="text"
          placeholder="Search functions..."
          className="w-full text-sm bg-transparent focus:outline-none text-[#1f1c17] placeholder:text-[#7a7568]"
        />
      </div>
      <div className="text-xs text-[#7a7568] px-1">
        517 functions available in OC-H0
      </div>
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg divide-y divide-[#1f1c17]/5 max-h-[600px] overflow-y-auto">
        {functions.map((func) => (
          <button
            key={func}
            className="w-full px-3 py-2 text-left hover:bg-[#f7f3ea] transition-colors flex items-center justify-between"
          >
            <span className="text-sm font-mono text-[#1e4d4a] font-semibold">{func}</span>
            <span className="text-xs text-[#7a7568]">→</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function HelpContent() {
  return (
    <div className="space-y-4">
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
        <div className="font-mono text-lg text-[#1e4d4a] font-semibold mb-2">LET</div>
        <div className="font-mono text-sm text-[#7a7568] mb-3">
          LET(name1, value1, [name2, value2, ...], calculation)
        </div>
        <div className="text-sm text-[#1f1c17] leading-relaxed mb-4">
          Assigns names to calculation results to allow storing intermediate calculations, values, and names inside a formula. This makes complex formulas easier to read and maintain.
        </div>
        
        <div className="pt-3 border-t border-[#1f1c17]/10">
          <div className="text-xs font-semibold text-[#1f1c17] mb-3">Arguments</div>
          <div className="space-y-3 text-sm">
            <div>
              <div className="font-mono text-[#1e4d4a] font-semibold mb-1">name1</div>
              <div className="text-[#7a7568]">Required. The name to assign. Must be a valid variable name.</div>
            </div>
            <div>
              <div className="font-mono text-[#c88d2e] font-semibold mb-1">value1</div>
              <div className="text-[#7a7568]">Required. The value to assign to name1. Can be any valid expression.</div>
            </div>
            <div>
              <div className="font-mono text-[#b84532] font-semibold mb-1">calculation</div>
              <div className="text-[#7a7568]">Required. The final calculation that can reference the assigned names.</div>
            </div>
          </div>
        </div>

        <div className="pt-3 border-t border-[#1f1c17]/10 mt-4">
          <div className="text-xs font-semibold text-[#1f1c17] mb-2">Metadata</div>
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="flex items-center justify-between">
              <span className="text-[#7a7568]">Category</span>
              <span className="text-[#1f1c17] font-medium">Logical</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[#7a7568]">Status</span>
              <span className="px-2 py-0.5 bg-[#1e4d4a] text-white rounded font-medium">Supported</span>
            </div>
          </div>
        </div>
      </div>

      <div className="bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10 border border-[#c88d2e]/20 rounded-lg p-4">
        <div className="text-xs font-semibold text-[#1f1c17] mb-2">Example</div>
        <div className="font-mono text-xs text-[#1f1c17] bg-white p-3 rounded border border-[#1f1c17]/10">
          =LET(<br />
          &nbsp;&nbsp;x, 5,<br />
          &nbsp;&nbsp;y, 10,<br />
          &nbsp;&nbsp;x + y<br />
          )
        </div>
      </div>
    </div>
  );
}

function FormattingContent() {
  return (
    <div className="space-y-4">
      {/* Number Format */}
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
        <div className="text-sm font-semibold text-[#1f1c17] mb-3">Number Format</div>
        <div className="space-y-3">
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Format Code</label>
            <input
              type="text"
              defaultValue="General"
              className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded focus:outline-none focus:ring-2 focus:ring-[#1e4d4a]"
            />
          </div>
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Decimal Places</label>
            <select className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded focus:outline-none focus:ring-2 focus:ring-[#1e4d4a]">
              <option>Auto</option>
              <option>0</option>
              <option>1</option>
              <option>2</option>
              <option>3</option>
              <option>4</option>
            </select>
          </div>
        </div>
      </div>

      {/* Conditional Formatting */}
      <div className="bg-white border border-[#c88d2e]/20 rounded-lg p-4">
        <div className="text-sm font-semibold text-[#1f1c17] mb-3">Conditional Formatting</div>
        <div className="space-y-2">
          <label className="flex items-center gap-2 text-sm cursor-pointer">
            <input type="checkbox" className="rounded border-[#1f1c17]/20" />
            <span className="text-[#1f1c17]">Result color by value</span>
          </label>
          <label className="flex items-center gap-2 text-sm cursor-pointer">
            <input type="checkbox" className="rounded border-[#1f1c17]/20" />
            <span className="text-[#1f1c17]">Icon set</span>
          </label>
          <label className="flex items-center gap-2 text-sm cursor-pointer">
            <input type="checkbox" defaultChecked className="rounded border-[#1f1c17]/20" />
            <span className="text-[#1f1c17]">Data bars</span>
          </label>
          <label className="flex items-center gap-2 text-sm cursor-pointer">
            <input type="checkbox" className="rounded border-[#1f1c17]/20" />
            <span className="text-[#1f1c17]">Color scale</span>
          </label>
        </div>
      </div>

      {/* Style */}
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
        <div className="text-sm font-semibold text-[#1f1c17] mb-3">Style</div>
        <div className="space-y-3">
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Font Weight</label>
            <select className="w-full px-3 py-2 text-sm border border-[#1f1c17]/15 rounded focus:outline-none focus:ring-2 focus:ring-[#1e4d4a]">
              <option>Default</option>
              <option>Bold</option>
              <option>Light</option>
            </select>
          </div>
          <div>
            <label className="text-xs text-[#7a7568] mb-1.5 block">Text Color</label>
            <input
              type="color"
              defaultValue="#1f1c17"
              className="w-full h-10 border border-[#1f1c17]/15 rounded cursor-pointer"
            />
          </div>
        </div>
      </div>
    </div>
  );
}

function DetailsContent() {
  return (
    <div className="space-y-3 font-mono text-xs">
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
        <div className="text-[#7a7568] mb-2">snapshot_id:</div>
        <div className="text-[#1f1c17] break-all text-[10px]">
          capability-snapshot-oc-h0-edit-accept-recalc-t75244163946
        </div>
      </div>
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
        <div className="text-[#7a7568] mb-1">Diagnostics:</div>
        <div className="space-y-1 text-[#1f1c17]">
          <div>buffer_len: 11</div>
          <div>cursor_index: 11</div>
          <div>selection: 11..11</div>
        </div>
      </div>
    </div>
  );
}

function FlagsContent() {
  return (
    <div className="space-y-3">
      {['LET', 'SEQUENCE', 'FILTER', 'SUM', 'MOD', 'AVERAGE', 'IF', 'INDEX', 'MATCH', 'VLOOKUP'].map((func) => (
        <div key={func} className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
          <div className="flex items-center justify-between mb-2">
            <span className="font-mono text-sm font-semibold text-[#1e4d4a]">{func}</span>
            <span className="px-2 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] text-xs rounded font-medium">
              Supported
            </span>
          </div>
          <div className="text-xs text-[#7a7568]">
            Category: {func === 'LET' || func === 'IF' ? 'Logical' : func === 'SUM' || func === 'AVERAGE' ? 'Math' : 'Lookup'}
          </div>
        </div>
      ))}
    </div>
  );
}

function EvidenceContent() {
  return (
    <div className="space-y-4">
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
        <div className="text-sm font-semibold text-[#1f1c17] mb-3">Bundle Metadata</div>
        <div className="space-y-2 font-mono text-xs">
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Created:</span>
            <span className="text-[#1f1c17]">2026-04-03 14:32</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Size:</span>
            <span className="text-[#1f1c17]">2.4 KB</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Format:</span>
            <span className="text-[#1f1c17]">v1.0</span>
          </div>
        </div>
      </div>
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
        <div className="text-sm font-semibold text-[#1f1c17] mb-3">Contents</div>
        <div className="space-y-1 text-xs">
          <div className="p-2 bg-[#f7f3ea] rounded text-[#1f1c17]">Formula text</div>
          <div className="p-2 bg-[#f7f3ea] rounded text-[#1f1c17]">DNA result</div>
          <div className="p-2 bg-[#f7f3ea] rounded text-[#1f1c17]">Excel observation</div>
          <div className="p-2 bg-[#f7f3ea] rounded text-[#1f1c17]">Comparison envelope</div>
          <div className="p-2 bg-[#f7f3ea] rounded text-[#1f1c17]">Timing data</div>
        </div>
      </div>
    </div>
  );
}

function EnvelopeContent() {
  return (
    <div className="space-y-4">
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
        <div className="text-sm font-semibold text-[#1f1c17] mb-3">Envelope Summary</div>
        <div className="space-y-2 text-sm">
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Coverage:</span>
            <span className="font-semibold text-[#1e4d4a]">Full</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Dimensions:</span>
            <span className="font-mono font-semibold text-[#1f1c17]">7</span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[#7a7568]">Blocked:</span>
            <span className="font-mono font-semibold text-[#1f1c17]">0</span>
          </div>
        </div>
      </div>
      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-4">
        <div className="text-sm font-semibold text-[#1f1c17] mb-3">All Dimensions</div>
        <div className="space-y-1 text-xs">
          {['result_value', 'result_type', 'result_shape', 'display_text', 'formatting', 'color', 'style'].map((dim) => (
            <div key={dim} className="flex items-center gap-2 p-2 bg-[#f7f3ea] rounded">
              <div className="w-2 h-2 rounded-full bg-[#1e4d4a]" />
              <span className="font-mono text-[#1f1c17]">{dim}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
