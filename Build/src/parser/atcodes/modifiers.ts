import type { Modifier } from "../types";
import type { Token } from "../tokenizer";

export interface ParserCore {
  check: (type: string) => boolean;
  consume: () => Token;
  errorAt: (msg: string, token?: Token) => Error;
}

export interface ParserState extends ParserCore {
  expect: (type: string, errorMsg?: string) => Token;
  parseBlockBody: () => string;
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

    let handler = "";

    if (parser.check("LBRACE")) {
      parser.consume();
      handler = parser.parseBlockBody();
      parser.expect("RBRACE");
    } else {
      throw parser.errorAt(
        "Event handlers must use block syntax: @on:click { ... }",
        eventToken,
      );
    }

    return {
      type: "event",
      event,
      handler,
    };
  }

  // @on:eventName (shorthand)
  if (EVENT_NAMES.has(name)) {
    const event = name;
    let handler = "";

    if (parser.check("LBRACE")) {
      parser.consume();
      handler = parser.parseBlockBody();
      parser.expect("RBRACE");
    } else if (parser.check("IDENT")) {
      handler = parser.consume().value;
    } else {
      handler = "() => {}";
    }

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

  throw parser.errorAt(`Unknown modifier: @${name}`, nameToken);
}
