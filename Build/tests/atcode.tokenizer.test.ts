import { tokenize } from "../src/parser/tokenizer";
import { describe, test, expect } from "@jest/globals";

describe("Tokenizer - Atcode Extensions", () => {
  test("should tokenize @ token", () => {
    const tokens = tokenize("@state");
    expect(tokens).toEqual([
      { type: "AT", value: "@", line: 1, col: 1 },
      { type: "IDENT", value: "state", line: 1, col: 2 },
    ]);
  });

  test("should tokenize = token", () => {
    const tokens = tokenize("count = 0");
    expect(tokens).toEqual([
      { type: "IDENT", value: "count", line: 1, col: 1 },
      { type: "EQUALS", value: "=", line: 1, col: 7 },
      { type: "IDENT", value: "0", line: 1, col: 9 },
    ]);
  });

  test("should tokenize interpolation in string", () => {
    const tokens = tokenize('text: "Count: {count}"');
    const types = tokens.map(t => t.type);
    expect(types).toContain("INTERP_START");
    expect(types).toContain("EXPR");
    expect(types).toContain("INTERP_END");
    
    const expr = tokens.find(t => t.type === "EXPR");
    expect(expr?.value).toBe("count");
  });

  test("should tokenize multiple interpolations", () => {
    const tokens = tokenize('text: "Hello {name}, you have {count} items"');
    const exprs = tokens.filter(t => t.type === "EXPR");
    expect(exprs).toHaveLength(2);
    expect(exprs[0].value).toBe("name");
    expect(exprs[1].value).toBe("count");
  });

  test("should tokenize nested braces in interpolation", () => {
    const tokens = tokenize('text: "{items.map(x => x.name)}"');
    const expr = tokens.find(t => t.type === "EXPR");
    expect(expr?.value).toBe("items.map(x => x.name)");
  });

  test("should handle standalone @ as invalid syntax", () => {
    const tokens = tokenize("@");
    expect(tokens[0].type).toBe("AT");
  });
});
