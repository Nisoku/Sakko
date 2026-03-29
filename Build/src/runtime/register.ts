import type { RootNode } from "../parser/parser";
import { compileComponent } from "../compiler/component";
import { toPascalCase } from "../utils";

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

  const factory = new Function('require', `
    ${evalCode}
    return ${componentName};
  `)(typeof require !== 'undefined' ? require : () => ({})) as () => HTMLElement;

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

          connectedCallback() {
            const component = factory();
            this.shadowRoot!.appendChild(component);
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
