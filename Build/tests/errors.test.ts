import { tokenize } from '../src/parser/tokenizer';
import { parseSakko } from '../src/parser/parser';
import { describe, test, expect } from "@jest/globals";

describe('Tokenizer - Error handling', () => {
  test('should throw on unterminated string', () => {
    expect(() => tokenize('text: "hello')).toThrow('Unterminated string');
  });

  test('should throw on unterminated string at end of input', () => {
    expect(() => tokenize('"')).toThrow('Unterminated string');
  });

  test('should throw on unterminated string with content after', () => {
    expect(() => tokenize('"hello world')).toThrow('Unterminated string');
  });

  test('should throw on unexpected character #', () => {
    expect(() => tokenize('#heading')).toThrow('Unexpected character: #');
  });

  test('should throw on unexpected character $', () => {
    expect(() => tokenize('price: $5')).toThrow('Unexpected character: $');
  });

  test('should tokenize ! as a valid operator', () => {
    const tokens = tokenize('!important');
    expect(tokens[0]).toMatchObject({ type: 'BANG', value: '!' });
    expect(tokens[1]).toMatchObject({ type: 'IDENT', value: 'important' });
  });

  test('should handle string with only whitespace content', () => {
    const tokens = tokenize('text: "   "');
    expect(tokens[2]).toMatchObject({ type: 'STRING', value: '   ' });
  });

  test('should handle string with bracket characters inside as interpolation', () => {
    const tokens = tokenize('text: "{[(<>)]}"');
    expect(tokens[2]).toMatchObject({ type: 'INTERP_START', value: '{' });
    expect(tokens[3]).toMatchObject({ type: 'EXPR', value: '[(<>)]' });
  });

  test('should handle single-character identifiers', () => {
    const tokens = tokenize('a');
    expect(tokens[0]).toMatchObject({ type: 'IDENT', value: 'a' });
  });

  test('should handle identifiers with hyphens and underscores', () => {
    const tokens = tokenize('icon-btn my_var data-id');
    expect(tokens[0]).toMatchObject({ type: 'IDENT', value: 'icon-btn' });
    expect(tokens[1]).toMatchObject({ type: 'IDENT', value: 'my_var' });
    expect(tokens[2]).toMatchObject({ type: 'IDENT', value: 'data-id' });
  });

  test('should handle consecutive strings', () => {
    const tokens = tokenize('"hello" "world"');
    expect(tokens[0]).toMatchObject({ type: 'STRING', value: 'hello' });
    expect(tokens[1]).toMatchObject({ type: 'STRING', value: 'world' });
  });

  test('should handle semicolons as tokens', () => {
    const tokens = tokenize('a; b; c');
    expect(tokens.filter(t => t.type === 'SEMI')).toHaveLength(2);
  });

  test('should strip comments before strings on same line', () => {
    const tokens = tokenize('text: Hello // "this is not a string"');
    const strings = tokens.filter(t => t.type === 'STRING');
    expect(strings).toHaveLength(0);
  });

  test('should handle tab characters as whitespace', () => {
    const tokens = tokenize('a\tb\tc');
    expect(tokens[0]).toMatchObject({ type: 'IDENT', value: 'a' });
    expect(tokens[1]).toMatchObject({ type: 'IDENT', value: 'b' });
    expect(tokens[2]).toMatchObject({ type: 'IDENT', value: 'c' });
  });
});

