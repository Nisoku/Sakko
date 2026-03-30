import type { AtcodeDeclaration, ParserState } from "../types";
import type { Token } from "../tokenizer";

export function parseStateDeclaration(
  parser: ParserState,
  atToken: Token,
): AtcodeDeclaration {
  const hasBraces = parser.check("LBRACE");
  if (hasBraces) parser.consume();

  const declarations: Array<{ name: string; value: string }> = [];

  while (true) {
    if (!parser.peek()) break;

    if (hasBraces && parser.check("RBRACE")) {
      parser.consume();
      break;
    }

    if (parser.check("IDENT") && parser.peek()?.value === "const") {
      parser.consume();
      const nextToken = parser.peek();
      if (!nextToken || nextToken.type !== "IDENT") {
        throw parser.errorAt("Expected identifier after 'const'", nextToken);
      }
    }

    const varToken = parser.peek();
    const isVarDecl = varToken?.type === "IDENT" && parser.peekAheadIs("EQUALS");

    if (!isVarDecl) {
      if (declarations.length === 0) {
        throw parser.errorAt("Expected variable declaration", varToken);
      }
      break;
    }

    parser.consume(); // Consume IDENT
    const varName = varToken!.value;

    parser.expect("EQUALS");
    const valueExpr = parser.parseExpression();
    declarations.push({ name: varName, value: valueExpr });

    if (parser.check("SEMI") || parser.check("COMMA")) {
      parser.consume();
    }
  }

  return {
    type: "state",
    declarations,
    line: atToken.line,
    col: atToken.col,
  };
}
