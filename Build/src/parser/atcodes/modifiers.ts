import type { Modifier, ParserState } from "../types";
import type { Token } from "../tokenizer";

function parseEventHandler(
  parser: ParserState,
  eventName: string,
  eventToken?: Token,
): string {
  if (parser.check("LBRACE")) {
    parser.consume();
    const handler = parser.parseBlockBody();
    parser.expect("RBRACE");
    return handler;
  }
  throw parser.errorAt(
    `Event handlers must use block syntax: @on:${eventName} { ... }`,
    eventToken || parser.peek(),
  );
}

const EVENT_NAMES = new Set([
  "click",
  "mouseenter",
  "mouseleave",
  "keydown",
  "keyup",
  "input",
  "change",
  "submit",
  "focus",
  "blur",
  "dblclick",
  "mousedown",
  "mouseup",
  "drag",
  "drop",
  "touchstart",
  "touchend",
]);

export function parseInlineModifier(parser: ParserState): Modifier {
  const nameToken = parser.expect("IDENT");
  const name = nameToken.value;

  // @on:event { ... }
  if (name === "on") {
    parser.expect("COLON");
    const eventToken = parser.expect("IDENT");
    const event = eventToken.value;

    const handler = parseEventHandler(parser, event, eventToken);

    return {
      type: "event",
      event,
      handler,
    };
  }

  // @on:eventName (shorthand)
  if (EVENT_NAMES.has(name)) {
    const event = name;
    const handler = parser.check("IDENT")
      ? parser.consume().value
      : parseEventHandler(parser, event, nameToken);

    return {
      type: "event",
      event,
      handler,
    };
  }

  // @class:classname
  if (name === "class") {
    parser.expect("COLON");
    const classToken = parser.expect("IDENT");

    return {
      type: "atcode",
      name: "class",
      body: classToken.value,
    };
  }

  // @bind="signal"
  if (name === "bind") {
    parser.expect("EQUALS");
    const signal = parser.check("STRING")
      ? parser.consume().value
      : parser.expect("IDENT").value;

    return {
      type: "atcode",
      name: "bind",
      body: signal,
    };
  }

  // @style "css-string" | @style { ... }
  if (name === "style") {
    if (parser.check("STRING")) {
      const value = parser.consume().value;
      return { type: "atcode", name: "style", body: value };
    }
    if (parser.check("LBRACE")) {
      parser.consume();
      const body = parser.parseBlockBody();
      parser.expect("RBRACE");
      return { type: "atcode", name: "style", body };
    }
    throw parser.errorAt(`@style requires a string or object body`, nameToken);
  }

  // @if "signalName"
  if (name === "if") {
    parser.expect("EQUALS");
    const signal = parser.check("STRING")
      ? parser.consume().value
      : parser.expect("IDENT").value;

    return {
      type: "atcode",
      name: "if",
      body: signal,
    };
  }

  // @each item in source (inline, limited: source is signal name)
  if (name === "each") {
    const item = parser.expect("IDENT");
    const inToken = parser.expect("IDENT", "Expected 'in' in @each expression");
    if (inToken.value !== "in") {
      throw parser.errorAt(
        `Expected 'in' in @each expression, got '${inToken.value}'`,
        inToken,
      );
    }
    const source = parser.expect("IDENT");
    return {
      type: "atcode",
      name: "each",
      body: `${item.value} in ${source.value}`,
    };
  }

  throw parser.errorAt(`Unknown modifier: @${name}`, nameToken);
}
