import type { ElementNode, ASTNode, Modifier, ParserState } from "../types";

export function parseElementNode(
   
  _parser: ParserState,
  name: string,
  modifiers: Array<{ type: string; value?: string; key?: string }>,
  children: ASTNode[],
): ElementNode {
  return {
    type: "element",
    name,
    modifiers: modifiers as Modifier[],
    children,
  };
}
