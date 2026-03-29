import type {
  AtcodeDeclaration,
  Modifier,
  InterpolatedText,
  InterpolatedTextPart,
} from "../parser/parser";

export interface ComponentContext {
  componentId: string;
  componentName: string;
  stateVars: Set<string>;
  derivedVars: Set<string>;
  elementIndex: number;
}

export function nextElementId(ctx: ComponentContext): number {
  return ctx.elementIndex++;
}

export function compileStateDeclarations(
  declarations: AtcodeDeclaration[],
  ctx: ComponentContext,
): string {
  const lines: string[] = [];

  for (const decl of declarations) {
    if (decl.type === "state") {
      for (const { name, value } of decl.declarations) {
        ctx.stateVars.add(name);

        lines.push(
          `const ${name} = signal(path("component", "${ctx.componentId}", "${name}"), ${value});`,
        );
      }
    }

    if (decl.type === "derived") {
      for (const { name, expr } of decl.declarations) {
        ctx.derivedVars.add(name);

        lines.push(
          `const ${name} = derived(path("component", "${ctx.componentId}", "${name}"), () => ${expr});`,
        );
      }
    }
  }

  return lines.join("\n");
}

export function compileEffectDeclarations(
  declarations: AtcodeDeclaration[],
  ctx: ComponentContext,
): string {
  const lines: string[] = [];

  for (const decl of declarations) {
    if (decl.type === "effect") {
      const bodyWithGets = addGetCallsToStateVars(decl.body, ctx);

      lines.push(`effect(() => {\n  ${bodyWithGets}\n});`);
    }
  }

  return lines.join("\n");
}

function addGetCallsToStateVars(code: string, ctx: ComponentContext): string {
  let result = code;

  for (const varName of [...ctx.stateVars, ...ctx.derivedVars]) {
    const regex = new RegExp(
      `\\b${varName}\\b(?!\\.get|\\.set|\\s*=|\\[)`,
      "g",
    );
    result = result.replace(regex, `${varName}.get()`);
  }

  return result;
}

export function compileEventHandler(
  modifier: Modifier & { type: "event" },
  ctx: ComponentContext,
  elementVar?: string,
): string {
  const { event, handler } = modifier;
  const compiledHandler = compileHandlerBody(handler, ctx);
  const el = elementVar || "element";

  return `bindEvent(${el}, "${event}", () => {
  ${compiledHandler}
});`;
}

function compileHandlerBody(code: string, ctx: ComponentContext): string {
  let result = code;

  for (const varName of ctx.stateVars) {
    result = result.replace(
      new RegExp(`${varName}\\s*\\+\\+`, "g"),
      `${varName}.set(${varName}.get() + 1)`,
    );

    result = result.replace(
      new RegExp(`${varName}\\s*--`, "g"),
      `${varName}.set(${varName}.get() - 1)`,
    );

    result = result.replace(
      new RegExp(`${varName}\\s*\\+\\s*=\\s*([^;]+)`, "g"),
      (match, expr) => {
        const cleanExpr = expr.trim();
        const exprWithGet = addGetCallsToStateVars(cleanExpr, ctx);
        return `${varName}.set(${varName}.get() + ${exprWithGet})`;
      },
    );

    result = result.replace(
      new RegExp(`${varName}\\s*-\\s*=\\s*([^;]+)`, "g"),
      (match, expr) => {
        const cleanExpr = expr.trim();
        const exprWithGet = addGetCallsToStateVars(cleanExpr, ctx);
        return `${varName}.set(${varName}.get() - ${exprWithGet})`;
      },
    );

    result = result.replace(
      new RegExp(`${varName}\\s*=\\s*([^;]+)`, "g"),
      (match, expr) => {
        const cleanExpr = expr.trim();
        const exprWithGet = addGetCallsToStateVars(cleanExpr, ctx);
        return `${varName}.set(${exprWithGet})`;
      },
    );
  }

  return result;
}

export interface BindResult {
  code: string;
  signalName: string | null;
  bindingType: "two-way" | "text" | "attribute" | "event";
}

export function compileBindModifier(
  signalName: string,
  elementType: string,
  ctx: ComponentContext,
  elementVar?: string,
): BindResult {
  const el = elementVar || "element";

  if (elementType === "input" || elementType === "saz-input") {
    return {
      code: `${el}.value = ${signalName};`,
      signalName,
      bindingType: "two-way",
    };
  }

  if (elementType === "checkbox" || elementType === "saz-checkbox") {
    return {
      code: `bindInputChecked(${el}, ${signalName});`,
      signalName,
      bindingType: "two-way",
    };
  }

  if (elementType === "select" || elementType === "saz-select") {
    return {
      code: `${el}.value = ${signalName};`,
      signalName,
      bindingType: "two-way",
    };
  }

  return {
    code: `bindInputValue(${el}, ${signalName});`,
    signalName,
    bindingType: "two-way",
  };
}

export function compileInterpolation(
  parts: InterpolatedTextPart[],
  ctx: ComponentContext,
): { static: boolean; code: string } {
  const hasReactiveExpr = parts.some(
    (part) =>
      part.type === "expr" &&
      [...ctx.stateVars, ...ctx.derivedVars].some((v) =>
        part.value.includes(v),
      ),
  );

  if (!hasReactiveExpr) {
    const str = parts
      .map((p) => (p.type === "text" ? p.value : `\${${p.value}}`))
      .join("");
    return { static: true, code: `\`${str}\`` };
  }

  const templateParts = parts
    .map((p) => {
      if (p.type === "text") return p.value;

      let expr = p.value;
      for (const varName of [...ctx.stateVars, ...ctx.derivedVars]) {
        expr = expr.replace(
          new RegExp(`\\b${varName}\\b`, "g"),
          `${varName}.get()`,
        );
      }
      return `\${${expr}}`;
    })
    .join("");

  return {
    static: false,
    code: `effect(() => {
    element.textContent = \`${templateParts}\`;
  });`,
  };
}
