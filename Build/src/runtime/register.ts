import type { RootNode } from "../parser/parser";
import { compileComponent } from "../compiler/component";

interface RegisteredComponent {
  name: string;
  factory: (id?: string) => HTMLElement;
  source: string;
}

const componentRegistry = new Map<string, RegisteredComponent>();

export function registerSakkoComponent(ast: RootNode): void {
  const componentCode = compileComponent(ast);

  const factory = new Function(
    "require",
    `
    const module = { exports: {} };
    ${componentCode}
    return module.exports.${toPascalCase(ast.name)};
    `,
  )(require as any) as () => HTMLElement;

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

function toPascalCase(str: string): string {
  return str
    .split(/[-_]/)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join("");
}
