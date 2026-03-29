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

export type ASTNode = ElementNode | InlineNode | ListNode;
