import { parseSakko } from "../src/parser/parser";
import { compileComponent } from "../src/compiler/component";

describe("Compiler - Atcode Compilation", () => {
  test("compiles @state to signal", () => {
    const input = `<counter {
      @state {
        count = 0
      }
      text: "Count"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
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
    expect(compiled).toContain('items.length');
  });

  test("compiles @on:click to addEventListener", () => {
    const input = `<app {
      @state {
        count = 0
      }
      
      button @on:click {
        count++
      }: "+"
    }>`;

    const ast = parseSakko(input);
    const compiled = compileComponent(ast);
    
    expect(compiled).toContain('addEventListener("click"');
    expect(compiled).toContain('count.set(count.get() + 1)');
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
    
    expect(compiled).toContain('effect(() => {');
    expect(compiled).toContain('element.value = String(username.get())');
    expect(compiled).toContain('element.addEventListener("input"');
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
    
    expect(compiled).toContain('effect(() => {');
    expect(compiled).toContain('textContent =');
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
    
    expect(compiled).toContain('export function getSignal');
    expect(compiled).toContain('case "count"');
    expect(compiled).toContain('case "step"');
  });
});
