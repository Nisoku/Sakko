import { compileComponent } from "./compiler/component";
import { parseSakko } from "./parser/parser";

export { tokenize } from "./parser/tokenizer";
export { parseSakko, Parser } from "./parser/parser";
export type {
  RootNode,
  ElementNode,
  InlineNode,
  ListNode,
  ASTNode,
  Modifier,
  AtcodeDeclaration,
  InterpolatedText,
  InterpolatedTextPart,
} from "./parser/parser";
export type { Token, TokenType } from "./parser/tokenizer";

export { compileComponent } from "./compiler/component";
export {
  compileStateDeclarations,
  compileEffectDeclarations,
  compileEventHandler,
} from "./compiler/atcode";
export type { ComponentContext } from "./compiler/atcode";

export {
  registerSakkoComponent,
  getComponent,
  getComponentSource,
  getAllComponents,
} from "./runtime/register";

export function compile(code: string): { code: string; ast?: any } {
  try {
    const ast = parseSakko(code);
    const compiled = compileComponent(ast);
    return { code: compiled, ast };
  } catch (e) {
    return { code: `// Error: ${e}` };
  }
}

export function compileAtCode(code: string): { code: string } {
  const trimmed = code.trim();
  if (!trimmed.startsWith("<")) {
    // Preserve newlines and indentation properly
    return compile(`<wrapper {\n${trimmed}\n}>`);
  }
  return compile(code);
}
