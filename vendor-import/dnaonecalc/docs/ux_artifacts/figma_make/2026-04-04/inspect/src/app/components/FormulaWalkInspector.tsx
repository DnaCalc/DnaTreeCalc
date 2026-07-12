import { useState } from 'react';
import { ChevronRight, ChevronDown, CircleDot, CheckCircle2, MinusCircle } from 'lucide-react';

interface TreeNode {
  id: string;
  label: string;
  type: 'function' | 'binding' | 'value' | 'reference';
  value?: string;
  status: 'evaluated' | 'bound' | 'opaque' | 'blocked';
  children?: TreeNode[];
}

const sampleFormulaTree: TreeNode[] = [
  {
    id: 'let',
    label: 'LET',
    type: 'function',
    status: 'evaluated',
    value: '30',
    children: [
      {
        id: 'binding-1',
        label: 'values',
        type: 'binding',
        status: 'bound',
        children: [
          {
            id: 'seq',
            label: 'SEQUENCE(10, 1, 1, 1)',
            type: 'function',
            status: 'evaluated',
            value: '[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]',
          },
        ],
      },
      {
        id: 'binding-2',
        label: 'filtered',
        type: 'binding',
        status: 'bound',
        children: [
          {
            id: 'filter',
            label: 'FILTER',
            type: 'function',
            status: 'evaluated',
            value: '[2, 4, 6, 8, 10]',
            children: [
              {
                id: 'filter-ref',
                label: 'values',
                type: 'reference',
                status: 'bound',
                value: '[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]',
              },
              {
                id: 'filter-pred',
                label: 'MOD(values, 2) = 0',
                type: 'function',
                status: 'evaluated',
                value: 'predicate',
              },
            ],
          },
        ],
      },
      {
        id: 'body',
        label: 'SUM(filtered)',
        type: 'function',
        status: 'evaluated',
        value: '30',
        children: [
          {
            id: 'sum-ref',
            label: 'filtered',
            type: 'reference',
            status: 'bound',
            value: '[2, 4, 6, 8, 10]',
          },
        ],
      },
    ],
  },
];

function TreeNodeComponent({ node, depth = 0 }: { node: TreeNode; depth?: number }) {
  const [expanded, setExpanded] = useState(true);
  const hasChildren = node.children && node.children.length > 0;

  const getStatusIcon = () => {
    switch (node.status) {
      case 'evaluated':
        return <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />;
      case 'bound':
        return <CircleDot className="w-3.5 h-3.5 text-[#c88d2e]" />;
      case 'opaque':
        return <MinusCircle className="w-3.5 h-3.5 text-[#7a7568]" />;
      case 'blocked':
        return <MinusCircle className="w-3.5 h-3.5 text-[#b84532]" />;
    }
  };

  const getTypeColor = () => {
    switch (node.type) {
      case 'function':
        return 'text-[#1e4d4a]';
      case 'binding':
        return 'text-[#3e5238]';
      case 'value':
        return 'text-[#c88d2e]';
      case 'reference':
        return 'text-[#b84532]';
    }
  };

  const getStatusBadge = () => {
    switch (node.status) {
      case 'evaluated':
        return <span className="px-2 py-0.5 bg-[#1e4d4a]/10 text-[#1e4d4a] text-xs font-medium rounded">Evaluated</span>;
      case 'bound':
        return <span className="px-2 py-0.5 bg-[#c88d2e]/10 text-[#c88d2e] text-xs font-medium rounded">Bound</span>;
      case 'opaque':
        return <span className="px-2 py-0.5 bg-[#7a7568]/10 text-[#7a7568] text-xs font-medium rounded">Opaque</span>;
      case 'blocked':
        return <span className="px-2 py-0.5 bg-[#b84532]/10 text-[#b84532] text-xs font-medium rounded">Blocked</span>;
    }
  };

  return (
    <div className="font-mono text-sm">
      <div
        className={`flex items-start gap-2 py-1.5 px-2 rounded hover:bg-[#f7f3ea] transition-colors cursor-pointer group`}
        style={{ paddingLeft: `${depth * 20 + 8}px` }}
        onClick={() => hasChildren && setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2 flex-1 min-w-0">
          {hasChildren ? (
            expanded ? (
              <ChevronDown className="w-4 h-4 text-[#7a7568] flex-shrink-0" />
            ) : (
              <ChevronRight className="w-4 h-4 text-[#7a7568] flex-shrink-0" />
            )
          ) : (
            <div className="w-4" />
          )}
          
          {getStatusIcon()}
          
          <span className={`${getTypeColor()} font-medium truncate`}>
            {node.label}
          </span>

          {node.value && (
            <span className="text-[#7a7568] truncate flex-shrink-0">
              → <span className="text-[#1f1c17]">{node.value}</span>
            </span>
          )}
        </div>

        <div className="opacity-0 group-hover:opacity-100 transition-opacity">
          {getStatusBadge()}
        </div>
      </div>

      {hasChildren && expanded && (
        <div>
          {node.children!.map((child) => (
            <TreeNodeComponent key={child.id} node={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
}

export function FormulaWalkInspector() {
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-sm font-semibold text-[#1f1c17] mb-1">Formula Walk</h3>
        <p className="text-xs text-[#7a7568]">
          Structured partial evaluation showing subexpressions, bound names, and intermediate values
        </p>
      </div>

      <div className="bg-white border border-[#1f1c17]/15 rounded-lg p-3">
        {sampleFormulaTree.map((node) => (
          <TreeNodeComponent key={node.id} node={node} />
        ))}
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs">
        <div className="flex items-center gap-2 p-2 bg-[#1e4d4a]/5 rounded">
          <CheckCircle2 className="w-3.5 h-3.5 text-[#1e4d4a]" />
          <span className="text-[#7a7568]">Evaluated</span>
        </div>
        <div className="flex items-center gap-2 p-2 bg-[#c88d2e]/5 rounded">
          <CircleDot className="w-3.5 h-3.5 text-[#c88d2e]" />
          <span className="text-[#7a7568]">Bound</span>
        </div>
        <div className="flex items-center gap-2 p-2 bg-[#7a7568]/5 rounded">
          <MinusCircle className="w-3.5 h-3.5 text-[#7a7568]" />
          <span className="text-[#7a7568]">Opaque</span>
        </div>
        <div className="flex items-center gap-2 p-2 bg-[#b84532]/5 rounded">
          <MinusCircle className="w-3.5 h-3.5 text-[#b84532]" />
          <span className="text-[#7a7568]">Blocked</span>
        </div>
      </div>
    </div>
  );
}
