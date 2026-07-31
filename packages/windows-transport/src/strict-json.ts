export type StrictJson = null | boolean | number | string | StrictJsonArray | StrictJsonObject;

export interface StrictJsonArray extends ReadonlyArray<StrictJson> {}

export interface StrictJsonObject {
  readonly [key: string]: StrictJson;
}

const MAX_NESTING = 64;

export class StrictJsonError extends Error {
  constructor(message: string, readonly offset: number) {
    super(`${message} at character ${offset}.`);
    this.name = "StrictJsonError";
  }
}

export function parseStrictJson(input: string): StrictJson {
  return new StrictJsonParser(input).parse();
}

export function isStrictObject(value: unknown): value is StrictJsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

class StrictJsonParser {
  private offset = 0;

  constructor(private readonly input: string) {}

  parse(): StrictJson {
    this.skipWhitespace();
    const value = this.parseValue(0);
    this.skipWhitespace();
    if (this.offset !== this.input.length) {
      this.fail("Unexpected trailing data");
    }
    return value;
  }

  private parseValue(depth: number): StrictJson {
    if (depth > MAX_NESTING) {
      this.fail("JSON nesting exceeds the limit");
    }
    switch (this.peek()) {
      case "n":
        return this.parseLiteral("null", null);
      case "t":
        return this.parseLiteral("true", true);
      case "f":
        return this.parseLiteral("false", false);
      case "\"":
        return this.parseString();
      case "[":
        return this.parseArray(depth + 1);
      case "{":
        return this.parseObject(depth + 1);
      default:
        if (this.peek() === "-" || isDigit(this.peek())) {
          return this.parseNumber();
        }
        this.fail("Expected a JSON value");
    }
  }

  private parseLiteral<T extends null | boolean>(literal: string, value: T): T {
    if (this.input.slice(this.offset, this.offset + literal.length) !== literal) {
      this.fail("Invalid JSON literal");
    }
    this.offset += literal.length;
    return value;
  }

  private parseArray(depth: number): readonly StrictJson[] {
    this.consume("[");
    this.skipWhitespace();
    const values: StrictJson[] = [];
    if (this.tryConsume("]")) {
      return values;
    }
    while (true) {
      this.skipWhitespace();
      values.push(this.parseValue(depth));
      this.skipWhitespace();
      if (this.tryConsume("]")) {
        return values;
      }
      this.consume(",");
    }
  }

  private parseObject(depth: number): StrictJsonObject {
    this.consume("{");
    this.skipWhitespace();
    const fields: Record<string, StrictJson> = Object.create(null) as Record<string, StrictJson>;
    if (this.tryConsume("}")) {
      return fields;
    }
    while (true) {
      this.skipWhitespace();
      const key = this.parseString();
      this.skipWhitespace();
      this.consume(":");
      this.skipWhitespace();
      if (Object.hasOwn(fields, key)) {
        this.fail("Duplicate JSON object field");
      }
      fields[key] = this.parseValue(depth);
      this.skipWhitespace();
      if (this.tryConsume("}")) {
        return fields;
      }
      this.consume(",");
    }
  }

  private parseString(): string {
    this.consume("\"");
    let value = "";
    while (true) {
      const character = this.peek();
      if (character === undefined) {
        this.fail("Unterminated JSON string");
      }
      if (character === "\"") {
        this.offset += 1;
        return value;
      }
      if (character === "\\") {
        this.offset += 1;
        value += this.parseEscape();
        continue;
      }
      if (character.charCodeAt(0) <= 0x1f) {
        this.fail("Control character in JSON string");
      }
      value += character;
      this.offset += 1;
    }
  }

  private parseEscape(): string {
    const escape = this.next();
    switch (escape) {
      case "\"":
      case "\\":
      case "/":
        return escape;
      case "b":
        return "\b";
      case "f":
        return "\f";
      case "n":
        return "\n";
      case "r":
        return "\r";
      case "t":
        return "\t";
      case "u":
        return this.parseUnicodeEscape();
      default:
        this.fail("Invalid JSON escape");
    }
  }

  private parseUnicodeEscape(): string {
    const first = this.parseHexQuad();
    if (first >= 0xd800 && first <= 0xdbff) {
      if (!this.tryConsume("\\") || !this.tryConsume("u")) {
        this.fail("High surrogate without a low surrogate");
      }
      const second = this.parseHexQuad();
      if (second < 0xdc00 || second > 0xdfff) {
        this.fail("Invalid low surrogate");
      }
      return String.fromCodePoint(0x10000 + ((first - 0xd800) << 10) + second - 0xdc00);
    }
    if (first >= 0xdc00 && first <= 0xdfff) {
      this.fail("Low surrogate without a high surrogate");
    }
    return String.fromCodePoint(first);
  }

  private parseHexQuad(): number {
    const value = this.input.slice(this.offset, this.offset + 4);
    if (!/^[0-9a-fA-F]{4}$/u.test(value)) {
      this.fail("Invalid Unicode escape");
    }
    this.offset += 4;
    return Number.parseInt(value, 16);
  }

  private parseNumber(): number {
    const match = /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/u.exec(
      this.input.slice(this.offset),
    );
    if (match === null) {
      this.fail("Invalid JSON number");
    }
    this.offset += match[0].length;
    const value = Number(match[0]);
    if (!Number.isFinite(value)) {
      this.fail("JSON number is not finite");
    }
    return value;
  }

  private skipWhitespace(): void {
    while (this.peek() !== undefined && /[ \t\r\n]/u.test(this.peek() as string)) {
      this.offset += 1;
    }
  }

  private consume(expected: string): void {
    if (!this.tryConsume(expected)) {
      this.fail("Unexpected JSON token");
    }
  }

  private tryConsume(expected: string): boolean {
    if (this.input.startsWith(expected, this.offset)) {
      this.offset += expected.length;
      return true;
    }
    return false;
  }

  private next(): string | undefined {
    const value = this.peek();
    if (value !== undefined) {
      this.offset += 1;
    }
    return value;
  }

  private peek(): string | undefined {
    return this.input[this.offset];
  }

  private fail(message: string): never {
    throw new StrictJsonError(message, this.offset);
  }
}

function isDigit(value: string | undefined): boolean {
  return value !== undefined && value >= "0" && value <= "9";
}
