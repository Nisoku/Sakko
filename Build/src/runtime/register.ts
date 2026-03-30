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

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof document !== "undefined";
}

async function createFactoryFromCode(
  evalCode: string,
  componentName: string,
  modulePath: string,
): Promise<{ factory: (id?: string) => HTMLElement; dispose: (id: string) => void }> {
  if (isBrowser()) {
    let moduleCode = evalCode;
    if (!evalCode.includes(`export { ${componentName}, dispose }`)) {
      moduleCode = `${evalCode}\nexport { ${componentName}, dispose };\n`;
    }
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
    const result = new Function('require', `
      const module = { exports: {} };
      const exports = module.exports;
      ${evalCode}
      return { factory: module.exports.${componentName}, dispose: module.exports.dispose };
    `)(require);
    return result as { factory: (id?: string) => HTMLElement; dispose: (id: string) => void };
  }
}

export async function registerSakkoComponent(ast: RootNode, options: RegisterOptions = {}): Promise<void> {
  const importMode = options.sairinImport ?? "global";
  const globalName = options.sairinGlobal ?? "sairin";
  const modulePath = options.sairinModule ?? "sairin";
  const normalizedName = ast.name.toLowerCase();

  const componentCode = compileComponent(ast, { sairinImport: importMode, sairinGlobal: globalName, sairinModule: modulePath });
  const componentName = toPascalCase(ast.name);

  if (importMode === "esm") {
    throw new Error(`registerSakkoComponent: ESM mode requires a bundler. Use 'global' mode or call compileComponent separately.`);
  }

  if (importMode === "cjs" && isBrowser()) {
    throw new Error(`registerSakkoComponent: CJS mode is not supported in browsers. Use 'global' mode or call compileComponent separately for bundler integration.`);
  }

  let evalCode = componentCode;
  if (importMode === "cjs") {
    evalCode = componentCode
      .replace(/import\s+\*\s+as\s+(\w+)\s+from\s+['"]([^'"]+)['"];?/g, (_m, name, mod) =>
        `const ${name} = require('${mod}');`
      )
      .replace(/import\s+\{\s*([\s\S]*?)\s*\}\s*from\s+['"]([^'"]+)['"];?/g, (_m, specifiers, mod) => {
        const names = specifiers.split(',').map((s: string) => {
          const trimmed = s.trim();
          const match = trimmed.match(/^(\w+)(?:\s+as\s+\w+)?$/);
          return match ? match[1] : trimmed;
        }).filter(Boolean);
        return `const {${names.join(', ')}} = require('${mod}');`;
      })
      .replace(/export\s+\{\s*([\s\S]*?)\s*\};?/g, (_m, specifiers) => {
        const lines = specifiers.split(',').map((s: string) => {
          const trimmed = s.trim();
          const match = trimmed.match(/^(\w+)(?:\s+as\s+(\w+))?$/);
          if (match) {
            const original = match[1];
            const alias = match[2];
            return `module.exports.${alias || original} = ${original};`;
          }
          return '';
        }).filter(Boolean);
        return lines.join('\n');
      })
      .replace(/export\s+default\s+function\s+(\w+)/g, 'module.exports = function $1')
      .replace(/export\s+default\s+/g, 'module.exports = ')
      .replace(/export\s+function\s+/g, 'function ')
      .replace(/export\s+const\s+/g, 'const ');

    evalCode += `\nmodule.exports.${componentName} = ${componentName};\n`;
    evalCode += `module.exports.dispose = dispose;\n`;
  }

  const { factory, dispose } = await createFactoryFromCode(evalCode, componentName, modulePath);

  componentRegistry.set(normalizedName, {
    name: normalizedName,
    factory,
    dispose,
    source: componentCode,
  });

  if (typeof customElements !== "undefined") {
    const tagName = `sakko-${normalizedName}`;
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
            const entry = componentRegistry.get(normalizedName);
            if (!entry) return;
            this._componentId = `${normalizedName}-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
            this._component = entry.factory(this._componentId);
            this.shadowRoot!.appendChild(this._component);
            this._rendered = true;
          }

          disconnectedCallback() {
            if (this._component) {
              const entry = componentRegistry.get(normalizedName);
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
  const entry = componentRegistry.get(name.toLowerCase());
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
  return componentRegistry.get(name.toLowerCase())?.source;
}
