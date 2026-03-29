import { tokenizerError } from "../errors";

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
  | "STRING"
  | "AT"
  | "EQUALS"
  | "INTERP_START"
  | "INTERP_END"
  | "EXPR"
  | "DOT"
  | "PLUS"
  | "MINUS"
  | "STAR";

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

    // Comments: skip to end of line (only if there's a newline or no < at all)
    if (ch === "/" && i + 1 < input.length && input[i + 1] === "/") {
      // Check if there's a newline in this comment (before any <)
      const commentContent = input.slice(i + 2);
      const nextNewline = commentContent.indexOf("\n");
      const nextLT = commentContent.indexOf("<");
      // Strip if: newline exists before <, or there's no < at all in the comment
      const hasNewlineBeforeLT =
        nextNewline !== -1 && (nextLT === -1 || nextNewline < nextLT);
      if (hasNewlineBeforeLT || nextLT === -1) {
        while (i < input.length && input[i] !== "\n" && input[i] !== "\r") {
          i++;
        }
        continue;
      }
      // < comes before newline, don't skip comment
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
      "@": "AT",
      "=": "EQUALS",
      ".": "DOT",
      "+": "PLUS",
      "-": "MINUS",
      "*": "STAR",
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

      const remaining = input.slice(i);
      const hasInterpolation = /\{[a-zA-Z_$]/.test(remaining);

      if (hasInterpolation) {
        const result = tokenizeStringWithInterpolation(
          input,
          i,
          line,
          col,
          startCol,
        );
        tokens.push(...result.tokens);
        i = result.endIndex + 1;
        line = result.endLine;
        col = result.endCol + 1;
        continue;
      }

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
        tokenizerError("Unterminated string", {
          position: i,
          line,
          column: startCol,
          suggestion: 'Add a closing quote "',
        });
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

    tokenizerError(`Unexpected character: ${ch}`, {
      position: i,
      line,
      column: col,
      suggestion: `Remove or escape this character`,
    });
    throw new Error(`Unexpected character: ${ch} at line ${line}, col ${col}`);
  }

  return tokens;
}

function tokenizeStringWithInterpolation(
  input: string,
  startIndex: number,
  line: number,
  col: number,
  originalStartCol: number,
): { tokens: Token[]; endIndex: number; endLine: number; endCol: number } {
  const tokens: Token[] = [];
  let i = startIndex;
  let currentLine = line;
  let currentCol = col;
  let textBuffer = "";
  let textStartCol = currentCol;

  while (i < input.length && input[i] !== '"') {
    if (input[i] === "{") {
      if (textBuffer) {
        tokens.push({
          type: "STRING",
          value: textBuffer,
          line: currentLine,
          col: textStartCol,
        });
        textBuffer = "";
      }

      tokens.push({
        type: "INTERP_START",
        value: "{",
        line: currentLine,
        col: currentCol,
      });
      i++;
      currentCol++;

      let expr = "";
      let braceDepth = 1;
      const exprStartCol = currentCol;

      while (i < input.length && braceDepth > 0) {
        if (input[i] === "{") braceDepth++;
        if (input[i] === "}") braceDepth--;

        if (braceDepth > 0) {
          expr += input[i];
        }

        if (input[i] === "\n") {
          currentLine++;
          currentCol = 1;
        } else {
          currentCol++;
        }
        i++;
      }

      tokens.push({
        type: "EXPR",
        value: expr.trim(),
        line: currentLine,
        col: exprStartCol,
      });
      tokens.push({
        type: "INTERP_END",
        value: "}",
        line: currentLine,
        col: currentCol - 1,
      });

      textStartCol = currentCol;
      continue;
    }

    textBuffer += input[i];
    if (input[i] === "\n") {
      currentLine++;
      currentCol = 1;
    } else {
      currentCol++;
    }
    i++;
  }

  if (textBuffer) {
    tokens.push({
      type: "STRING",
      value: textBuffer,
      line: currentLine,
      col: textStartCol,
    });
  }

  return {
    tokens,
    endIndex: i,
    endLine: currentLine,
    endCol: currentCol,
  };
}
