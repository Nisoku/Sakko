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
          `const ${name} = signal(path("component", id, "${name}"), ${value});`,
        );
      }
    }

    if (decl.type === "derived") {
      for (const { name, expr } of decl.declarations) {
        ctx.derivedVars.add(name);

        const transformedExpr = addGetCallsToStateVars(expr, ctx);
        lines.push(
          `const ${name} = derived(path("component", id, "${name}"), () => ${transformedExpr});`,
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
    if (!/^[A-Za-z_]\w*$/.test(varName)) continue;

    const escapedVarName = escapeRegExp(varName);
    const regex = new RegExp(
      `(?<![A-Za-z0-9_$.])${escapedVarName}(?![A-Za-z0-9_$])(?!\\.get|\\.set|\\s*=|\\[)`,
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

  return `bindEvent(${el}, "${event}", (e) => {\n  ${compiledHandler}\n});`;
}

function compileHandlerBody(code: string, ctx: ComponentContext): string {
  let result = code;

  for (const varName of ctx.stateVars) {
    if (!/^[A-Za-z_]\w*$/.test(varName)) continue;
    const escapedVarName = escapeRegExp(varName);

    result = result.replace(
      new RegExp(`(?<![A-Za-z0-9_$])${escapedVarName}(?![A-Za-z0-9_$])\\+\\+`, "g"),
      `${varName}.set(${varName}.get() + 1)`,
    );

    result = result.replace(
      new RegExp(`(?<![A-Za-z0-9_$])${escapedVarName}(?![A-Za-z0-9_$])--`, "g"),
      `${varName}.set(${varName}.get() - 1)`,
    );

    result = result.replace(
      new RegExp(`(?<![A-Za-z0-9_$])${escapedVarName}(?![A-Za-z0-9_$])\\s*\\+=\\s*([^;]+)`, "g"),
      (match, expr) => {
        const cleanExpr = expr.trim();
        const exprWithGet = addGetCallsToStateVars(cleanExpr, ctx);
        return `${varName}.set(${varName}.get() + ${exprWithGet})`;
      },
    );

    result = result.replace(
      new RegExp(`(?<![A-Za-z0-9_$])${escapedVarName}(?![A-Za-z0-9_$])\\s*-=\\s*([^;]+)`, "g"),
      (match, expr) => {
        const cleanExpr = expr.trim();
        const exprWithGet = addGetCallsToStateVars(cleanExpr, ctx);
        return `${varName}.set(${varName}.get() - ${exprWithGet})`;
      },
    );

    result = result.replace(
      new RegExp(`(?<![A-Za-z0-9_$])${escapedVarName}(?![A-Za-z0-9_$])\\s*=(?![=>])\\s*([^;]+)`, "g"),
      (match, expr) => {
        const cleanExpr = expr.trim();
        const exprWithGet = addGetCallsToStateVars(cleanExpr, ctx);
        return `${varName}.set(${exprWithGet})`;
      },
    );
  }

  return addGetCallsToStateVars(result, ctx);
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

  if (elementType === "checkbox" || elementType === "sakko-checkbox") {
    return {
      code: `bindInputChecked(${el}, ${signalName});`,
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

export function compileClassModifier(
  expr: string,
  elementType: string,
  ctx: ComponentContext,
  elementVar?: string,
): string {
  const el = elementVar || "element";
  const exprWithGets = addGetCallsToStateVars(expr, ctx);

  return `effect(() => {
    const classVal = ${exprWithGets};
    if (typeof classVal === 'string') {
      ${el}.className = classVal;
    } else if (typeof classVal === 'object' && classVal !== null) {
      for (const [cls, active] of Object.entries(classVal)) {
        if (active) ${el}.classList.add(cls);
        else ${el}.classList.remove(cls);
      }
    }
  });`;
}

function escapeRegExp(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function compileInterpolation(
  parts: InterpolatedTextPart[],
  ctx: ComponentContext,
  elementVar: string,
): { static: boolean; code: string } {
  const reactiveVars = [...ctx.stateVars, ...ctx.derivedVars];
  const hasReactiveExpr = parts.some(
    (part) =>
      part.type === "expr" &&
      reactiveVars.some((v) => {
        const regex = new RegExp(`\\b${escapeRegExp(v)}\\b`);
        return regex.test(part.value);
      }),
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
        if (!/^[A-Za-z_]\w*$/.test(varName)) continue;
        const escapedVarName = escapeRegExp(varName);
        expr = expr.replace(
          new RegExp(`(?<![A-Za-z0-9_$.])${escapedVarName}(?![A-Za-z0-9_$])`, "g"),
          `${varName}.get()`,
        );
      }
      return `\${${expr}}`;
    })
    .join("");

  return {
    static: false,
    code: `effect(() => {
    ${elementVar}.textContent = \`${templateParts}\`;
  });`,
  };
}
