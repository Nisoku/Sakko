"use strict";

const path = require("path");
const fs = require("fs");

const outDir = process.argv[2];
if (!outDir) {
  console.error("usage: node node-check.cjs <out-dir>");
  process.exit(2);
}

const gluePath = path.resolve(outDir, "sakko_wasm.js");
if (!fs.existsSync(gluePath)) {
  console.error(`missing wasm-bindgen glue: ${gluePath}`);
  process.exit(2);
}

const sakko = require(gluePath);

let failures = 0;
function check(name, fn) {
  try {
    fn();
    console.log(`ok   ${name}`);
  } catch (err) {
    failures += 1;
    console.error(`FAIL ${name}: ${err.message}`);
  }
}

function assert(cond, msg) {
  if (!cond) throw new Error(msg);
}

check("version()", () => {
  assert(/^\d+\.\d+\.\d+$/.test(sakko.version()), `bad version ${sakko.version()}`);
});

check("tokenize_json()", () => {
  const tokens = JSON.parse(sakko.tokenize_json('<todo { div: "x" }>'));
  assert(Array.isArray(tokens) && tokens.length > 0, "token list empty");
  assert(tokens[0].kind === "LT", `expected LT, got ${tokens[0].kind}`);
});

check("parse_json()", () => {
  const ast = JSON.parse(sakko.parse_json("<app { div { span: \"y\" } }>"));
  assert(ast.name === "app", `expected app root, got ${ast.name}`);
  assert(ast.children[0].type === "element", "first child not an element");
  assert(ast.children[0].name === "div", "first child not a div");
});

check("check_json() clean", () => {
  const report = JSON.parse(
    sakko.check_json('<counter { @state { count = 0 } text: "{count}" }>')
  );
  assert(report.ok === true, "expected clean report");
  assert(report.diagnostics.length === 0, "expected no diagnostics");
});

check("check_json() diagnostics", () => {
  const report = JSON.parse(sakko.check_json('<app { div @class={5}: "x" }>'));
  assert(report.ok === false, "expected failing report");
  const first = report.diagnostics[0];
  assert(first.code === "SKT015", `expected SKT015, got ${first.code}`);
  assert(first.severity === "error", `expected error severity`);
  assert(first.rendered.includes("SKT015"), "rendered() missing code");
});

check("check_json() record js escapes", () => {
  const report = JSON.parse(
    sakko.check_json(
      '<app { @state { screen = js { return 1 } as number } text: "{screen}" }>'
    )
  );
  assert(report.jsEscapes.length === 1, "expected one recorded escape");
});

check("check_json() throws on document error", () => {
  let threw = false;
  try {
    sakko.check_json("<unclosed {");
  } catch (err) {
    const payload = JSON.parse(err.message);
    assert(payload.message, "error payload missing message");
    threw = true;
  }
  assert(threw, "expected a thrown Error");
});

if (failures > 0) {
  console.error(`${failures} check(s) failed`);
  process.exit(1);
}
console.log("all checks passed");