import { readFileSync, readdirSync } from "fs";
import { join } from "path";
import { tokenize } from "../src/parser/tokenizer";
import { parseSakko, Parser } from "../src/parser/parser";

const EXAMPLES_DIR = join(__dirname, "..", "..", "Examples");

describe("Example .sako files", () => {
  const files = readdirSync(EXAMPLES_DIR).filter((f) => f.endsWith(".sako"));

  files.forEach((file) => {
    test(`${file} parses without errors`, () => {
      const source = readFileSync(join(EXAMPLES_DIR, file), "utf-8");
      expect(() => {
        const ast = parseSakko(source);
        expect(ast.type).toBe("root");
        expect(ast.name).toBeTruthy();
        expect(ast.children.length).toBeGreaterThan(0);
      }).not.toThrow();
    });
  });
});

describe("Demo source snippets", () => {
  function parseSnippet(name: string, source: string) {
    test(`${name} parses without errors`, () => {
      expect(() => {
        const ast = parseSakko(source);
        expect(ast.type).toBe("root");
      }).not.toThrow();
    });
  }

  parseSnippet("welcome card", `<demo {
    card {
      heading: "Welcome to Sazami"
      text: "A zero-dependency UI engine."
      row(gap medium): [
        button(primary): "Get Started",
        button(dim): "Learn More"
      ]
    }
  }>`);

  parseSnippet("buttons", `<demo {
    stack(gap medium) {
      row(gap small center): [
        button(primary): "Primary",
        button(secondary): "Secondary",
        button(accent): "Accent",
        button(danger): "Danger",
        button(success): "Success",
        button(dim): "Dim"
      ];
      row(gap small center): [
        button(primary small): "Small",
        button(primary medium): "Medium",
        button(primary large): "Large"
      ];
      row(gap small center): [
        button(disabled): "Disabled",
        button(loading): "Loading..."
      ]
    }
  }>`);

  parseSnippet("forms with string modifiers", `<demo {
    stack(gap medium) {
      row(gap medium): [
        input(placeholder "Your name"): "",
        input(placeholder "you@example.com" type "email"): ""
      ];
      row(gap large center): [
        checkbox: "Accept terms",
        checkbox(checked): "Newsletter"
      ];
      row(gap large center): [
        toggle: "Dark mode",
        toggle(checked): "Notifications"
      ]
    }
  }>`);

  parseSnippet("icons", `<demo {
    stack(gap medium) {
      row(gap medium center): [
        icon: "play",
        icon: "pause",
        icon: "stop",
        icon: "previous",
        icon: "next"
      ];
      row(gap small center): [
        icon(small): "home",
        icon: "home",
        icon(large): "home"
      ];
      row(gap medium center): [
        icon(primary): "heart",
        icon(accent): "heart",
        icon(dim): "heart"
      ]
    }
  }>`);

  parseSnippet("grid layout", `<demo {
    grid(cols 3 gap medium) {
      card { heading: "Analytics"; text: "View your metrics." },
      card(primary) { heading: "Reports"; text: "Generate exports." },
      card { heading: "Settings"; text: "Configure prefs." },
      card(accent) { heading: "Featured"; text(bold): "New!" },
      card { heading: "Billing"; text: "Manage plans." },
      card { heading: "Support"; text: "Get help." }
    }
  }>`);

  parseSnippet("media player", `<demo {
    card {
      row(center gap medium) {
        image(src "https://placehold.co/80x80/2563eb/white?text=Album"): ""
        stack(gap tiny) {
          heading: "Shrine Core"
          text(dim): "Sazami Orchestra"
        }
      };
      row(center gap medium) {
        icon-btn: "previous",
        icon-btn(large primary): "play",
        icon-btn: "next"
      };
      divider: ""
    }
  }>`);
});
