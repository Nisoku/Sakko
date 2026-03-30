import type {
  RootNode,
  ASTNode,
  InlineNode,
  ElementNode,
} from "../parser/parser";
import {
  type ComponentContext,
  compileStateDeclarations,
  compileEffectDeclarations,
  compileEventHandler,
  compileBindModifier,
  compileClassModifier,
  compileInterpolation,
  nextElementId,
} from "./atcode";
import { toPascalCase } from "../utils";

export type SairinImportMode = "global" | "esm" | "cjs";

export interface CompileOptions {
  id?: string;
  sairinImport?: SairinImportMode;
  sairinGlobal?: string;
  sairinModule?: string;
}

function formatCode(code: string): string {
  const lines = code.split("\n");
  let result: string[] = [];
  let baseIndent = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    if (!trimmed) {
      result.push("");
      continue;
    }

    // Check if this line closes something that was opened on a previous line
    if (
      (trimmed === "}" ||
        trimmed.startsWith("});") ||
        trimmed.startsWith("])") ||
        trimmed.startsWith("})")) &&
      baseIndent > 0
    ) {
      // But first check if previous line was an opening
      const prevTrimmed = i > 0 ? lines[i - 1].trim() : "";
      if (!prevTrimmed.endsWith("{") && !prevTrimmed.includes(") {")) {
        baseIndent--;
      }
    }

    result.push("  ".repeat(baseIndent) + trimmed);

    // Increase indent after this line if it opens a block
    if (
      trimmed.endsWith("{") ||
      trimmed.includes(") {") ||
      trimmed.includes("=> {")
    ) {
      baseIndent++;
    }
  }

  return result.join("\n");
}

function hashString(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash;
  }
  return Math.abs(hash);
}

function generateSairinImports(mode: SairinImportMode, globalName: string, modulePath: string): string {
  switch (mode) {
    case "global":
      return `const { signal, effect, derived, path } = ${globalName};
const { bind, bindEvent, bindInputValue, bindInputChecked } = ${globalName};`;
    case "esm":
      return `import { signal, effect, derived, path } from '${modulePath}';
import { bind, bindEvent, bindInputValue, bindInputChecked } from '${modulePath}';`;
    case "cjs":
      return `const { signal, effect, derived, path } = require('${modulePath}');
const { bind, bindEvent, bindInputValue, bindInputChecked } = require('${modulePath}');`;
  }
}

export function compileComponent(
  root: RootNode,
  options?: CompileOptions,
): string {
  const componentId = options?.id
    ? `comp_${options.id}_${hashString(root.name)}`
    : `comp_${hashString(root.name)}`;
  const componentName = toPascalCase(root.name);
  const importMode = options?.sairinImport ?? "global";
  const globalName = options?.sairinGlobal ?? "sairin";
  const modulePath = options?.sairinModule ?? "sairin";

  const ctx: ComponentContext = {
    componentId,
    componentName,
    stateVars: new Set(),
    derivedVars: new Set(),
    elementIndex: 0,
  };

  const imports = generateSairinImports(importMode, globalName, modulePath);

  const stateCode = compileStateDeclarations(root.declarations, ctx);
  const effectCode = compileEffectDeclarations(root.declarations, ctx);
  const renderCode = compileChildren(root.children, ctx);

  const signalPopulations = [...ctx.stateVars, ...ctx.derivedVars]
    .map((v) => `  signals["${v}"] = ${v};`)
    .join("\n");

  const componentFn = `export function ${componentName}(id = ${JSON.stringify(componentId)}) {
  const signals = {};
  instanceSignals.set(id, signals);

${stateCode}

${effectCode}

${signalPopulations}

  const root = document.createElement('div');
  root.className = ${JSON.stringify(root.name)};

${renderCode}

  return root;
}`;

  const getSignalFn = `export function getSignal(id, signalName) {
  const signals = instanceSignals.get(id);
  return signals ? (signals[signalName] || null) : null;
}`;

  const disposeFn = `export function dispose(id) {
  instanceSignals.delete(id);
}`;

  return formatCode([imports, "const instanceSignals = new Map();\nconst REACTIVE_CLASSES = Symbol('sakko.reactiveClasses');", "", componentFn, "", getSignalFn, "", disposeFn].join("\n\n"));
}

