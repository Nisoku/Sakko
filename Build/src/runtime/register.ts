import type { RootNode } from "../parser/parser";
import { compileComponent, type SairinImportMode } from "../compiler/component";
import { toPascalCase } from "../utils";

interface RegisteredComponent {
  readonly name: string;
  readonly factory: (id?: string) => HTMLElement;
  readonly dispose: (id: string) => void;
  readonly source: string;
}

const componentRegistry = new Map<string, RegisteredComponent>();

export interface RegisterOptions {
  sairinImport?: SairinImportMode;
  sairinGlobal?: string;
  sairinModule?: string;
}

async function createFactoryFromCode(
  evalCode: string,
  componentName: string,
  modulePath: string,
): Promise<{ factory: (id?: string) => HTMLElement; dispose: (id: string) => void }> {
  const isBrowser = typeof window !== "undefined" && typeof document !== "undefined";

  if (isBrowser) {
    const moduleCode = `
${evalCode}
export { ${componentName}, dispose };
`;
    const blob = new Blob([moduleCode], { type: "text/javascript" });
    const url = URL.createObjectURL(blob);
    try {
      const mod = await import(url);
      return {
        factory: mod[componentName],
        dispose: mod.dispose,
      };
    } finally {
      URL.revokeObjectURL(url);
    }
  } else {
    const factory = new Function(`
      const module = { exports: {} };
      const exports = module.exports;
      ${evalCode}
      return { factory: module.exports.${componentName}, dispose: module.exports.dispose };
    `)() as { factory: (id?: string) => HTMLElement; dispose: (id: string) => void };
    return factory;
  }
}

export async function registerSakkoComponent(ast: RootNode, options: RegisterOptions = {}): Promise<void> {
  const importMode = options.sairinImport ?? "global";
  const globalName = options.sairinGlobal ?? "sairin";
  const modulePath = options.sairinModule ?? "sairin";

  const componentCode = compileComponent(ast, { sairinImport: importMode, sairinGlobal: globalName, sairinModule: modulePath });
  const componentName = toPascalCase(ast.name);

  if (importMode === "esm") {
    throw new Error(`registerSakkoComponent: ESM mode requires a bundler. Use 'global' or 'cjs' mode, or call compileComponent separately.`);
  }

  let evalCode = componentCode;
  if (importMode === "cjs") {
    evalCode = componentCode
      .replace(/import\s+\*\s+as\s+(\w+)\s+from\s+['"]([^'"]+)['"];?/g, (_m, name, mod) =>
        `const ${name} = require('${mod}');`
      )
      .replace(/export\s+\{([\s\S]*?)\};?/g, (_m, specifiers) => {
        const names = specifiers.split(',').map((s: string) => s.trim()).filter(Boolean);
        return names.map((n: string) => `module.exports.${n} = ${n};`).join('\n');
      })
      .replace(/export\s+default\s+function/g, 'module.exports = function')
      .replace(/export\s+default\s+/g, 'module.exports = ')
      .replace(/export\s+function/g, 'function')
      .replace(/export\s+const/g, 'const');
  }

  const { factory, dispose } = await createFactoryFromCode(evalCode, componentName, modulePath);

  componentRegistry.set(ast.name, {
    name: ast.name,
    factory,
    dispose,
    source: componentCode,
  });

  if (typeof customElements !== "undefined") {
    const tagName = `sakko-${ast.name.toLowerCase()}`;
    if (!customElements.get(tagName)) {
      customElements.define(
        tagName,
        class extends HTMLElement {
          private _rendered = false;
          private _component: HTMLElement | null = null;
          private _componentId: string | null = null;

          constructor() {
            super();
            this.attachShadow({ mode: "open" });
          }

          connectedCallback() {
            if (this._rendered) return;
            const entry = componentRegistry.get(ast.name);
            if (!entry) return;
            this._componentId = `${ast.name}-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
            this._component = entry.factory(this._componentId);
            this.shadowRoot!.appendChild(this._component);
            this._rendered = true;
          }

          disconnectedCallback() {
            if (this._component) {
              const entry = componentRegistry.get(ast.name);
              if (entry && this._componentId) {
                entry.dispose(this._componentId);
              }
              this.shadowRoot!.innerHTML = "";
              this._component = null;
              this._componentId = null;
            }
            this._rendered = false;
          }
        },
      );
    }
  }
}

export function getComponent(name: string): Readonly<RegisteredComponent> | undefined {
  const entry = componentRegistry.get(name);
  if (!entry) return undefined;
  return Object.freeze({ ...entry });
}

export function getAllComponents(): ReadonlyMap<string, Readonly<RegisteredComponent>> {
  const frozen = new Map<string, Readonly<RegisteredComponent>>();
  for (const [key, entry] of componentRegistry) {
    frozen.set(key, Object.freeze({ ...entry }));
  }
  return Object.freeze(frozen);
}

export function getComponentSource(name: string): string | undefined {
  return componentRegistry.get(name)?.source;
}
