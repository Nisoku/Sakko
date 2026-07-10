let logger: {
  info: (msg: string, opts?: Record<string, unknown>) => void;
  warn: (msg: string, opts?: Record<string, unknown>) => void;
  error: (msg: string, opts?: Record<string, unknown>) => void;
} | null = null;

function getLogger(scope: string) {
  if (!logger) {
    try {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const satori = require("@nisoku/satori");
      const s = satori.createSatori({ logLevel: "error", enableConsole: true });
      logger = s.createLogger(scope);
    } catch {
      logger = {
        info: (msg: string, opts?: unknown) =>
          console.log(`[${scope}] ${msg}`, opts),
        warn: (msg: string, opts?: unknown) =>
          console.warn(`[${scope}] ${msg}`, opts),
        error: (msg: string, opts?: unknown) =>
          console.error(`[${scope}] ${msg}`, opts),
      };
    }
  }
  return logger;
}

export interface TokenizerErrorOptions {
  position: number;
  line?: number;
  column?: number;
  suggestion?: string;
}

export interface ParserErrorOptions {
  line?: number;
  column?: number;
  suggestion?: string;
  cause?: string;
}

export function tokenizerError(
  message: string,
  options: TokenizerErrorOptions,
): void {
  const log = getLogger("sakko")!;
  log.error(message, {
    state: {
      position: options.position,
      line: options.line,
      column: options.column,
    },
    suggest: options.suggestion,
    tags: ["tokenizer", "error"],
  });
}

export function parserError(
  message: string,
  options: ParserErrorOptions,
): void {
  const log = getLogger("sakko")!;
  log.error(message, {
    state: { line: options.line, column: options.column },
    suggest: options.suggestion,
    cause: options.cause,
    tags: ["parser", "error"],
  });
}

export function unknownComponentError(
  component: string,
  suggestion?: string,
): void {
  const log = getLogger("sakko")!;
  log.warn(`Unknown component "${component}", using saz-${component}`, {
    suggest: suggestion,
    tags: ["transformer", "warning"],
  });
}

export function transformError(
  message: string,
  options: { suggestion?: string; cause?: string },
): void {
  const log = getLogger("sakko")!;
  log.error(message, {
    suggest: options.suggestion,
    cause: options.cause,
    tags: ["transformer", "error"],
  });
}
