import { tokenize } from "./tokenizer";

export type Modifier =
  | { type: "flag"; value: string }
  | { type: "pair"; key: string; value: string };

export type RootNode = {
  type: "root";
  name: string;
  modifiers: Modifier[];
  children: ASTNode[];
};

export type ElementNode = {
  type: "element";
  name: string;
  modifiers: Modifier[];
  children: ASTNode[];
};

export type InlineNode = {
  type: "inline";
  name: string;
  modifiers: Modifier[];
  value: string;
};

export type ListNode = {
  type: "list";
  items: ASTNode[];
};

export type ASTNode = ElementNode | InlineNode | ListNode;

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

  private errorAt(msg: string, token?: Token): Error {
    if (!token || !this.source) return new Error(msg);
    const lines = this.source.split("\n");
    const lineText = lines[token.line - 1] || "";
    const pointer = " ".repeat(Math.max(0, token.col - 1)) + "^";
    return new Error(
      `${msg} at line ${token.line}, col ${token.col}\n  ${lineText}\n  ${pointer}`,
    );
  }

  peek(): Token | undefined {
    return this.tokens[this.position];
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

    // Root element can have modifiers: <stack(gap medium) { ... }>
    const modifiers = this.check("LPAREN") ? this.parseModifiers() : [];

    this.expect("LBRACE", "Expected '{'");

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

    this.expect("RBRACE", "Expected '}'");
    this.expect("GT", "Expected '>'");

    return { type: "root", name, modifiers, children };
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

    const modifiers = this.check("LPAREN") ? this.parseModifiers() : [];

    if (this.check("COLON")) {
      this.consume();

      if (this.check("LBRACKET")) {
        const list = this.parseList();
        return { type: "element", name, modifiers, children: [list] };
      }

      const valToken = this.peek();
      if (
        !valToken ||
        (valToken.type !== "IDENT" && valToken.type !== "STRING")
      ) {
        throw this.errorAt(
          `Expected value after ':' but got ${valToken?.type || "end of input"}`,
          valToken,
        );
      }
      const value = this.consume().value;
      return { type: "inline", name, modifiers, value };
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
}

export function parseSakko(input: string): RootNode {
  const tokens = tokenize(input);
  if (tokens.length === 0) {
    throw new Error("Empty input");
  }
  const parser = new Parser(tokens, input);
  return parser.parseRoot();
}
