import type { AtcodeDeclaration } from "../types";
import type { Token } from "../tokenizer";
import { parseStateDeclaration } from "./state";
import { parseEffectDeclaration } from "./effect";
import { parseDerivedDeclaration } from "./derived";

export function parseAtcodeDeclaration(
  parser: any,
  atToken: Token,
): AtcodeDeclaration {
  const nameToken = parser.peek();
  if (!nameToken || nameToken.type !== "IDENT") {
    throw parser.errorAt("Expected identifier after @", atToken);
  }
  const name = parser.consume().value;

  if (name === "state") {
    return parseStateDeclaration(parser, atToken);
  }

  if (name === "effect") {
    return parseEffectDeclaration(parser, atToken);
  }

  if (name === "derived") {
    return parseDerivedDeclaration(parser, atToken);
  }

  // Unknown atcode, try to parse generically
  return {
    type: "effect",
    body: name,
    line: atToken.line,
    col: atToken.col,
  };
}
