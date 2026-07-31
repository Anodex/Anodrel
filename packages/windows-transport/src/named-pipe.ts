import { createConnection, type Socket } from "node:net";
import { TextDecoder } from "node:util";

import type {
  CancellationEnvelope,
  PlatformOperation,
  RequestEnvelope,
  ResponseEnvelope,
} from "@anodrel/protocol";
import type { PlatformTransport } from "@anodrel/sdk";

import { BootstrapInvitation } from "./bootstrap.js";
import { isStrictObject, parseStrictJson, type StrictJson } from "./strict-json.js";

const FRAME_MAGIC = "ANDR";
const FRAME_HEADER_BYTES = 12;
const MAX_PAYLOAD_BYTES = 65_536;
const MAX_BUFFER_BYTES = (FRAME_HEADER_BYTES + MAX_PAYLOAD_BYTES) * 4;
const MAX_QUEUED_FRAMES = 4;

export class WindowsTransportError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "WindowsTransportError";
  }
}

/** A development-only client transport for Anodrel's direct Windows named pipe. */
export class WindowsNamedPipeTransport implements PlatformTransport {
  readonly #frames: FrameReader;
  #sequence: Promise<void> = Promise.resolve();

  private constructor(private readonly socket: Socket) {
    this.#frames = new FrameReader(socket);
  }

  static async connect(invitation: BootstrapInvitation): Promise<WindowsNamedPipeTransport> {
    const socket = await connectPipe(invitation.pipeName);
    const transport = new WindowsNamedPipeTransport(socket);
    try {
      const authenticated = await transport.roundTrip(invitation.takeAuthentication());
      if (
        !isStrictObject(authenticated) ||
        Object.keys(authenticated).length !== 1 ||
        authenticated.kind !== "session.authenticated"
      ) {
        throw new WindowsTransportError("Host rejected the bootstrap authentication.");
      }
      return transport;
    } catch (error) {
      socket.destroy();
      throw error;
    }
  }

  send<TOperation extends PlatformOperation>(
    request: RequestEnvelope<TOperation>,
  ): Promise<ResponseEnvelope<TOperation>> {
    return this.enqueue(async () => {
      const response = await this.roundTrip(request);
      if (!isResponseEnvelope(response)) {
        throw new WindowsTransportError("Host sent an invalid protocol response.");
      }
      return response as unknown as ResponseEnvelope<TOperation>;
    });
  }

  async cancel(cancellation: CancellationEnvelope): Promise<void> {
    await this.enqueue(async () => {
      await this.roundTrip(cancellation);
    });
  }

  close(): Promise<void> {
    if (this.socket.destroyed) {
      return Promise.resolve();
    }
    return new Promise<void>((resolve) => {
      this.socket.once("close", resolve);
      this.socket.end();
    });
  }

  private enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const result = this.#sequence.then(operation, operation);
    this.#sequence = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  private async roundTrip(message: unknown): Promise<StrictJson> {
    const response = this.#frames.next();
    await writeFrame(this.socket, message);
    return response;
  }
}

class FrameReader {
  #buffer = Buffer.alloc(0);
  #messages: StrictJson[] = [];
  #waiting:
    | { readonly resolve: (value: StrictJson) => void; readonly reject: (error: Error) => void }
    | undefined;
  #failure: Error | undefined;

  constructor(socket: Socket) {
    socket.on("data", (chunk: Buffer) => this.accept(chunk));
    socket.on("error", (error: Error) => this.fail(error));
    socket.on("end", () => this.fail(new WindowsTransportError("Host pipe closed.")));
    socket.on("close", () => this.fail(new WindowsTransportError("Host pipe closed.")));
  }

