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
    .replace(/import\s+{([^}]+)}\s+from\s+['"]([^'"]+)['"];?/g, 'const {$1} = require(\'$2\');')
    .replace(/export\s+function/g, 'function')
    .replace(/export\s+const/g, 'const');

  const fallbackRequire = (pkg: string) => {
    if (pkg === "@nisoku/sairin") return sairin;
    if (typeof require !== "undefined") return require(pkg);
    return {};
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

          connectedCallback() {
            if (this._rendered) return;
            const component = factory();
            this.shadowRoot!.appendChild(component);
            this._rendered = true;
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
