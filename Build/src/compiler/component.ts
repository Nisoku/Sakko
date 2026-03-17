import type { RootNode, ASTNode, InlineNode, ElementNode } from "../parser/parser";
import {
  type ComponentContext,
  compileStateDeclarations,
  compileEffectDeclarations,
  compileEventHandler,
  compileBindModifier,
  compileInterpolation,
} from "./atcode";

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
    if ((trimmed === "}" || trimmed.startsWith("});") || trimmed.startsWith("});") || trimmed.startsWith("})")) && baseIndent > 0) {
      // But first check if previous line was an opening
      const prevTrimmed = i > 0 ? lines[i-1].trim() : "";
      if (!prevTrimmed.endsWith("{") && !prevTrimmed.includes(") {")) {
        baseIndent--;
      }
    }

    result.push("  ".repeat(baseIndent) + trimmed);

    // Increase indent after this line if it opens a block
    if (trimmed.endsWith("{") || trimmed.includes(") {") || trimmed.includes("=> {")) {
      baseIndent++;
    }
  }

  return result.join("\n");
}

export function compileComponent(root: RootNode): string {
  const componentId = `comp_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
  const componentName = toPascalCase(root.name);

  const ctx: ComponentContext = {
    componentId,
    componentName,
    stateVars: new Set(),
    derivedVars: new Set(),
  };

  const imports = `import { signal, effect, derived, path } from '@nisoku/sairin';`;
  
  const stateCode = compileStateDeclarations(root.declarations, ctx);
  const effectCode = compileEffectDeclarations(root.declarations, ctx);
  
  const renderCode = compileChildren(root.children, ctx);

  const componentFn = `export function ${componentName}(id = "${componentId}") {
${stateCode}

${effectCode}

const root = document.createElement('div');
root.className = '${root.name}';

${renderCode}

return root;
}`;

  const signalCases = [...ctx.stateVars, ...ctx.derivedVars]
    .map(v => `    case "${v}": return ${v};`)
    .join("\n");
  
  const getSignalFn = `export function getSignal(signalName: string) {
  switch (signalName) {
${signalCases}
    default: return null;
  }
}`;

  return formatCode([imports, "", componentFn, "", getSignalFn].join("\n\n"));
}

function compileChildren(
  children: ASTNode[],
  ctx: ComponentContext
): string {
  const lines: string[] = [];

  for (const child of children) {
    if (child.type === "inline") {
      lines.push(compileInlineNode(child, ctx));
    } else if (child.type === "element") {
      lines.push(compileElementNode(child, ctx));
    } else if (child.type === "list") {
      for (const item of child.items) {
        if (item.type === "inline") {
          lines.push(compileInlineNode(item, ctx));
        } else if (item.type === "element") {
          lines.push(compileElementNode(item, ctx));
        }
      }
    }
  }

  return lines.join("\n\n");
}

function compileInlineNode(
  node: InlineNode,
  ctx: ComponentContext
): string {
  const elementVar = `${node.name}Element`;
  
  const lines: string[] = [];
  lines.push(`// ${node.name}`);
  lines.push(`const ${elementVar} = document.createElement('saz-${node.name}');`);

  for (const mod of node.modifiers) {
    if (mod.type === "flag") {
      lines.push(`${elementVar}.setAttribute('${mod.value}', '');`);
    } else if (mod.type === "pair") {
      lines.push(`${elementVar}.setAttribute('${mod.key}', '${mod.value}');`);
    } else if (mod.type === "event") {
      const handlerCode = compileEventHandler(mod, ctx);
      lines.push(handlerCode);
    } else if (mod.type === "atcode" && mod.name === "bind") {
      const bindCode = compileBindModifier(mod.body, node.name, ctx);
      lines.push(bindCode);
    }
  }

  if (typeof node.value === "string") {
    lines.push(`${elementVar}.textContent = "${node.value}";`);
  } else if (node.value && typeof node.value === "object" && "parts" in node.value) {
    const { static: isStatic, code } = compileInterpolation(node.value.parts, ctx);
    if (isStatic) {
      lines.push(`${elementVar}.textContent = ${code};`);
    } else {
      lines.push(code);
    }
  }

  lines.push(`root.appendChild(${elementVar});`);

  return lines.join("\n");
}

function compileElementNode(
  node: ElementNode,
  ctx: ComponentContext
): string {
  const elementVar = `${node.name}Container`;
  
  const lines: string[] = [];
  lines.push(`// ${node.name} container`);
  lines.push(`const ${elementVar} = document.createElement('saz-${node.name}');`);

  for (const mod of node.modifiers) {
    if (mod.type === "flag") {
      lines.push(`${elementVar}.setAttribute('${mod.value}', '');`);
    } else if (mod.type === "pair") {
      lines.push(`${elementVar}.setAttribute('${mod.key}', '${mod.value}');`);
    } else if (mod.type === "event") {
      const handlerCode = compileEventHandler(mod, ctx);
      lines.push(handlerCode);
    } else if (mod.type === "atcode" && mod.name === "bind") {
      const bindCode = compileBindModifier(mod.body, node.name, ctx);
      lines.push(bindCode);
    }
  }

  if (node.children.length > 0) {
    const childCode = compileChildren(node.children, ctx);
    lines.push(childCode);
  }

  lines.push(`root.appendChild(${elementVar});`);

  return lines.join("\n");
}

function toPascalCase(str: string): string {
  return str
    .split(/[-_]/)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join("");
}
