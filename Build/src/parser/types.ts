export type Modifier =
  | { type: "flag"; value: string }
  | { type: "pair"; key: string; value: string }
  | { type: "atcode"; name: string; body: string }
  | { type: "event"; event: string; handler: string };

export type InterpolatedTextPart =
  | { type: "text"; value: string }
  | { type: "expr"; value: string };

export type InterpolatedText = {
  type: "interpolated";
  parts: InterpolatedTextPart[];
};

export type AtcodeDeclaration =
  | {
      type: "state";
      declarations: Array<{ name: string; value: string }>;
      line: number;
      col: number;
    }
  | {
      type: "effect";
      body: string;
      line: number;
      col: number;
    }
  | {
      type: "derived";
      declarations: Array<{ name: string; expr: string }>;
      line: number;
      col: number;
    };

export type RootNode = {
  type: "root";
  name: string;
  modifiers: Modifier[];
  declarations: AtcodeDeclaration[];
  children: ASTNode[];
};

export type ElementNode = {
  type: "element";
  name: string;
  modifiers: Modifier[];
  children: ASTNode[];
};

export type InlineNode = {
  type: "inline";
  name: string;
  modifiers: Modifier[];
  value: string | InterpolatedText;
};

export type ListNode = {
  type: "list";
  items: ASTNode[];
};

export type ASTNode = RootNode | ElementNode | InlineNode | ListNode;

import type { Token } from "./tokenizer";

export interface ParserCore {
  check: (type: string) => boolean;
  consume: () => Token;
  peek: () => Token | undefined;
  peekAheadIs: (type: string) => boolean;
  errorAt: (msg: string, token?: Token) => Error;
}

export interface ParserState extends ParserCore {
  expect: (type: string, errorMsg?: string) => Token;
  parseBlockBody: () => string;
  parseExpression: () => string;
  parseList: () => ListNode;
  parseNode: () => ASTNode;
}
