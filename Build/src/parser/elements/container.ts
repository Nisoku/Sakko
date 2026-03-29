import type { ElementNode, ListNode, ASTNode } from "../types";

export interface ParserState {
  check: (type: string) => boolean;
  parseList: () => ListNode;
  parseNode: () => ASTNode;
}

export function parseElementNode(
  parser: ParserState,
  name: string,
  modifiers: Array<{ type: string; value?: string; key?: string }>,
  children: ASTNode[],
): ElementNode {
  return {
    type: "element",
    name,
    modifiers: modifiers as any,
    children,
  };
}
