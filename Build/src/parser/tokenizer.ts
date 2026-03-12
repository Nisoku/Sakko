export type TokenType =
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

export type Token = {
  type: TokenType;
  value: string;
  line: number;
  col: number;
};

export function tokenize(input: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  let line = 1;
  let col = 1;

  while (i < input.length) {
    const ch = input[i];

    if (ch === "\n") {
      i++;
      line++;
      col = 1;
      continue;
    }
    if (ch === "\r") {
      i++;
      if (input[i] === "\n") i++;
      line++;
      col = 1;
      continue;
    }
    if (ch === " " || ch === "\t") {
      i++;
      col++;
      continue;
    }

    // Comments: skip to end of line
    if (ch === "/" && i + 1 < input.length && input[i + 1] === "/") {
      while (i < input.length && input[i] !== "\n" && input[i] !== "\r") {
        i++;
      }
      continue;
    }

    const SYMBOLS: Record<string, TokenType> = {
      "<": "LT",
      ">": "GT",
      "{": "LBRACE",
      "}": "RBRACE",
      "(": "LPAREN",
      ")": "RPAREN",
      "[": "LBRACKET",
      "]": "RBRACKET",
      ":": "COLON",
      ";": "SEMI",
      ",": "COMMA",
    };

    if (SYMBOLS[ch]) {
      tokens.push({ type: SYMBOLS[ch], value: ch, line, col });
      i++;
      col++;
      continue;
    }

    if (ch === '"') {
      const startCol = col;
      i++;
      col++;
      let str = "";
      while (i < input.length && input[i] !== '"') {
        if (input[i] === "\n") {
          line++;
          col = 1;
        } else {
          col++;
        }
        str += input[i];
        i++;
      }
      if (i >= input.length) {
        throw new Error(`Unterminated string at line ${line}, col ${startCol}`);
      }
      i++;
      col++;
      tokens.push({ type: "STRING", value: str, line, col: startCol });
      continue;
    }

    if (/[a-zA-Z0-9_\-]/.test(ch)) {
      const startCol = col;
      let ident = "";
      while (i < input.length && /[a-zA-Z0-9_\-]/.test(input[i])) {
        ident += input[i];
        i++;
        col++;
      }
      tokens.push({ type: "IDENT", value: ident, line, col: startCol });
      continue;
    }

    throw new Error(`Unexpected character: ${ch} at line ${line}, col ${col}`);
  }

  return tokens;
}
