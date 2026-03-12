type TokenType =
  | "LT"
  | "GT"
  | "LBRACE"
  | "RBRACE"
  | "LPAREN"
  | "RPAREN"
  | "LBRACKET"
  | "RBRACKET"
  | "COLON"
  | "SEMI"
  | "COMMA"
  | "IDENT"
  | "STRING";

type Token = {
  type: TokenType;
  value: string;
  line: number;
  col: number;
};