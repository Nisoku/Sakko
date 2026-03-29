import type { AtcodeDeclaration, ParserState } from "../types";
import type { Token } from "../tokenizer";

export function parseEffectDeclaration(
  parser: ParserState,
  atToken: Token,
): AtcodeDeclaration {
  const hasBraces = parser.check("LBRACE");
  if (!hasBraces) {
    throw parser.errorAt("@effect requires a braced block", atToken);
  }
  parser.consume();

  const body = parser.parseBlockBody();
  parser.expect("RBRACE");

  return {
    type: "effect",
    body,
    line: atToken.line,
    col: atToken.col,
  };
}
