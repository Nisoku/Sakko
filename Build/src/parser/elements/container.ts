import type { ElementNode, ListNode, ASTNode, ParserState } from "../types";

export function parseElementNode(
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  _parser: ParserState,
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
