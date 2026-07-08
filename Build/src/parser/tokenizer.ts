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
  | "BACKTICK_STRING"
  | "AT"
  | "EQUALS"
  | "INTERP_START"
  | "INTERP_END"
  | "EXPR"
  | "DOT"
  | "PLUS"
  | "MINUS"
  | "STAR"
  | "PIPE"
  | "AMPERSAND"
  | "BANG"
  | "QUESTION"
  | "PERCENT";

export type Token = {
  type: TokenType;
  value: string;
  line: number;
  col: number;
};

/** Map a single escape character to its runtime value. */
function handleEscapeSequence(esc: string): string {
  switch (esc) {
    case "n":
      return "\n";
    case "t":
      return "\t";
    case "r":
      return "\r";
    case '"':
      return '"';
    case "'":
      return "'";
    case "`":
      return "`";
    case "\\":
      return "\\";
    case "$":
      return "$";
    default:
      return "\\" + esc; // preserve unknown escapes as-is
  }
}

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
      "|": "PIPE",
      "&": "AMPERSAND",
      "!": "BANG",
      "?": "QUESTION",
      "%": "PERCENT",
    };

    if (SYMBOLS[ch]) {
      tokens.push({ type: SYMBOLS[ch], value: ch, line, col });
      i++;
      col++;
      continue;
    }

    if (ch === '"' || ch === "`") {
      const quote = ch;
      const startCol = col;
      i++;
      col++;

      // Scan only the substring up to the next unescaped closing quote
      // so we don't accidentally detect braces outside this literal.
      let scanEnd = i;
      while (scanEnd < input.length && input[scanEnd] !== quote) {
        if (input[scanEnd] === "\\" && scanEnd + 1 < input.length) {
          scanEnd += 2; // skip the escaped character
        } else {
          scanEnd++;
        }
      }
      const literalContent = input.slice(i, scanEnd);
      const hasInterpolation =
        quote === '"' && /\{[\s\S]*?\}/.test(literalContent);

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
      while (i < input.length && input[i] !== quote) {
        if (input[i] === "\\" && i + 1 < input.length) {
          i++;
          col++;
          str += handleEscapeSequence(input[i]);
          i++;
          col++;
          continue;
        }
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
          suggestion: `Add a closing ${quote}`,
        });
        throw new Error(`Unterminated string at line ${line}, col ${startCol}`);
      }
      i++;
      col++;
      const tokenType = quote === "`" ? "BACKTICK_STRING" : "STRING";
      tokens.push({ type: tokenType, value: str, line, col: startCol });
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

      if (braceDepth > 0) {
        tokenizerError("Unterminated interpolation expression", {
          position: i,
          line: currentLine,
          column: exprStartCol,
          suggestion: "Add a closing brace '}'",
        });
        throw new Error(
          `Unterminated interpolation expression at line ${currentLine}, col ${exprStartCol}`,
        );
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

    // Handle escape sequences inside interpolated strings
    if (input[i] === "\\" && i + 1 < input.length) {
      i++;
      currentCol++;
      textBuffer += handleEscapeSequence(input[i]);
      currentCol++;
      i++;
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

  if (i >= input.length) {
    tokenizerError("Unterminated string", {
      position: i,
      line: currentLine,
      column: originalStartCol,
      suggestion: 'Add a closing quote "',
    });
    throw new Error(
      `Unterminated string at line ${currentLine}, col ${originalStartCol}`,
    );
  }

  if (textBuffer.length > 0 || tokens.length === 0) {
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
