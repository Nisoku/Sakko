import { parseSakko } from "../src/parser/parser";
import { compileComponent } from "../src/compiler/component";
import { tokenize } from "../src/parser/tokenizer";
import { describe, test, expect } from "@jest/globals";

describe("Compiler - Atcode Compilation", () => {
  test("compiles @state to signal", () => {
    const input = `<counter {
      @state {
        count = 0
      }
      text: "Count"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast, { sairinImport: "esm" });
    
    expect(compiled).toContain('import { signal, effect, derived, path }');
    expect(compiled).toContain('signal(path("component"');
    expect(compiled).toContain('"count"), 0)');
  });

  test("compiles @effect to effect", () => {
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
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('effect(() => {');
    expect(compiled).toContain('console.log("Count:",count.get())');
  });

  test("compiles @derived to derived", () => {
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
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('derived(path("component"');
    expect(compiled).toContain('items.get().length');
  });

  test("compiles @on:click to addEventListener", () => {
    const input = `<app {
      @state {
        count = 0
      }
      
      button @on:click {
        count++
      }
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('bindEvent(button0, "click", (e) => {');
    expect(compiled).toContain('count.set');
  });

  test("compiles event handler with e.target.value", () => {
    const input = `<app {
      @state {
        value = ""
      }
      input @on:input { value = e.target.value }
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast, { sairinImport: "esm" });
    
    expect(compiled).toContain('bindEvent(input0, "input", (e) => {');
    expect(compiled).toContain('value.set(e.target.value)');
    expect(compiled).not.toContain('value.get()'); // RHS should not have get() for e.target.value
  });

  test("compiles compound assignment", () => {
    const input = `<app {
      @state {
        count = 0
        step = 1
      }
      
      button @on:click {
        count += step
      }: "+"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('count.set(count.get() + step.get())');
  });

  test("compiles @bind to input binding", () => {
    const input = `<app {
      input @bind="username": ""
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('bindInputValue(input0, username);');
  });

  test("compiles interpolation to effect", () => {
    const input = `<app {
      @state {
        count = 0
      }
      
      text: "Count: {count}"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain("document.createElement('sakko-text');");
    expect(compiled).toContain(".textContent =");
    expect(compiled).toContain('count.get()');
  });

  test("generates valid component function", () => {
    const input = `<counter {
      @state {
        count = 0
      }
      text: "Count"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('export function Counter');
    expect(compiled).toContain('return root;');
  });

  test("generates getSignal export", () => {
    const input = `<counter {
      @state {
        count = 0
        step = 1
      }
      text: "Count"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('const instanceSignals = new Map();');
    expect(compiled).toContain('export function getSignal(id, signalName)');
    expect(compiled).toContain('const signals = instanceSignals.get(id);');
  });

  test("compiles indexed signal access", () => {
    const input = `<app {
      @state {
        list = [1, 2, 3]
      }
      text: "{list[0]}"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('list.get()[0]');
  });

  test("static text with backslash is escaped in generated template literal", () => {
    const input = `<app {
      text: "line1\\nline2"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);

    // The text content should be present in the output (escaped as a JSON string or template literal)
    expect(compiled).toContain("line1");
    expect(compiled).toContain("line2");
  });
});
