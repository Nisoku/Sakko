import { tokenize, type Token } from "./tokenizer";
import { parserError } from "../errors";
import { parseAtcodeDeclaration } from "./atcodes";
import { parseInlineModifier } from "./atcodes/modifiers";
import type {
  Modifier,
  InterpolatedTextPart,
  InterpolatedText,
  AtcodeDeclaration,
  RootNode,
  ElementNode,
  InlineNode,
  ListNode,
  ASTNode,
} from "./types";

export type {
  Modifier,
  InterpolatedTextPart,
  InterpolatedText,
  AtcodeDeclaration,
  RootNode,
  ElementNode,
  InlineNode,
  ListNode,
  ASTNode,
};

const KNOWN_KEYS = new Set([
  "cols",
  "gap",
  "radius",
  "md:cols",
  "lg:cols",
  "placeholder",
  "type",
  "size",
  "variant",
  "layout",
  "src",
  "alt",
  "icon",
  "label",
  "value",
  "center-point",
  "min",
  "max",
  "step",
  "name",
  "heading",
  "slot",
  "active",
  "open",
  "message",
  "title",
]);

export class Parser {
  tokens: Token[];
  position: number;
  private source: string;

  constructor(tokens: Token[], source?: string) {
    this.tokens = tokens;
    this.position = 0;
    this.source = source || "";
  }

  errorAt(msg: string, token?: Token): Error {
    parserError(msg, {
      line: token?.line,
      column: token?.col,
      suggestion: this._getSuggestion(msg),
    });
    if (!token || !this.source) return new Error(msg);
    const lines = this.source.split("\n");
    const lineText = lines[token.line - 1] || "";
    const pointer = " ".repeat(Math.max(0, token.col - 1)) + "^";
    return new Error(
      `${msg} at line ${token.line}, col ${token.col}\n  ${lineText}\n  ${pointer}`,
    );
  }

  private _getSuggestion(msg: string): string | undefined {
    if (msg.includes("Unexpected end of input"))
      return "Check for missing closing brackets";
    if (msg.includes("Expected")) return "Add the expected token";
    if (msg.includes("Unexpected token")) return "Remove or replace this token";
    return undefined;
  }

  peek(): Token | undefined {
    return this.tokens[this.position];
  }

  peekAhead(offset: number): Token | undefined {
    return this.tokens[this.position + offset];
  }

  peekAheadIs(type: string): boolean {
    return this.peekAhead(1)?.type === type;
  }

  consume(): Token {
    const token = this.tokens[this.position];
    if (!token) {
      const last = this.tokens[this.tokens.length - 1];
      throw this.errorAt("Unexpected end of input", last);
    }
    this.position++;
    return token;
  }

  check(type: string): boolean {
    return this.peek()?.type === type;
  }

  expect(type: string, errorMsg?: string): Token {
    const token = this.peek();
    if (!token || token.type !== type) {
      const msg =
        errorMsg || `Expected ${type} but got ${token?.type || "end of input"}`;
      throw this.errorAt(msg, token);
    }
    return this.consume();
  }

  parseRoot(): RootNode {
    this.expect("LT", "Expected '<'");

    const nameToken = this.peek();
    if (!nameToken || nameToken.type !== "IDENT") {
      throw this.errorAt("Expected identifier after '<'", nameToken);
    }
    const name = this.consume().value;

    const modifiers = this.check("LPAREN") ? this.parseModifiers() : [];

    this.expect("LBRACE", "Expected '{'");

    const declarations: AtcodeDeclaration[] = [];
    const children: ASTNode[] = [];

    while (!this.check("RBRACE")) {
      if (!this.peek()) {
        throw this.errorAt(
          "Unexpected end of input, expected '}'",
          this.tokens[this.tokens.length - 1],
        );
      }

      if (this.check("AT")) {
        const atPosition = this.position;
        this.consume();
        const nextToken = this.peek();

        if (
          nextToken?.type === "IDENT" &&
          (nextToken.value === "state" ||
            nextToken.value === "effect" ||
            nextToken.value === "derived")
        ) {
          this.position = atPosition;
          const atToken = this.consume();
          declarations.push(parseAtcodeDeclaration(this, atToken));
        } else {
          this.position = atPosition;
          const node = this.parseNode();
          children.push(node);
        }
      } else {
        children.push(this.parseNode());
      }
      if (this.check("SEMI")) this.consume();
      if (this.check("COMMA")) this.consume();
    }

    this.expect("RBRACE", "Expected '}'");
    this.expect("GT", "Expected '>'");

    return { type: "root", name, modifiers, declarations, children };
  }

