import { parseSakko } from "../src/parser/parser";

describe("Parser", () => {
  test("should parse simple root block", () => {
    const ast = parseSakko("<page { text: Hello }>");

    expect(ast.type).toBe("root");
    expect(ast.name).toBe("page");
    expect(ast.children).toHaveLength(1);
    expect(ast.children[0]).toMatchObject({
      type: "inline",
      name: "text",
      modifiers: [],
      value: "Hello",
    });
  });

  test("should parse nested elements", () => {
    const ast = parseSakko("<page { card { text: Hello; button: Click } }>");

    expect(ast.children).toHaveLength(1);
    const card = ast.children[0];
    expect(card.type).toBe("element");
    if (card.type === "element") {
      expect(card.children).toHaveLength(2);
    }
  });

  test("should parse modifiers", () => {
    const ast = parseSakko("<page { button(accent large): Save }>");

    const btn = ast.children[0];
    if (btn.type === "inline") {
      expect(btn.modifiers).toEqual([
        { type: "flag", value: "accent" },
        { type: "flag", value: "large" },
      ]);
      expect(btn.value).toBe("Save");
    }
  });

  test("should parse key-value modifiers", () => {
    const ast = parseSakko(
      "<page { grid(cols 3 gap medium): [ card { text: One }, card { text: Two } ] }>"
    );

    const grid = ast.children[0];
    if (grid.type === "element") {
      expect(grid.modifiers).toEqual([
        { type: "pair", key: "cols", value: "3" },
        { type: "pair", key: "gap", value: "medium" },
      ]);
    }
  });

  test("should handle mixed flag and key-value modifiers", () => {
    const ast = parseSakko('<page { text(bold dim): "Label" }>');

    const text = ast.children[0];
    if (text.type === "inline") {
      expect(text.modifiers).toEqual([
        { type: "flag", value: "bold" },
        { type: "flag", value: "dim" },
      ]);
    }
  });

  test("should parse complex nested structure", () => {
    const ast = parseSakko(`
      <player {
        card(row medium center curved) {
          coverart(round): "album.jpg"
          details {
            text(bold): "Midnight City"
            text(dim small): "M83"
          }
          controls {
            icon-btn: play;
            icon-btn: skip;
            badge(accent): LIVE
          }
        }
      }>
    `);

    expect(ast.name).toBe("player");
    expect(ast.children).toHaveLength(1);
    const card = ast.children[0];
    if (card.type === "element") {
      expect(card.name).toBe("card");
      expect(card.children).toHaveLength(3);
    }
  });

  test("should parse list with modifiers", () => {
    const ast = parseSakko(
      "<page { row(center): [ button: One, button: Two, button: Three ] }>"
    );

    const row = ast.children[0];
    if (row.type === "element") {
      expect(row.modifiers).toEqual([{ type: "flag", value: "center" }]);
      const list = row.children[0];
      if (list.type === "list") {
        expect(list.items).toHaveLength(3);
      }
    }
  });

  test("should throw error on malformed input", () => {
    expect(() => parseSakko("page { text: Hello }")).toThrow();
    expect(() => parseSakko("<page text: Hello }")).toThrow();
    expect(() => parseSakko("<page { text: Hello ")).toThrow();
  });

  test("should handle quoted strings correctly", () => {
    const ast = parseSakko('<page { text: "Hello World with spaces" }>');

    const child = ast.children[0];
    if (child.type === "inline") {
      expect(child.value).toBe("Hello World with spaces");
    }
  });

  test("should parse empty block", () => {
    const ast = parseSakko("<page { card {} }>");
    expect(ast.children).toHaveLength(1);
    const card = ast.children[0];
    if (card.type === "element") {
      expect(card.children).toHaveLength(0);
    }
  });

  test("error messages include line and column info", () => {
    expect(() => parseSakko("<page {\n  text:\n}>")).toThrow(/line/);
  });

  test("error messages include source snippet", () => {
    try {
      parseSakko("<page {\n  text:\n}>");
      fail("should have thrown");
    } catch (e: any) {
      expect(e.message).toContain("Expected value after ':'");
      expect(e.message).toContain("line 3");
    }
  });

  test("handles URL strings without treating // as comments", () => {
    const ast = parseSakko('<page { image: "https://example.com/img.png" }>');
    expect(ast.children).toHaveLength(1);
    const img = ast.children[0];
    if (img.type === "inline") {
      expect(img.value).toBe("https://example.com/img.png");
    }
  });

  test("supports string values in modifiers", () => {
    const ast = parseSakko('<page { input(placeholder "Enter your name"): "" }>');
    const node = ast.children[0];
    expect(node.type).toBe("inline");
    if (node.type === "inline") {
      expect(node.modifiers).toEqual([
        { type: "pair", key: "placeholder", value: "Enter your name" },
      ]);
    }
  });

  test("supports mixed string and ident modifier values", () => {
    const ast = parseSakko('<page { input(placeholder "Email" type email): "" }>');
    const node = ast.children[0];
    if (node.type === "inline") {
      expect(node.modifiers).toEqual([
        { type: "pair", key: "placeholder", value: "Email" },
        { type: "pair", key: "type", value: "email" },
      ]);
    }
  });

  test("handles deeply nested structures", () => {
    const ast = parseSakko(`<page {
      card {
        row {
          column {
            text: "Deep"
          }
        }
      }
    }>`);
    expect(ast.children).toHaveLength(1);
    const card = ast.children[0];
    if (card.type === "element") {
      expect(card.children).toHaveLength(1);
      const row = card.children[0];
      if (row.type === "element") {
        expect(row.children).toHaveLength(1);
      }
    }
  });

  test("handles multiple semicolons on same line", () => {
    const ast = parseSakko("<page { text: A; text: B; text: C }>");
    expect(ast.children).toHaveLength(3);
  });

  test("handles list inside block with trailing comma", () => {
    const ast = parseSakko("<page { row: [text: A, text: B] }>");
    const row = ast.children[0];
    if (row.type === "element") {
      expect(row.children).toHaveLength(1);
      const list = row.children[0];
      if (list.type === "list") {
        expect(list.items).toHaveLength(2);
      }
    }
  });

  test("handles empty root", () => {
    const ast = parseSakko("<page {}>");
    expect(ast.name).toBe("page");
    expect(ast.children).toHaveLength(0);
  });

  test("handles element with only modifiers and no children (colon + value)", () => {
    const ast = parseSakko('<page { badge(accent): "NEW" }>');
    const badge = ast.children[0];
    expect(badge.type).toBe("inline");
    if (badge.type === "inline") {
      expect(badge.value).toBe("NEW");
      expect(badge.modifiers).toEqual([{ type: "flag", value: "accent" }]);
    }
  });

  test("handles many modifier flags", () => {
    const ast = parseSakko("<page { card(row medium center curved disabled) {} }>");
    const card = ast.children[0];
    if (card.type === "element") {
      expect(card.modifiers).toHaveLength(5);
    }
  });

  test("throws on double semicolons", () => {
    expect(() => parseSakko("<page { text: A;; text: B }>")).toThrow();
  });

  test("throws on missing closing angle bracket", () => {
    expect(() => parseSakko("<page { text: A }")).toThrow();
  });

  test("throws on missing opening angle bracket", () => {
    expect(() => parseSakko("page { text: A }>")).toThrow();
  });

  test("parses root with modifiers", () => {
    const ast = parseSakko("<stack(gap medium) { text: A }>");
    expect(ast.name).toBe("stack");
    expect(ast.modifiers).toHaveLength(1);
    expect(ast.modifiers![0]).toEqual({ type: "pair", key: "gap", value: "medium" });
  });

  test("parses root with flag modifiers", () => {
    const ast = parseSakko("<card(row center curved) { text: Hello }>");
    expect(ast.modifiers).toHaveLength(3);
    expect(ast.modifiers![0]).toEqual({ type: "flag", value: "row" });
    expect(ast.modifiers![1]).toEqual({ type: "flag", value: "center" });
    expect(ast.modifiers![2]).toEqual({ type: "flag", value: "curved" });
  });

  test("parses void elements", () => {
    const ast = parseSakko("<page { divider }>");
    expect(ast.children).toHaveLength(1);
    expect(ast.children[0]).toEqual({ type: "inline", name: "divider", modifiers: [], value: "" });
  });

  test("parses void elements with modifiers", () => {
    const ast = parseSakko("<page { spacer(large) }>");
    expect(ast.children).toHaveLength(1);
    const child = ast.children[0] as any;
    expect(child.name).toBe("spacer");
    expect(child.modifiers).toHaveLength(1);
    expect(child.modifiers[0]).toEqual({ type: "flag", value: "large" });
  });

  test("allows commas as separators in braces", () => {
    const ast = parseSakko("<page { card { text: A }, card { text: B } }>");
    expect(ast.children).toHaveLength(2);
  });

  test("allows mixed semicolons and commas", () => {
    const ast = parseSakko("<page { text: A; text: B, text: C }>");
    expect(ast.children).toHaveLength(3);
  });

  test("parses multiple void elements in sequence", () => {
    const ast = parseSakko("<page { divider spacer divider }>");
    expect(ast.children).toHaveLength(3);
    expect(ast.children.map((c: any) => c.name)).toEqual(["divider", "spacer", "divider"]);
  });

  test("parses string modifier with special characters", () => {
    const ast = parseSakko('<page { input(placeholder "Hello, World! (test)"): "" }>');
    const child = ast.children[0] as any;
    expect(child.modifiers).toHaveLength(1);
    expect(child.modifiers[0]).toEqual({ type: "pair", key: "placeholder", value: "Hello, World! (test)" });
  });

  test("parses URL in string value", () => {
    const ast = parseSakko('<page { image(src "https://example.com/img.jpg"): "" }>');
    const child = ast.children[0] as any;
    expect(child.modifiers[0].value).toBe("https://example.com/img.jpg");
  });

  test("parses deeply nested void elements", () => {
    const ast = parseSakko("<page { card { text: Title; divider; text: Body } }>");
    const card = ast.children[0] as any;
    expect(card.children).toHaveLength(3);
    expect(card.children[1].name).toBe("divider");
    expect(card.children[1].value).toBe("");
  });
});
