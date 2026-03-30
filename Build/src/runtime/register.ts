import type { RootNode } from "../parser/parser";
import { compileComponent } from "../compiler/component";
import { toPascalCase } from "../utils";
import * as sairin from "@nisoku/sairin";

interface RegisteredComponent {
  readonly name: string;
  readonly factory: (id?: string) => HTMLElement;
  readonly source: string;
}

const componentRegistry = new Map<string, RegisteredComponent>();

export function registerSakkoComponent(ast: RootNode): void {
  const componentCode = compileComponent(ast);
  const componentName = toPascalCase(ast.name);

  // Transform ESM to simple CJS-like eval format
  const evalCode = componentCode
    // import { a, b } from 'mod' → const {a, b} = require('mod');
    .replace(/import\s+{([\s\S]*?)}\s+from\s+['"]([^'"]+)['"];?/g, (match, p1, p2) => {
      const cleaned = p1.replace(/\s+/g, ' ').trim();
      return `const {${cleaned}} = require('${p2}');`;
    })
    // import * as name from 'mod' → const name = require('mod');
    .replace(/import\s+\*\s+as\s+(\w+)\s+from\s+['"]([^'"]+)['"];?/g, (_m, name, mod) =>
      `const ${name} = require('${mod}');`
    )
    // export { foo, bar } → (append to module.exports)
    .replace(/export\s+\{([\s\S]*?)\};?/g, (_m, specifiers) => {
      const names = specifiers.split(',').map((s: string) => s.trim()).filter(Boolean);
      return names.map((n: string) => `module.exports.${n} = ${n};`).join('\n');
    })
    .replace(/export\s+default\s+function/g, 'module.exports = function')
    .replace(/export\s+default\s+/g, 'module.exports = ')
    .replace(/export\s+function/g, 'function')
    .replace(/export\s+const/g, 'const');

  const fallbackRequire = (pkg: string) => {
    if (pkg === "@nisoku/sairin") return sairin;
    if (typeof require !== "undefined") return require(pkg);
    throw new Error(`fallbackRequire: Cannot resolve module "${pkg}". standard 'require' is not available in this environment.`);
  };

  const factory = new Function("require", `
    ${evalCode}
    return ${componentName};
  `)(fallbackRequire) as (id?: string) => HTMLElement;

  componentRegistry.set(ast.name, {
    name: ast.name,
    factory,
    source: componentCode,
  });

  if (typeof customElements !== "undefined") {
    const tagName = `sakko-${ast.name}`;
    if (!customElements.get(tagName)) {
      customElements.define(
        tagName,
        class extends HTMLElement {
          constructor() {
            super();
            this.attachShadow({ mode: "open" });
          }

          private _rendered = false;
          private _component: HTMLElement | null = null;

          connectedCallback() {
            if (this._rendered) return;
            const entry = componentRegistry.get(ast.name);
            if (!entry) return;
            const id = `${ast.name}-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
            this._component = entry.factory(id);
            this.shadowRoot!.appendChild(this._component);
            this._rendered = true;
          }

          disconnectedCallback() {
            if (this._component) {
              this.shadowRoot!.innerHTML = "";
              this._component = null;
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
