//! Private pure validation primitives shared by protocol payload guards.

import type { ProtocolVersion } from "../index.js";

export function isValidatedHttpsUrl(value: string): boolean {
  if (value.length === 0 || !/^[\x21-\x7e]+$/.test(value) || value.includes("\\")) {
    return false;
  }
  const match = /^https:\/\/([^/?#]+)(?:[/?#].*)?$/.exec(value);
  if (match === null) {
    return false;
  }
  const authority = match[1];
  if (authority === undefined) {
    return false;
  }
  if (authority.includes("@")) {
    return false;
  }
  const separator = authority.lastIndexOf(":");
  const host = separator === -1 ? authority : authority.slice(0, separator);
  const port = separator === -1 ? undefined : authority.slice(separator + 1);
  if (
    !isDnsStyleHost(host) ||
    (port !== undefined && (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65_535))
  ) {
    return false;
  }
  return true;
}

export function isValidatedHttpsTextFetchUrl(value: string): boolean {
  if (
    value.length === 0 ||
    !/^[A-Za-z0-9\-._~:/?@!$&'()*+,;=%]+$/.test(value) ||
    value.includes("\\") ||
    value.includes("#") ||
    !value.startsWith("https://") ||
    /%(?![0-9A-Fa-f]{2})/.test(value)
  ) {
    return false;
  }
  const match = /^https:\/\/([^/?]+)(?:[/?].*)?$/.exec(value);
  if (match === null) {
    return false;
  }
  const authority = match[1];
  if (authority === undefined || authority.includes("@")) {
    return false;
  }
  const separator = authority.lastIndexOf(":");
  const host = separator === -1 ? authority : authority.slice(0, separator);
  const port = separator === -1 ? undefined : authority.slice(separator + 1);
  return (
    isDnsStyleHost(host) &&
    !isIpv4Literal(host) &&
    (port === undefined || (/^\d+$/.test(port) && Number(port) >= 1 && Number(port) <= 65_535))
  );
}

function isIpv4Literal(value: string): boolean {
  const labels = value.split(".");
  return (
    labels.length === 4 &&
    labels.every((label) => /^\d+$/.test(label) && Number(label) >= 0 && Number(label) <= 255)
  );
}

function isDnsStyleHost(value: string): boolean {
  return (
    value.length > 0 &&
    value.length <= 253 &&
    !value.endsWith(".") &&
    value.split(".").every(
      (label) =>
        label.length > 0 &&
        label.length <= 63 &&
        /^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$/.test(label),
    )
  );
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isProtocolVersion(value: unknown): value is ProtocolVersion {
  if (!isRecord(value)) {
    return false;
  }

  const { major, minor } = value;
  return (
    typeof major === "number" &&
    typeof minor === "number" &&
    Number.isSafeInteger(major) &&
    Number.isSafeInteger(minor) &&
    major >= 0 &&
    minor >= 0
  );
}

export function isLimitedIdentifier(value: string, maximumBytes: number): boolean {
  return value.length > 0 && new TextEncoder().encode(value).byteLength <= maximumBytes;
}