  parseBlockBody(): string {
    let body = "";
    let braceDepth = 0;

    while (this.peek() && (braceDepth > 0 || !this.check("RBRACE"))) {
      const token = this.peek();

      if (token?.type === "LBRACE") braceDepth++;
      if (token?.type === "RBRACE" && braceDepth > 0) braceDepth--;

      if (braceDepth === 0 && token?.type === "RBRACE") break;

      if (token) {
        if (token.type === "STRING") {
          body += `"${token.value}"`;
        } else {
          body += token.value;
        }
        this.consume();
      }
    }

    return body.trim();
  }

  parseExpression(): string {
    let expr = "";
    let parenDepth = 0;
    let braceDepth = 0;
    let bracketDepth = 0;

    while (this.peek()) {
      const token = this.peek();

      if (parenDepth === 0 && braceDepth === 0 && bracketDepth === 0) {
        if (
          token?.type === "SEMI" ||
          token?.type === "RBRACE" ||
          token?.type === "COMMA"
        ) {
          break;
        }
        if (token?.type === "IDENT" && this.peekAheadIs("EQUALS")) {
          break;
        }
      }

      if (token?.type === "LPAREN") parenDepth++;
      if (token?.type === "RPAREN") parenDepth--;
      if (token?.type === "LBRACE") braceDepth++;
      if (token?.type === "RBRACE") braceDepth--;
      if (token?.type === "LBRACKET") bracketDepth++;
      if (token?.type === "RBRACKET") bracketDepth--;

      if (token) {
        if (expr) {
          const isWordEnd = /[a-zA-Z0-9_$]/.test(expr.slice(-1));
          const isWordStart = /[a-zA-Z0-9_$]/.test(token.value[0]);
          if (isWordEnd && isWordStart) {
            expr += " ";
          }
        }
        expr += token.value;
        this.consume();
      }
    }

    return expr.trim();
  }

  parseNode(): ASTNode {
    const token = this.peek();
    if (!token || token.type !== "IDENT") {
      throw this.errorAt(
        `Expected identifier but got ${token?.type || "end of input"}`,
        token,
      );
    }
    const name = this.consume().value;

    const modifiers: Modifier[] = [];

    while (this.check("LPAREN") || this.check("AT")) {
      if (this.check("LPAREN")) {
        modifiers.push(...this.parseModifiers());
      }

      if (this.check("AT")) {
        this.consume(); // consume @
        modifiers.push(parseInlineModifier(this));
      }
    }

    if (this.check("COLON")) {
      this.consume();

      if (this.check("LBRACKET")) {
        const list = this.parseList();
        return { type: "element", name, modifiers, children: [list] };
      }

      const valToken = this.peek();
      if (!valToken) {
        throw this.errorAt(
          `Expected value after ':' but got end of input`,
          this.tokens[this.tokens.length - 1],
        );
      }

      if (valToken.type === "STRING" || valToken.type === "INTERP_START") {
        const value = this.parseInterpolatedValue();
        return { type: "inline", name, modifiers, value };
      }

      if (valToken.type === "IDENT") {
        const value = this.consume().value;
        return { type: "inline", name, modifiers, value };
      }

      throw this.errorAt(
        `Expected value after ':' but got ${valToken.type || "end of input"}`,
        valToken,
      );
    }

    if (this.check("LBRACKET")) {
      const list = this.parseList();
      return { type: "element", name, modifiers, children: [list] };
    }

    if (this.check("LBRACE")) {
      this.consume();
      const children: ASTNode[] = [];

      while (!this.check("RBRACE")) {
        if (!this.peek()) {
          throw this.errorAt(
            "Unexpected end of input, expected '}'",
            this.tokens[this.tokens.length - 1],
          );
        }
        children.push(this.parseNode());
        if (this.check("SEMI")) this.consume();
        if (this.check("COMMA")) this.consume();
      }

      this.consume();
      return { type: "element", name, modifiers, children };
    }

    // Void element: no colon, braces, or brackets follows.
    // Treat as an inline node with an empty value (e.g. divider, spacer(large)).
    return { type: "inline", name, modifiers, value: "" };
  }