  next(): Promise<StrictJson> {
    if (this.#failure !== undefined) {
      return Promise.reject(this.#failure);
    }
    const message = this.#messages.shift();
    if (message !== undefined) {
      return Promise.resolve(message);
    }
    if (this.#waiting !== undefined) {
      return Promise.reject(new WindowsTransportError("Concurrent pipe reads are not allowed."));
    }
    return new Promise<StrictJson>((resolve, reject) => {
      this.#waiting = { resolve, reject };
    });
  }

  private accept(chunk: Buffer): void {
    if (this.#failure !== undefined) {
      return;
    }
    this.#buffer = Buffer.concat([this.#buffer, chunk]);
    if (this.#buffer.byteLength > MAX_BUFFER_BYTES) {
      this.fail(new WindowsTransportError("Host pipe exceeded the receive bound."));
      return;
    }
    try {
      while (this.#buffer.byteLength >= FRAME_HEADER_BYTES) {
        if (this.#buffer.subarray(0, 4).toString("ascii") !== FRAME_MAGIC) {
          throw new WindowsTransportError("Host frame magic is invalid.");
        }
        if (this.#buffer.readUInt16LE(4) !== 1 || this.#buffer.readUInt16LE(6) !== 0) {
          throw new WindowsTransportError("Host frame version is unsupported.");
        }
        const payloadLength = this.#buffer.readUInt32LE(8);
        if (payloadLength > MAX_PAYLOAD_BYTES) {
          throw new WindowsTransportError("Host frame exceeds the payload limit.");
        }
        const frameLength = FRAME_HEADER_BYTES + payloadLength;
        if (this.#buffer.byteLength < frameLength) {
          return;
        }
        const payload = new TextDecoder("utf-8", { fatal: true }).decode(
          this.#buffer.subarray(FRAME_HEADER_BYTES, frameLength),
        );
        this.#buffer = this.#buffer.subarray(frameLength);
        this.deliver(parseStrictJson(payload));
      }
    } catch (error) {
      this.fail(
        error instanceof Error
          ? error
          : new WindowsTransportError("Host frame could not be decoded."),
      );
    }
  }

  private deliver(message: StrictJson): void {
    const waiting = this.#waiting;
    if (waiting !== undefined) {
      this.#waiting = undefined;
      waiting.resolve(message);
      return;
    }
    if (this.#messages.length >= MAX_QUEUED_FRAMES) {
      this.fail(new WindowsTransportError("Host exceeded the queued frame bound."));
      return;
    }
    this.#messages.push(message);
  }

  private fail(error: Error): void {
    if (this.#failure !== undefined) {
      return;
    }
    this.#failure = error;
    this.#buffer.fill(0);
    this.#buffer = Buffer.alloc(0);
    const waiting = this.#waiting;
    if (waiting !== undefined) {
      this.#waiting = undefined;
      waiting.reject(error);
    }
  }
}

function connectPipe(pipeName: string): Promise<Socket> {
  return new Promise<Socket>((resolve, reject) => {
    const socket = createConnection(pipeName);
    const rejectOnce = (error: Error): void => {
      socket.destroy();
      reject(error);
    };
    socket.once("error", rejectOnce);
    socket.once("connect", () => {
      socket.off("error", rejectOnce);
      resolve(socket);
    });
  });
}

function writeFrame(socket: Socket, message: unknown): Promise<void> {
  const payload = Buffer.from(JSON.stringify(message), "utf8");
  if (payload.byteLength > MAX_PAYLOAD_BYTES) {
    payload.fill(0);
    return Promise.reject(new WindowsTransportError("Client frame exceeds the payload limit."));
  }
  const frame = Buffer.allocUnsafe(FRAME_HEADER_BYTES + payload.byteLength);
  frame.write(FRAME_MAGIC, 0, "ascii");
  frame.writeUInt16LE(1, 4);
  frame.writeUInt16LE(0, 6);
  frame.writeUInt32LE(payload.byteLength, 8);
  payload.copy(frame, FRAME_HEADER_BYTES);
  payload.fill(0);
  return new Promise<void>((resolve, reject) => {
    socket.write(frame, (error) => {
      frame.fill(0);
      if (error == null) {
        resolve();
      } else {
        reject(error);
      }
    });
  });
}

function isResponseEnvelope(value: StrictJson): boolean {
  return (
    isStrictObject(value) &&
    value.kind === "response" &&
    typeof value.requestId === "string" &&
    (value.status === "success" || value.status === "failure")
  );
}
