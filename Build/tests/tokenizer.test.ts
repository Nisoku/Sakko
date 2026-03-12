import { tokenize } from "../src/parser/tokenizer";

describe("Tokenizer", () => {
  test("should tokenize basic elements", () => {
    const input = "card { text: Hello }";
    const tokens = tokenize(input);

    expect(tokens).toEqual([
      { type: "IDENT", value: "card", line: 1, col: 1 },
      { type: "LBRACE", value: "{", line: 1, col: 6 },
      { type: "IDENT", value: "text", line: 1, col: 8 },
      { type: "COLON", value: ":", line: 1, col: 12 },
      { type: "IDENT", value: "Hello", line: 1, col: 14 },
      { type: "RBRACE", value: "}", line: 1, col: 20 },
    ]);
  });

  test("should tokenize root block", () => {
    const input = '<page { card { text: "Hello World" } }>';
    const tokens = tokenize(input);

    expect(tokens[0].type).toBe("LT");
    expect(tokens[1]).toMatchObject({ type: "IDENT", value: "page" });
    expect(tokens[7]).toMatchObject({ type: "STRING", value: "Hello World" });
    expect(tokens[tokens.length - 1].type).toBe("GT");
  });

  test("should tokenize modifiers", () => {
    const tokens = tokenize("button(accent large): Save");

    expect(tokens[0]).toMatchObject({ type: "IDENT", value: "button" });
    expect(tokens[1]).toMatchObject({ type: "LPAREN" });
    expect(tokens[2]).toMatchObject({ type: "IDENT", value: "accent" });
    expect(tokens[3]).toMatchObject({ type: "IDENT", value: "large" });
    expect(tokens[4]).toMatchObject({ type: "RPAREN" });
  });

  test("should tokenize key-value modifiers", () => {
    const tokens = tokenize("grid(cols 3 gap medium): [ item ]");

    expect(tokens[2]).toMatchObject({ type: "IDENT", value: "cols" });
    expect(tokens[3]).toMatchObject({ type: "IDENT", value: "3" });
    expect(tokens[4]).toMatchObject({ type: "IDENT", value: "gap" });
    expect(tokens[5]).toMatchObject({ type: "IDENT", value: "medium" });
  });

  test("should tokenize lists and semicolons", () => {
    const tokens = tokenize("controls { button: play; button: pause; button: stop }");

    expect(tokens.filter(t => t.type === "SEMI")).toHaveLength(2);
    expect(tokens[0]).toMatchObject({ type: "IDENT", value: "controls" });
  });

  test("should handle comments", () => {
    const input = `
      // This is a comment
      card { 
        // Another comment
        text: Hello 
      }
    `;
    const tokens = tokenize(input);
    const idents = tokens.filter(t => t.type === "IDENT");

    expect(idents.map(t => t.value)).toEqual(["card", "text", "Hello"]);
  });

  test("should handle identifiers with hyphens and underscores", () => {
    const tokens = tokenize('custom-button_name: "Test value"');

    expect(tokens[0]).toMatchObject({ type: "IDENT", value: "custom-button_name" });
    expect(tokens[2]).toMatchObject({ type: "STRING", value: "Test value" });
  });

  test("should throw error on unterminated string", () => {
    expect(() => tokenize('text: "unclosed string')).toThrow("Unterminated string");
  });

  test("should throw error on unexpected character", () => {
    expect(() => tokenize("text: @invalid")).toThrow("Unexpected character: @");
  });

  test("should preserve URLs inside strings", () => {
    const tokens = tokenize('image: "https://example.com/photo.jpg"');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str?.value).toBe("https://example.com/photo.jpg");
  });

  test("should not treat // inside strings as comments", () => {
    const tokens = tokenize('coverart: "https://placehold.co/80"');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str?.value).toBe("https://placehold.co/80");
  });

  test("should track line numbers", () => {
    const tokens = tokenize("a\nb\nc");
    expect(tokens[0].line).toBe(1);
    expect(tokens[1].line).toBe(2);
    expect(tokens[2].line).toBe(3);
  });

  test("handles backslash in strings (no escape processing)", () => {
    const tokens = tokenize('text: "hello\\nworld"');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str?.value).toBe("hello\\nworld");
  });

  test("handles empty strings", () => {
    const tokens = tokenize('input: ""');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str?.value).toBe("");
  });

  test("throws on unterminated string", () => {
    expect(() => tokenize('text: "hello')).toThrow(/Unterminated string/);
  });

  test("tokenizes angle brackets", () => {
    const tokens = tokenize("<page>");
    expect(tokens[0]).toMatchObject({ type: "LT", value: "<" });
    expect(tokens[2]).toMatchObject({ type: "GT", value: ">" });
  });

  test("tokenizes commas", () => {
    const tokens = tokenize("a, b, c");
    const commas = tokens.filter((t) => t.type === "COMMA");
    expect(commas).toHaveLength(2);
  });

  test("handles multiline strings with comments", () => {
    const input = '// comment\ntext: "value"';
    const tokens = tokenize(input);
    const ident = tokens.find((t) => t.type === "IDENT" && t.value === "text");
    expect(ident).toBeTruthy();
    expect(ident?.line).toBe(2);
  });

  test("handles string with colon inside", () => {
    const tokens = tokenize('text: "key: value"');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str?.value).toBe("key: value");
  });

  test("handles string with braces inside", () => {
    const tokens = tokenize('text: "{ hello }"');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str?.value).toBe("{ hello }");
  });
});