  parseModifiers(): Modifier[] {
    this.consume();
    const modifiers: Modifier[] = [];

    while (!this.check("RPAREN")) {
      if (!this.peek()) {
        throw this.errorAt(
          "Unexpected end of input, expected ')'",
          this.tokens[this.tokens.length - 1],
        );
      }

      if (this.check("AT")) {
        const atToken = this.consume();
        const nameToken = this.expect("IDENT");

        if (nameToken.value === "on") {
          this.expect("COLON");
          const eventToken = this.expect("IDENT");
          const event = eventToken.value;

          let handler: string;

          if (this.check("LBRACE")) {
            this.consume();
            handler = this.parseBlockBody();
            this.expect("RBRACE");
          } else {
            throw this.errorAt(
              "Event handlers must use block syntax: @on:click { ... }",
              this.peek(),
            );
          }

          modifiers.push({
            type: "event",
            event,
            handler,
          });
          continue;
        }

        if (nameToken.value === "bind") {
          this.expect("EQUALS");
          const signal = this.check("STRING")
            ? this.consume().value
            : this.expect("IDENT").value;

          modifiers.push({
            type: "atcode",
            name: "bind",
            body: signal,
          });
          continue;
        }

        throw this.errorAt(
          `Atcode @${nameToken.value} not yet supported in modifiers`,
          nameToken,
        );
      }

      const token = this.peek();
      if (!token || token.type !== "IDENT") {
        throw this.errorAt(
          `Expected identifier in modifiers but got ${token?.type || "end of input"}`,
          token,
        );
      }
      this.consume();

      const next = this.peek();
      if (
        KNOWN_KEYS.has(token.value) &&
        next &&
        (next.type === "IDENT" || next.type === "STRING") &&
        !this.check("RPAREN")
      ) {
        modifiers.push({
          type: "pair",
          key: token.value,
          value: this.consume().value,
        });
      } else {
        modifiers.push({ type: "flag", value: token.value });
      }
    }

    this.consume();
    return modifiers;
  }

  parseList(): ListNode {
    this.consume();
    const items: ASTNode[] = [];

    while (!this.check("RBRACKET")) {
      if (!this.peek()) {
        throw this.errorAt(
          "Unexpected end of input, expected ']'",
          this.tokens[this.tokens.length - 1],
        );
      }
      items.push(this.parseNode());
      if (this.check("COMMA")) {
        this.consume();
      } else if (!this.check("RBRACKET")) {
        throw this.errorAt('Expected "," or "]"', this.peek());
      }
    }

    this.consume();
    return { type: "list", items };
  }

  parseInterpolatedValue(): string | InterpolatedText {
    const parts: InterpolatedTextPart[] = [];

    while (this.check("STRING") || this.check("INTERP_START")) {
      if (this.check("STRING")) {
        const text = this.consume().value;
        if (text) {
          parts.push({ type: "text", value: text });
        }
      }

      if (this.check("INTERP_START")) {
        this.consume();
        const expr = this.expect("EXPR").value;
        parts.push({ type: "expr", value: expr });
        this.expect("INTERP_END");
      }
    }

    if (parts.length === 0) {
      return "";
    }

    if (parts.length === 1 && parts[0].type === "text") {
      return parts[0].value;
    }

    return { type: "interpolated", parts };
  }
}

function stripComments(input: string): string {
  let result = "";
  let i = 0;
  while (i < input.length) {
    if (input[i] === "/" && input[i + 1] === "/") {
      // Only strip if there's a newline after (real comment)
      const hasNewline =
        input.indexOf("\n", i + 2) !== -1 || input.indexOf("\r", i + 2) !== -1;
      if (!hasNewline) {
        // Not a real comment (might be in a string like URL), keep it
        result += input[i];
        i++;
        continue;
      }
      // Skip to newline
      while (i < input.length && input[i] !== "\n" && input[i] !== "\r") {
        i++;
      }
      // Keep the newline
      if (i < input.length) {
        result += input[i];
        i++;
        // Handle CRLF
        if (i < input.length && input[i - 1] === "\r" && input[i] === "\n") {
          result += input[i];
          i++;
        }
      }
      continue;
    }
    result += input[i];
    i++;
  }
  return result;
}

export function parseSakko(input: string): RootNode {
  const trimmed = input.trim();

  // Auto-wrap input that doesn't start with '<' - treat as component body
  if (trimmed && !trimmed.startsWith("<")) {
    input = `<wrapper {\n${trimmed}\n}>`;
  }

  // Debug: log the wrapped input
  // console.log("Parsing:", input);

  const tokens = tokenize(input);
  if (tokens.length === 0) {
    parserError("Empty input", { suggestion: "Add some content to parse" });
    throw new Error("Empty input");
  }
  const parser = new Parser(tokens, input);
  return parser.parseRoot();
}
