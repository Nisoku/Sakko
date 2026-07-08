import { parseSakko } from "../src/parser/parser";
import { describe, test, expect } from "@jest/globals";

describe("Parser - Atcode Declarations", () => {
  test("parses @state declaration", () => {
    const input = `<counter {
      @state {
        count = 0
        step = 1
      }
      text: "Count"
    }>`;

    const ast = parseSakko(input);
    
    expect(ast.declarations).toHaveLength(1);
    expect(ast.declarations[0].type).toBe("state");
    expect(ast.declarations[0].declarations).toEqual([
      { name: "count", value: "0" },
      { name: "step", value: "1" },
    ]);
  });

  test("parses @effect declaration", () => {
    const input = `<app {
      @state {
        count = 0
      }
      
      @effect {
        console.log("Count:", count)
      }
      
      text: "App"
    }>`;

    const ast = parseSakko(input);
    
    expect(ast.declarations).toHaveLength(2);
    expect(ast.declarations[1].type).toBe("effect");
    expect(ast.declarations[1].body).toBe('console.log("Count:",count)');
  });

  test("parses @derived declaration", () => {
    const input = `<app {
      @state {
        items = []
      }
      
      @derived {
        count = items.length
      }
      
      text: "App"
    }>`;

    const ast = parseSakko(input);
    
    expect(ast.declarations).toHaveLength(2);
    expect(ast.declarations[1].type).toBe("derived");
    expect(ast.declarations[1].declarations).toEqual([
      { name: "count", expr: "items.length" },
    ]);
  });

  test("parses @on:event modifier", () => {
    const input = `<app {
      @state {
        count = 0
      }
      
      button @on:click {
        count++
      }: "Increment"
    }>`;

    const ast = parseSakko(input);
    const button = ast.children[0];
    
    expect(button.type).toBe("inline");
    expect(button.modifiers).toContainEqual({
      type: "event",
      event: "click",
      handler: "count++",
    });
  });

  test("parses @bind modifier", () => {
    const input = `<app {
      input @bind="username": ""
    }>`;

    const ast = parseSakko(input);
    const inputNode = ast.children[0];
    
    expect(inputNode.type).toBe("inline");
    expect(inputNode.modifiers).toContainEqual({
      type: "atcode",
      name: "bind",
      body: "username",
    });
  });

  test("parses interpolated string", () => {
    const input = `<app {
      @state {
        name = "Alice"
      }
      
      text: "Hello, {name}!"
    }>`;

    const ast = parseSakko(input);
    const text = ast.children[0];
    
    expect(text.type).toBe("inline");
    expect(text.value).toEqual({
      type: "interpolated",
      parts: [
        { type: "text", value: "Hello, " },
        { type: "expr", value: "name" },
        { type: "text", value: "!" },
      ],
    });
  });

  test("parses mixed text and interpolation", () => {
    const input = `<app {
      @state {
        a = 1
        b = 2
      }
      
      text: "{a} + {b} = {a + b}"
    }>`;

    const ast = parseSakko(input);
    const text = ast.children[0];
    
    expect(text.value).toEqual({
      type: "interpolated",
      parts: [
        { type: "expr", value: "a" },
        { type: "text", value: " + " },
        { type: "expr", value: "b" },
        { type: "text", value: " = " },
        { type: "expr", value: "a + b" },
      ],
    });
  });

  test("throws on unknown atcode", () => {
    const input = `<app {
      @unknown {
        foo = bar
      }
    }>`;

    expect(() => parseSakko(input)).toThrow("Unknown atcode '@unknown'");
  });

  test("throws on @on without block", () => {
    const input = `<app {
      button @on:click: "Click"
    }>`;

    expect(() => parseSakko(input)).toThrow("Event handlers must use block syntax");
  });

  test("throws on malformed @state declaration", () => {
    const input = `<app {
      @state {
        invalid_no_equals
      }
    }>`;

    expect(() => parseSakko(input)).toThrow("Expected variable declaration");
  });

  test("parses @effect with backtick template literal", () => {
    const input = `<app {
      @state { count = 0 }
      @effect {
        document.title = \`Count: \${count}\`
      }
      text: "App"
    }>`;

    const ast = parseSakko(input);
    const effectDecl = ast.declarations.find(d => d.type === "effect");
    expect(effectDecl).toBeDefined();
    if (effectDecl?.type === "effect") {
      expect(effectDecl.body).toContain("`Count:");
      expect(effectDecl.body).toContain("${count}");
    }
  });

  test("parses @style modifier as atcode", () => {
    const input = `<page { button(@style "color: red"): "Click" }>`;
    const ast = parseSakko(input);
    const btn = ast.children[0] as any;
    expect(btn.modifiers).toContainEqual({
      type: "atcode",
      name: "style",
      body: "color: red",
    });
  });

  test("parses @if modifier as atcode", () => {
    const input = `<page { button(@if="isVisible"): "Click" }>`;
    const ast = parseSakko(input);
    const btn = ast.children[0] as any;
    expect(btn.modifiers).toContainEqual({
      type: "atcode",
      name: "if",
      body: "isVisible",
    });
  });

  test("parses @if with identifier (no quotes)", () => {
    const input = `<page { button(@if=isVisible): "Click" }>`;
    const ast = parseSakko(input);
    const btn = ast.children[0] as any;
    expect(btn.modifiers).toContainEqual({
      type: "atcode",
      name: "if",
      body: "isVisible",
    });
  });
});
