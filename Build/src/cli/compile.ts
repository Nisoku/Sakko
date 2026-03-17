#!/usr/bin/env node

import { readFileSync, writeFileSync } from "fs";
import { parseSakko } from "../parser/parser";
import { compileComponent } from "../compiler/component";

const args = process.argv.slice(2);

if (args.length < 2) {
  console.error("Usage: sakko-compile <input.sako> <output.js>");
  process.exit(1);
}

const [inputFile, outputFile] = args;

try {
  const source = readFileSync(inputFile, "utf-8");
  const ast = parseSakko(source);
  const compiled = compileComponent(ast);
  
  writeFileSync(outputFile, compiled, "utf-8");
  console.log(`Compiled ${inputFile} -> ${outputFile}`);
} catch (err) {
  console.error("Compilation failed:");
  console.error(err);
  process.exit(1);
}
