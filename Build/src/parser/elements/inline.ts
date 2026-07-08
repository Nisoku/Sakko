import type { InlineNode, InterpolatedText, Modifier } from "../types";
import type { Token } from "../tokenizer";

export interface ParserState {
  check: (type: string) => boolean;
  consume: () => Token;
  peek: () => Token | undefined;
  parseInterpolatedValue: () => string | InterpolatedText;
}

export function parseInlineNode(
  parser: ParserState,
  name: string,
  modifiers: Array<{ type: string; value?: string; key?: string }>,
): InlineNode {
  const valToken = parser.peek();

  if (!valToken) {
    return {
      type: "inline",
      name,
      modifiers: modifiers as Modifier[],
      value: "",
    };
  }

  if (valToken.type === "STRING" || valToken.type === "INTERP_START") {
    const value = parser.parseInterpolatedValue();
    return { type: "inline", name, modifiers: modifiers as Modifier[], value };
  }

  if (valToken.type === "IDENT") {
    const value = parser.consume().value;
    return { type: "inline", name, modifiers: modifiers as Modifier[], value };
  }

  return {
    type: "inline",
    name,
    modifiers: modifiers as Modifier[],
    value: "",
  };
}