function compileChildren(children: ASTNode[], ctx: ComponentContext, parentVar: string = "root"): string {
  const lines: string[] = [];

  for (const child of children) {
    if (child.type === "inline") {
      lines.push(compileInlineNode(child, ctx, parentVar));
    } else if (child.type === "element") {
      lines.push(compileElementNode(child, ctx, parentVar));
    } else if (child.type === "list") {
      for (const item of child.items) {
        if (item.type === "inline") {
          lines.push(compileInlineNode(item, ctx, parentVar));
        } else if (item.type === "element") {
          lines.push(compileElementNode(item, ctx, parentVar));
        }
      }
    }
  }

  return lines.join("\n\n");
}

function compileInlineNode(node: InlineNode, ctx: ComponentContext, parentVar: string): string {
  const idx = nextElementId(ctx);
  const elementVar = `${node.name}${idx}`;

  const lines: string[] = [];
  lines.push(`// ${node.name}`);
  lines.push(
    `const ${elementVar} = document.createElement('sakko-${node.name}');`,
  );

  for (const mod of node.modifiers) {
    if (mod.type === "flag") {
      lines.push(`${elementVar}.setAttribute(${JSON.stringify(mod.value)}, '');`);
    } else if (mod.type === "pair") {
      lines.push(`${elementVar}.setAttribute(${JSON.stringify(mod.key)}, ${JSON.stringify(mod.value)});`);
    } else if (mod.type === "event") {
      const handlerCode = compileEventHandler(mod, ctx, elementVar);
      lines.push(handlerCode);
    } else if (mod.type === "atcode" && mod.name === "bind") {
      const bindResult = compileBindModifier(
        mod.body,
        node.name,
        ctx,
        elementVar,
      );
      lines.push(bindResult.code);
    } else if (mod.type === "atcode" && mod.name === "class") {
      const classCode = compileClassModifier(
        mod.body,
        node.name,
        ctx,
        elementVar,
      );
      lines.push(classCode);
    }
  }

  if (typeof node.value === "string") {
    lines.push(`${elementVar}.textContent = ${JSON.stringify(node.value)};`);
  } else if (
    node.value &&
    typeof node.value === "object" &&
    "parts" in node.value
  ) {
    const { static: isStatic, code } = compileInterpolation(
      node.value.parts,
      ctx,
      elementVar,
    );
    if (isStatic) {
      lines.push(`${elementVar}.textContent = ${code};`);
    } else {
      lines.push(code);
    }
  }

  lines.push(`${parentVar}.appendChild(${elementVar});`);

  return lines.join("\n");
}

function compileElementNode(node: ElementNode, ctx: ComponentContext, parentVar: string): string {
  const idx = nextElementId(ctx);
  const elementVar = `${node.name}${idx}`;

  const lines: string[] = [];
  lines.push(`// ${node.name} container`);
  lines.push(
    `const ${elementVar} = document.createElement('sakko-${node.name}');`,
  );

  for (const mod of node.modifiers) {
    if (mod.type === "flag") {
      lines.push(`${elementVar}.setAttribute(${JSON.stringify(mod.value)}, '');`);
    } else if (mod.type === "pair") {
      lines.push(`${elementVar}.setAttribute(${JSON.stringify(mod.key)}, ${JSON.stringify(mod.value)});`);
    } else if (mod.type === "event") {
      const handlerCode = compileEventHandler(mod, ctx, elementVar);
      lines.push(handlerCode);
    } else if (mod.type === "atcode" && mod.name === "bind") {
      const bindResult = compileBindModifier(
        mod.body,
        node.name,
        ctx,
        elementVar,
      );
      lines.push(bindResult.code);
    } else if (mod.type === "atcode" && mod.name === "class") {
      const classCode = compileClassModifier(
        mod.body,
        node.name,
        ctx,
        elementVar,
      );
      lines.push(classCode);
    }
  }

  if (node.children.length > 0) {
    const childCode = compileChildren(node.children, ctx, elementVar);
    lines.push(childCode);
  }

  lines.push(`${parentVar}.appendChild(${elementVar});`);

  return lines.join("\n");
}