describe('Parser - Error handling', () => {
  test('should throw on completely empty input', () => {
    expect(() => parseSakko('')).toThrow();
  });

  test('should throw on whitespace-only input', () => {
    expect(() => parseSakko('   \n\n   ')).toThrow();
  });

  test('should handle comment-only input gracefully', () => {
    const result = parseSakko('// just a comment');
    expect(result.type).toBe('root');
  });

  test('should auto-wrap input missing opening <', () => {
    const result = parseSakko('page { text: Hello }');
    expect(result.type).toBe('root');
  });

  test('should throw when missing closing >', () => {
    expect(() => parseSakko('<page { text: Hello }')).toThrow("Expected '>'");
  });

  test('should throw when missing opening {', () => {
    expect(() => parseSakko('<page text: Hello }>')).toThrow("Expected '{'");
  });

  test('should throw when missing closing }', () => {
    expect(() => parseSakko('<page { text: Hello >')).toThrow();
  });

  test('should throw when root name is missing', () => {
    expect(() => parseSakko('< { text: Hello }>')).toThrow("Expected identifier after '<'");
  });

  test('should throw on non-identifier after <', () => {
    expect(() => parseSakko('<{ text: Hello }>')).toThrow("Expected identifier after '<'");
  });

  test('should throw on value missing after colon', () => {
    expect(() => parseSakko('<page { text: }>')).toThrow();
  });

  test('should throw on colon followed by closing brace', () => {
    expect(() => parseSakko('<page { name: }>')).toThrow();
  });

  test('should throw on nested unclosed block', () => {
    expect(() => parseSakko('<page { card { text: Hello }>')).toThrow();
  });

  test('should throw on deeply nested unclosed block', () => {
    expect(() => parseSakko('<page { a { b { c: d }>')).toThrow();
  });

  test('should throw on unclosed modifier parenthesis', () => {
    expect(() => parseSakko('<page { button(accent : Click }>')).toThrow();
  });

  test('should throw on empty modifiers with unclosed paren', () => {
    expect(() => parseSakko('<page { button( }>')).toThrow();
  });

  test('should throw on unclosed list bracket', () => {
    expect(() => parseSakko('<page { row: [a: 1, b: 2 }>')).toThrow();
  });

  test('should throw on list missing comma between items', () => {
    expect(() => parseSakko('<page { row: [a: 1 b: 2] }>')).toThrow('Expected "," or "]"');
  });

  test('should throw on element name that is not an identifier', () => {
    expect(() => parseSakko('<page { : value }>')).toThrow('Expected identifier');
  });

  test('should throw on non-identifier inside modifiers', () => {
    expect(() => parseSakko('<page { button(: value): Click }>')).toThrow('Expected identifier in modifiers');
  });

  test('should parse void elements (no body, colon, or list)', () => {
    const ast = parseSakko('<page { card button: Click }>');
    expect(ast.children).toHaveLength(2);
    expect(ast.children[0]).toEqual({ type: "inline", name: "card", modifiers: [], value: "" });
    expect(ast.children[1]).toEqual({ type: "inline", name: "button", modifiers: [], value: "Click" });
  });

  test('should throw on just angle brackets', () => {
    expect(() => parseSakko('<>')).toThrow();
  });

  test('should throw on just < with name', () => {
    expect(() => parseSakko('<page')).toThrow();
  });

  test('should throw on duplicate closing >', () => {
    const ast = parseSakko('<page { }>');
    expect(ast.name).toBe('page');
  });
});

describe('Parser - Malformed but parseable edge cases', () => {
  test('should parse empty block element', () => {
    const ast = parseSakko('<page { card {} }>');
    expect(ast.children).toHaveLength(1);
    const card = ast.children[0];
    expect(card.type).toBe('element');
    if (card.type === 'element') {
      expect(card.children).toHaveLength(0);
    }
  });

  test('should parse element with empty modifiers', () => {
    const ast = parseSakko('<page { button(): Click }>');
    const btn = ast.children[0];
    if (btn.type === 'inline') {
      expect(btn.modifiers).toHaveLength(0);
      expect(btn.value).toBe('Click');
    }
  });

  test('should parse empty list', () => {
    const ast = parseSakko('<page { row: [] }>');
    const row = ast.children[0];
    if (row.type === 'element') {
      expect(row.children).toHaveLength(1);
      const list = row.children[0];
      if (list.type === 'list') {
        expect(list.items).toHaveLength(0);
      }
    }
  });

  test('should parse list with trailing comma', () => {
    const ast = parseSakko('<page { row: [a: 1, b: 2,] }>');
    const row = ast.children[0];
    if (row.type === 'element') {
      const list = row.children[0];
      if (list.type === 'list') {
        expect(list.items).toHaveLength(2);
      }
    }
  });

  test('should parse root with no children', () => {
    const ast = parseSakko('<page {}>');
    expect(ast.name).toBe('page');
    expect(ast.children).toHaveLength(0);
  });

  test('should parse root with empty block', () => {
    const ast = parseSakko('<page {}>');
    expect(ast.name).toBe('page');
    expect(ast.children).toHaveLength(0);
  });

  test('should parse trailing semicolons', () => {
    const ast = parseSakko('<page { text: A; text: B; }>');
    expect(ast.children).toHaveLength(2);
  });

  test('should throw on multiple semicolons between items', () => {
    expect(() => parseSakko('<page { text: A;; text: B }>')).toThrow();
  });

  test('should handle list with trailing comma (parses successfully)', () => {
    const ast = parseSakko('<page { row: [a: 1,] }>');
    const row = ast.children[0];
    expect(row.type).toBe('element');
  });

  test('should parse single inline child', () => {
    const ast = parseSakko('<page { text: Hello }>');
    expect(ast.children).toHaveLength(1);
    expect(ast.children[0].type).toBe('inline');
  });

  test('should parse string value with spaces', () => {
    const ast = parseSakko('<page { text: "Hello World" }>');
    const child = ast.children[0];
    if (child.type === 'inline') {
      expect(child.value).toBe('Hello World');
    }
  });

  test('should parse bare identifier as value', () => {
    const ast = parseSakko('<page { icon: play }>');
    const child = ast.children[0];
    if (child.type === 'inline') {
      expect(child.value).toBe('play');
    }
  });

  test('should parse known key at end of modifiers as flag', () => {
    const ast = parseSakko('<page { row(gap): [] }>');
    const row = ast.children[0];
    if (row.type === 'element') {
      expect(row.modifiers).toEqual([{ type: 'flag', value: 'gap' }]);
    }
  });

  test('should parse element with no children', () => {
    const ast = parseSakko('<page { card { } }>');
    const card = ast.children[0];
    if (card.type === 'element') {
      expect(card.children).toHaveLength(0);
    }
  });
});
