import { useState } from 'react';
import { ChevronRight, ChevronDown, Circle, CheckCircle2, Lock, AlertCircle, Info } from 'lucide-react';

interface WalkNode {
  id: string;
  expression: string;
  state: 'evaluated' | 'bound' | 'opaque' | 'blocked';
  value?: string;
  type?: string;
  children?: WalkNode[];
  description?: string;
  reason?: string;
}

interface FormulaWalkPanelProps {
  onSelectNode: (nodeId: string) => void;
}

export function FormulaWalkPanel({ onSelectNode }: FormulaWalkPanelProps) {
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set(['root', 'let', 'sum']));

  const walkData: WalkNode = {
    id: 'root',
    expression: '=LET(...)',
    state: 'evaluated',
    value: '30',
    type: 'Number',
    description: 'Root expression',
    children: [
      {
        id: 'let',
        expression: 'LET(values, SEQUENCE(...), filtered, FILTER(...), SUM(...))',
        state: 'evaluated',
        value: '30',
        type: 'Number',
        description: 'LET binding with named variables',
        children: [
          {
            id: 'values-name',
            expression: 'values',
            state: 'bound',
            value: 'SEQUENCE(10, 1, 1, 1)',
            description: 'Name binding: values',
          },
          {
            id: 'values-value',
            expression: 'SEQUENCE(10, 1, 1, 1)',
            state: 'evaluated',
            value: '[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]',
            type: 'Array[10]',
            description: 'Sequence generation',
          },
          {
            id: 'filtered-name',
            expression: 'filtered',
            state: 'bound',
            value: 'FILTER(...)',
            description: 'Name binding: filtered',
          },
          {
            id: 'filtered-value',
            expression: 'FILTER(values, MOD(values, 2) = 0)',
            state: 'evaluated',
            value: '[2, 4, 6, 8, 10]',
            type: 'Array[5]',
            description: 'Filter even numbers',
            children: [
              {
                id: 'filter-array',
                expression: 'values',
                state: 'evaluated',
                value: '[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]',
                type: 'Array[10]',
                description: 'Resolved from bound name',
              },
              {
                id: 'filter-condition',
                expression: 'MOD(values, 2) = 0',
                state: 'evaluated',
                value: '[FALSE, TRUE, FALSE, TRUE, ...]',
                type: 'Array[10]',
                description: 'Row-wise condition evaluation',
              },
            ],
          },
          {
            id: 'sum',
            expression: 'SUM(filtered)',
            state: 'evaluated',
            value: '30',
            type: 'Number',
            description: 'Sum of filtered values',
            children: [
              {
                id: 'sum-arg',
                expression: 'filtered',
                state: 'evaluated',
                value: '[2, 4, 6, 8, 10]',
                type: 'Array[5]',
                description: 'Resolved from bound name',
              },
            ],
          },
        ],
      },
    ],
  };

  const toggleNode = (nodeId: string) => {
    const newExpanded = new Set(expandedNodes);
    if (newExpanded.has(nodeId)) {
      newExpanded.delete(nodeId);
    } else {
      newExpanded.add(nodeId);
    }
    setExpandedNodes(newExpanded);
  };

  const getStateConfig = (state: WalkNode['state']) => {
    switch (state) {
      case 'evaluated':
        return {
          icon: CheckCircle2,
          color: 'text-[#1e4d4a]',
          bg: 'bg-[#1e4d4a]/5',
          border: 'border-[#1e4d4a]/20',
          label: 'Evaluated',
        };
      case 'bound':
        return {
          icon: Circle,
          color: 'text-[#c88d2e]',
          bg: 'bg-[#c88d2e]/5',
          border: 'border-[#c88d2e]/20',
          label: 'Bound',
        };
      case 'opaque':
        return {
          icon: Lock,
          color: 'text-[#7a7568]',
          bg: 'bg-[#7a7568]/5',
          border: 'border-[#7a7568]/20',
          label: 'Opaque',
        };
      case 'blocked':
        return {
          icon: AlertCircle,
          color: 'text-[#b84532]',
          bg: 'bg-[#b84532]/5',
          border: 'border-[#b84532]/20',
          label: 'Blocked',
        };
    }
  };

  const renderNode = (node: WalkNode, depth: number = 0) => {
    const isExpanded = expandedNodes.has(node.id);
    const hasChildren = node.children && node.children.length > 0;
    const stateConfig = getStateConfig(node.state);
    const StateIcon = stateConfig.icon;

    return (
      <div key={node.id} className="select-none">
        {/* Node Row */}
        <div
          className={`group flex items-start gap-3 py-2.5 px-3 hover:bg-white rounded-lg cursor-pointer transition-colors ${
            depth === 0 ? 'bg-white border border-[#1f1c17]/10' : ''
          }`}
          style={{ marginLeft: `${depth * 24}px` }}
          onClick={() => {
            if (hasChildren) {
              toggleNode(node.id);
            }
            onSelectNode(node.id);
          }}
        >
          {/* Expand/Collapse Icon */}
          <div className="w-4 h-4 flex items-center justify-center flex-shrink-0 mt-0.5">
            {hasChildren ? (
              isExpanded ? (
                <ChevronDown className="w-4 h-4 text-[#7a7568]" />
              ) : (
                <ChevronRight className="w-4 h-4 text-[#7a7568]" />
              )
            ) : (
              <div className="w-1 h-1 rounded-full bg-[#7a7568]/30" />
            )}
          </div>

          {/* State Badge */}
          <div className={`flex items-center gap-1.5 px-2 py-1 rounded ${stateConfig.bg} ${stateConfig.border} border flex-shrink-0`}>
            <StateIcon className={`w-3 h-3 ${stateConfig.color}`} />
            <span className={`text-[10px] font-semibold uppercase tracking-wide ${stateConfig.color}`}>
              {stateConfig.label}
            </span>
          </div>

          {/* Expression & Value */}
          <div className="flex-1 min-w-0">
            <div className="flex items-start gap-3 mb-1">
              <code className="font-mono text-xs text-[#1f1c17] break-all flex-1">
                {node.expression}
              </code>
              {node.value && (
                <code className={`font-mono text-xs font-semibold flex-shrink-0 ${stateConfig.color}`}>
                  → {node.value}
                </code>
              )}
            </div>
            {node.description && (
              <div className="text-xs text-[#7a7568] mb-1">
                {node.description}
              </div>
            )}
            {node.type && (
              <div className="text-[10px] text-[#7a7568] font-medium">
                Type: {node.type}
              </div>
            )}
            {node.reason && (
              <div className="flex items-start gap-1.5 mt-2 p-2 bg-[#b84532]/5 border border-[#b84532]/20 rounded text-xs text-[#b84532]">
                <Info className="w-3 h-3 flex-shrink-0 mt-0.5" />
                <span>{node.reason}</span>
              </div>
            )}
          </div>
        </div>

        {/* Children */}
        {hasChildren && isExpanded && (
          <div className="mt-1">
            {node.children!.map((child) => renderNode(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="flex flex-col h-full bg-[#faf7f1]">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-[#f7f3ea]">
        <h2 className="text-sm font-semibold text-[#1f1c17] mb-2">Formula Walk</h2>
        <div className="text-xs text-[#7a7568] leading-relaxed">
          Tree-aligned partial evaluation showing subexpressions, bindings, and intermediate values
        </div>
      </div>

      {/* State Legend */}
      <div className="px-4 py-3 border-b border-[#1f1c17]/10 bg-white">
        <div className="flex items-center gap-4 text-xs">
          <div className="flex items-center gap-1.5">
            <CheckCircle2 className="w-3 h-3 text-[#1e4d4a]" />
            <span className="text-[#7a7568]">Evaluated</span>
          </div>
          <div className="flex items-center gap-1.5">
            <Circle className="w-3 h-3 text-[#c88d2e]" />
            <span className="text-[#7a7568]">Bound</span>
          </div>
          <div className="flex items-center gap-1.5">
            <Lock className="w-3 h-3 text-[#7a7568]" />
            <span className="text-[#7a7568]">Opaque</span>
          </div>
          <div className="flex items-center gap-1.5">
            <AlertCircle className="w-3 h-3 text-[#b84532]" />
            <span className="text-[#7a7568]">Blocked</span>
          </div>
        </div>
      </div>

      {/* Walk Tree */}
      <div className="flex-1 overflow-y-auto p-4">
        <div className="space-y-2">
          {renderNode(walkData)}
        </div>
      </div>
    </div>
  );
}
