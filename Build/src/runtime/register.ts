import type { RootNode } from "../parser/parser";
import { compileComponent } from "../compiler/component";
import { toPascalCase } from "../utils";
import * as sairin from "@nisoku/sairin";

interface RegisteredComponent {
  name: string;
  factory: (id?: string) => HTMLElement;
  source: string;
}

const componentRegistry = new Map<string, RegisteredComponent>();

export function registerSakkoComponent(ast: RootNode): void {
  const componentCode = compileComponent(ast);
  const componentName = toPascalCase(ast.name);

  // Transform ESM to simple CJS-like eval format
  const evalCode = componentCode
    .replace(/import\s+{([\s\S]*?)}\s+from\s+['"]([^'"]+)['"];?/g, (match, p1, p2) => {
      const cleaned = p1.replace(/\s+/g, ' ').trim();
      return `const {${cleaned}} = require('${p2}');`;
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
            this._component = factory();
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

export function getComponent(name: string): RegisteredComponent | undefined {
  return componentRegistry.get(name);
}

export function getAllComponents(): Map<string, RegisteredComponent> {
  return new Map(componentRegistry);
}

export function getComponentSource(name: string): string | undefined {
  return componentRegistry.get(name)?.source;
}
