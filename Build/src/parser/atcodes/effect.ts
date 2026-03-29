import type { AtcodeDeclaration } from "../types";
import type { Token } from "../tokenizer";

export interface ParserCore {
  check: (type: string) => boolean;
  consume: () => Token;
  peek: () => Token | undefined;
  errorAt: (msg: string, token?: Token) => Error;
}

export interface ParserState extends ParserCore {
  parseBlockBody: () => string;
}

export function parseEffectDeclaration(
  parser: ParserState,
  atToken: Token,
): AtcodeDeclaration {
  const hasBraces = parser.check("LBRACE");
  if (hasBraces) parser.consume();

  let body = "";
  if (hasBraces) {
    body = parser.parseBlockBody();
  } else {
    body = "";
  }

  return {
    type: "effect",
    body,
    line: atToken.line,
    col: atToken.col,
  };
}
