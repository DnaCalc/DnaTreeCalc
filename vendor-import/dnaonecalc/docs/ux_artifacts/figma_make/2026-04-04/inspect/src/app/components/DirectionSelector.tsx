import { Link } from 'react-router';
import { Sparkles, Target, Grid3x3, Flame, Database, Layout, Maximize2, Eye } from 'lucide-react';

export function DirectionSelector() {
  const directions = [
    {
      id: 'explore',
      path: '/explore',
      title: 'Focused Explore Mode',
      description: 'Refined single-mode layout optimized for formula authoring and function discovery. Formula editor, result, and completions/help visible together. Diagnostics stay close to the editor. Secondary controls (formatting, scenario flags) in right drawer.',
      icon: Maximize2,
      color: 'from-[#1e4d4a] to-[#c88d2e]',
      featured: true,
    },
    {
      id: 'inspect',
      path: '/inspect',
      title: 'Focused Inspect Mode',
      description: 'Semantic inspection environment with Formula Walk as primary surface. Tree-aligned partial evaluation showing subexpressions, bound names, intermediate values, and state categories (evaluated, bound, opaque, blocked). Parse, bind, and eval summaries nearby. Provenance and context in right drawer.',
      icon: Eye,
      color: 'from-[#3e5238] to-[#c88d2e]',
      featured: true,
    },
    {
      id: 'architecture',
      path: '/architecture',
      title: 'Information Architecture (Three-Mode Shell)',
      description: 'Definitive information architecture with three explicit task modes: Explore (formula editing), Inspect (semantic analysis), and Workbench (comparison/evidence). Clear ownership model across workspace, formula-space, run, and comparison levels. Mode-specific layouts with right drawer for secondary details.',
      icon: Layout,
      color: 'from-[#1e4d4a] to-[#b84532]',
      featured: true,
    },
    {
      id: 'refined',
      path: '/refined',
      title: 'Refined Warm Editorial',
      description: 'The definitive DNA OneCalc experience. Premium formula editor, two-level X-Ray inspector with Formula Walk, and comprehensive evidence workbench. Richer palette with oxidized teal, terracotta, amber brass, and moss.',
      icon: Flame,
      color: 'from-[#1e4d4a] to-[#3e5238]',
      featured: true,
    },
    {
      id: 'dense',
      path: '/dense',
      title: 'Dense Information Mode',
      description: 'High information density view with all technical details. Collapsible sections, editable fields, progressive disclosure of diagnostics, capability center, and dependency ledger. 4-column layout with completions, help, and formula walk always visible.',
      icon: Database,
      color: 'from-[#3e5238] to-[#c88d2e]',
      featured: false,
    },
    {
      id: 'warm-editorial',
      path: '/warm-editorial',
      title: 'Warm Editorial Rail-and-Tabs Workbench',
      description: 'Classic workbench layout with warm, inviting tones. Features persistent rail navigation, clear tab organization, and an editorial feel that balances technical precision with approachability.',
      icon: Sparkles,
      color: 'from-[#2d5f5d] to-[#5a6f4d]',
      featured: false,
    },
    {
      id: 'analytical-compare',
      path: '/analytical-compare',
      title: 'Analytical Compare Studio',
      description: 'Evidence-first interface designed for deep comparison and replay analysis. Emphasizes structured data presentation, detailed diagnostics, and systematic evidence inspection.',
      icon: Target,
      color: 'from-[#c65d47] to-[#d69f4c]',
      featured: false,
    },
    {
      id: 'modular-evidence',
      path: '/modular-evidence',
      title: 'Modular Evidence Cockpit',
      description: 'Flexible, panel-based interface optimized for power users. Features modular layout customization, dense information display, and advanced workbench controls.',
      icon: Grid3x3,
      color: 'from-[#a75842] to-[#2d5f5d]',
      featured: false,
    },
  ];

  return (
    <div className="min-h-screen bg-[#faf7f1] p-8">
      <div className="max-w-7xl mx-auto">
        <header className="mb-12">
          <h1 className="text-5xl font-semibold text-[#1f1c17] mb-3 tracking-tight">
            DNA OneCalc
          </h1>
          <p className="text-xl text-[#7a7568] max-w-3xl">
            A serious single-formula calculation host. Explore the three-mode information architecture,
            refined design direction, and dense information mode, plus original explorations.
          </p>
        </header>

        <div className="space-y-6">
          {/* Featured Directions */}
          <div className="grid grid-cols-1 gap-6">
            {directions.filter(d => d.featured).map((direction) => {
              const Icon = direction.icon;
              return (
                <Link
                  key={direction.id}
                  to={direction.path}
                  className="group relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#1e4d4a] to-[#3e5238] p-8 transition-all duration-300 hover:shadow-2xl hover:scale-[1.01] border-2 border-[#1e4d4a]/30"
                  style={{
                    background: `linear-gradient(to bottom right, ${direction.color.replace('from-', '').replace(' to-', ', ')})`
                  }}
                >
                  <div className="relative z-10">
                    <div className="flex items-start justify-between mb-4">
                      <div className="flex items-center gap-4">
                        <div className="w-16 h-16 rounded-xl bg-white/20 backdrop-blur-sm flex items-center justify-center shadow-lg">
                          <Icon className="w-9 h-9 text-white" />
                        </div>
                        <div>
                          <div className="px-3 py-1 bg-[#c88d2e] text-white text-xs font-semibold rounded-full mb-2 inline-block">
                            {direction.id === 'architecture' ? 'PRIMARY' : 'RECOMMENDED'}
                          </div>
                          <h2 className="text-3xl font-semibold text-white mb-2 leading-tight">
                            {direction.title}
                          </h2>
                        </div>
                      </div>
                    </div>
                    
                    <p className="text-white/90 leading-relaxed text-lg max-w-4xl">
                      {direction.description}
                    </p>
                    
                    <div className="mt-6 flex items-center text-white font-semibold text-lg group-hover:translate-x-2 transition-transform duration-300">
                      Explore {direction.id === 'architecture' ? 'information architecture' : 'refined direction'}
                      <svg className="w-5 h-5 ml-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M9 5l7 7-7 7" />
                      </svg>
                    </div>
                  </div>
                </Link>
              );
            })}
          </div>

          {/* Secondary & Original Explorations */}
          <div className="pt-6">
            <h3 className="text-lg font-semibold text-[#1f1c17] mb-4">Additional Explorations</h3>
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              {directions.filter(d => !d.featured).map((direction) => {
                const Icon = direction.icon;
                return (
                  <Link
                    key={direction.id}
                    to={direction.path}
                    className="group relative overflow-hidden rounded-xl bg-[#f7f3ea] border border-[#1f1c17]/10 p-6 transition-all duration-300 hover:shadow-xl hover:scale-[1.02] hover:border-[#1f1c17]/20"
                  >
                    <div className={`absolute inset-0 bg-gradient-to-br ${direction.color} opacity-0 group-hover:opacity-5 transition-opacity duration-300`} />
                    
                    <div className="relative z-10">
                      <div className={`w-12 h-12 rounded-lg bg-gradient-to-br ${direction.color} flex items-center justify-center mb-4 shadow-sm`}>
                        <Icon className="w-6 h-6 text-white" />
                      </div>
                      
                      <h2 className="text-lg font-semibold text-[#1f1c17] mb-2 leading-tight">
                        {direction.title}
                      </h2>
                      
                      <p className="text-[#7a7568] leading-relaxed text-sm">
                        {direction.description}
                      </p>
                      
                      <div className="mt-4 flex items-center text-[#1e4d4a] font-medium text-sm group-hover:translate-x-1 transition-transform duration-300">
                        View exploration
                        <svg className="w-4 h-4 ml-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                        </svg>
                      </div>
                    </div>
                  </Link>
                );
              })}
            </div>
          </div>
        </div>

        <footer className="mt-16 pt-8 border-t border-[#1f1c17]/10 text-center text-sm text-[#7a7568]">
          <p>
            Desktop-first, browser-capable application concept • Product order: Formula/Function Explorer → Live Formula Semantic X-Ray → Twin Oracle Workbench
          </p>
        </footer>
      </div>
    </div>
  );
}