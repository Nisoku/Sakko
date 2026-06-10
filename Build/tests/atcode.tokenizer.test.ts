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

  // --- Edge cases ---

  test("unterminated interpolated string throws", () => {
    // Missing closing quote after the interpolation
    expect(() => tokenize('"Hello {name')).toThrow();
  });

  test("escaped brace does not produce interpolation tokens", () => {
    // \{ is an unknown escape → stored as \{ in the text, no INTERP_START emitted
    const tokens = tokenize('"Hello \\{not interpolation\\}"');
    const types = tokens.map(t => t.type);
    expect(types).not.toContain("INTERP_START");
    expect(types).not.toContain("EXPR");
    expect(types).not.toContain("INTERP_END");
    const str = tokens.find(t => t.type === "STRING");
    // The value should contain the literal brace characters (backslash preserved)
    expect(str?.value).toContain("{");
  });

  test("empty interpolation produces empty EXPR token", () => {
    const tokens = tokenize('"{}"');
    const expr = tokens.find(t => t.type === "EXPR");
    expect(expr).toBeDefined();
    expect(expr?.value).toBe("");
  });

  test("adjacent interpolations produce two EXPR tokens", () => {
    const tokens = tokenize('"{a}{b}"');
    const exprs = tokens.filter(t => t.type === "EXPR");
    expect(exprs).toHaveLength(2);
    expect(exprs[0].value).toBe("a");
    expect(exprs[1].value).toBe("b");
  });

  describe('backtick strings', () => {
    test("tokenizes backtick string as BACKTICK_STRING", () => {
      const tokens = tokenize("`hello world`");
      expect(tokens).toHaveLength(1);
      expect(tokens[0].type).toBe("BACKTICK_STRING");
      expect(tokens[0].value).toBe("hello world");
    });

    test("backtick string with ${} does not produce interpolation tokens", () => {
      const tokens = tokenize("`Count: ${count}`");
      expect(tokens).toHaveLength(1);
      expect(tokens[0].type).toBe("BACKTICK_STRING");
      expect(tokens[0].value).toBe("Count: ${count}");
    });

    test("backtick string with nested template expressions", () => {
      const tokens = tokenize("`${a} + ${b} = ${a + b}`");
      expect(tokens).toHaveLength(1);
      expect(tokens[0].type).toBe("BACKTICK_STRING");
    });

    test("mixed double-quoted and backtick strings", () => {
      const tokens = tokenize('"normal" `template` "also normal"');
      const types = tokens.map(t => t.type);
      expect(types).toEqual(["STRING", "BACKTICK_STRING", "STRING"]);
    });

    test("backtick string inside @effect block body preserves template literal", () => {
      const tokens = tokenize('console.log(`Count: ${count}`)');
      const backtick = tokens.find(t => t.type === "BACKTICK_STRING");
      expect(backtick).toBeDefined();
      expect(backtick!.value).toBe("Count: ${count}");
    });

    test("unterminated backtick string throws", () => {
      expect(() => tokenize("`unclosed")).toThrow();
    });
  });
});
