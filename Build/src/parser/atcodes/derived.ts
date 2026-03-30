import type { AtcodeDeclaration, ParserState } from "../types";
import type { Token } from "../tokenizer";

export function parseDerivedDeclaration(
  parser: ParserState,
  atToken: Token,
): AtcodeDeclaration {
  const hasBraces = parser.check("LBRACE");
  if (hasBraces) parser.consume();

  const declarations: Array<{ name: string; expr: string }> = [];

  while (true) {
    if (!parser.peek()) break;

    if (hasBraces && parser.check("RBRACE")) {
      parser.consume();
      break;
    }

    if (parser.check("IDENT") && parser.peek()?.value === "const") {
      parser.consume();
    }

    const varToken = parser.peek();
    if (!varToken || varToken.type !== "IDENT") {
      break;
    }

    parser.consume();
    const varName = varToken.value;

    if (parser.check("EQUALS")) {
      parser.consume();
      const expr = parser.parseExpression();
      declarations.push({ name: varName, expr });

      if (parser.check("SEMI") || parser.check("COMMA")) {
        parser.consume();
      }
    } else {
      break;
    }
  }

  return {
    type: "derived",
    declarations,
    line: atToken.line,
    col: atToken.col,
  };
}
