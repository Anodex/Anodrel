/** First-party canonical unpadded base64url helpers for bounded protocol data. */

const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/** Encodes bytes as unpadded canonical base64url without a runtime codec dependency. */
export function encodeCanonicalBase64Url(bytes: Uint8Array): string {
  const characters: string[] = [];
  let index = 0;
  while (index + 2 < bytes.length) {
    const first = bytes[index] ?? 0;
    const second = bytes[index + 1] ?? 0;
    const third = bytes[index + 2] ?? 0;
    characters.push(
      ALPHABET[first >>> 2] ?? "",
      ALPHABET[((first & 0x03) << 4) | (second >>> 4)] ?? "",
      ALPHABET[((second & 0x0f) << 2) | (third >>> 6)] ?? "",
      ALPHABET[third & 0x3f] ?? "",
    );
    index += 3;
  }
  if (index < bytes.length) {
    const first = bytes[index] ?? 0;
    characters.push(
      ALPHABET[first >>> 2] ?? "",
      ALPHABET[(first & 0x03) << 4] ?? "",
    );
    if (index + 1 < bytes.length) {
      const second = bytes[index + 1] ?? 0;
      characters.push(ALPHABET[(second & 0x0f) << 2] ?? "");
    }
  }
  return characters.join("");
}

/**
 * Returns the decoded length of one canonical unpadded base64url value.
 *
 * `undefined` means its alphabet, length, padding, or unused trailing bits
 * are invalid. It performs no decoding or allocation.
 */
export function canonicalBase64UrlDecodedLength(value: string): number | undefined {
  const remainder = value.length % 4;
  if (remainder === 1) {
    return undefined;
  }
  let last = 0;
  for (let index = 0; index < value.length; index += 1) {
    const decoded = base64UrlValue(value.charCodeAt(index));
    if (decoded === undefined) {
      return undefined;
    }
    last = decoded;
  }
  if ((remainder === 2 && (last & 0x0f) !== 0) || (remainder === 3 && (last & 0x03) !== 0)) {
    return undefined;
  }
  return Math.floor(value.length / 4) * 3 + (remainder === 2 ? 1 : remainder === 3 ? 2 : 0);
}

function base64UrlValue(code: number): number | undefined {
  if (code >= 0x41 && code <= 0x5a) return code - 0x41;
  if (code >= 0x61 && code <= 0x7a) return code - 0x61 + 26;
  if (code >= 0x30 && code <= 0x39) return code - 0x30 + 52;
  if (code === 0x2d) return 62;
  if (code === 0x5f) return 63;
  return undefined;
}
