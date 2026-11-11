/*
 * The web globals that bun-types declares itself, as they are when lib.dom.d.ts is not loaded.
 *
 * This file is a module, like the files in web-globals/ of @types/node. So an interface here is
 * local to the file. It is not the global of its name, and it does not merge with lib.dom.d.ts.
 * At its end the file gives `Bun.__internal` one alias for each interface, which is `{}` when
 * lib.dom.d.ts is loaded. globals.d.ts declares the globals, each with its alias as a base:
 *
 *     interface AbortController extends Bun.__internal.LibOrFallbackAbortController {}
 *
 * So a project that loads lib.dom.d.ts gets the declaration of lib.dom and nothing beside it.
 *
 * In this file a name such as `Blob` means the interface of this file. `globalThis.Blob` is the
 * global, which also has what lib.dom.d.ts, @types/node and a project add to it. The type of a
 * member names the global.
 *
 * A member that only Bun has does not go here, because a project that loads lib.dom.d.ts does
 * not get these interfaces. It stays a member of the global interface in globals.d.ts, as
 * `Blob.json()` does.
 */
export {};

interface FormData {
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/FormData/append) */
  append(name: string, value: string | globalThis.Blob): void;
  append(name: string, value: string): void;
  append(name: string, blobValue: globalThis.Blob, filename?: string): void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/FormData/delete) */
  delete(name: string): void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/FormData/get) */
  get(name: string): Bun.FormDataEntryValue | null;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/FormData/getAll) */
  getAll(name: string): Bun.FormDataEntryValue[];
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/FormData/has) */
  has(name: string): boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/FormData/set) */
  set(name: string, value: string | globalThis.Blob): void;
  set(name: string, value: string): void;
  set(name: string, blobValue: globalThis.Blob, filename?: string): void;
  forEach(
    callbackfn: (value: Bun.FormDataEntryValue, key: string, parent: globalThis.FormData) => void,
    thisArg?: any,
  ): void;
}

interface FormDataIterable {
  /** Returns a list of keys in the list. */
  keys(): IterableIterator<string>;
  /** Returns a list of values in the list. */
  values(): IterableIterator<Bun.FormDataEntryValue>;
  /** Returns an array of key, value pairs for every entry in the list. */
  entries(): IterableIterator<[string, Bun.FormDataEntryValue]>;
  [Symbol.iterator](): IterableIterator<[string, Bun.FormDataEntryValue]>;
}

interface Crypto {
  readonly subtle: SubtleCrypto;

  /**
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/Crypto/randomUUID)
   */
  randomUUID(): `${string}-${string}-${string}-${string}-${string}`;
}

interface File {
  readonly lastModified: number;
  readonly name: string;
}

interface ErrorEvent {
  readonly colno: number;
  readonly error: any;
  readonly filename: string;
  readonly lineno: number;
  readonly message: string;
}

interface CloseEvent {
  /** Returns the WebSocket connection close code provided by the server. */
  readonly code: number;
  /** Returns the WebSocket connection close reason provided by the server. */
  readonly reason: string;
  /** Returns true if the connection closed cleanly; false otherwise. */
  readonly wasClean: boolean;
}

interface CustomEvent<T = any> {
  /** Any custom data the event was created with. Typically used for synthetic events. */
  readonly detail: T;
}

interface QueuingStrategy<T = any> {
  highWaterMark?: number;
  size?: QueuingStrategySize<T>;
}

interface QueuingStrategyInit {
  /**
   * The high water mark for the new queuing strategy.
   *
   * The value is not validated ahead of time. If it is negative, NaN, or not a
   * number, the stream constructor the strategy is passed to throws.
   */
  highWaterMark: number;
}

interface ByteLengthQueuingStrategy extends globalThis.QueuingStrategy<ArrayBufferView> {
  readonly highWaterMark: number;
  // lib.dom.d.ts has ArrayBufferView here too, not BufferSource
  readonly size: QueuingStrategySize<ArrayBufferView>;
}

interface ReadableStreamDefaultController {
  readonly desiredSize: number | null;
  close(): void;
  error(e?: any): void;
}

interface ReadableStreamDefaultReader {
  releaseLock(): void;
}

interface ReadableStreamGenericReader {
  readonly closed: Promise<void>;
  cancel(reason?: any): Promise<void>;
}

interface ReadableWritablePair<R = any, W = any> {
  readable: ReadableStream<R>;
  writable: WritableStream<W>;
}

interface WritableStreamDefaultController {
  error(e?: any): void;
}

interface WritableStreamDefaultWriter {
  readonly closed: Promise<void>;
  readonly desiredSize: number | null;
  readonly ready: Promise<void>;
  abort(reason?: any): Promise<void>;
  close(): Promise<void>;
  releaseLock(): void;
}

interface TransformStream<I = any, O = any> {
  readonly readable: ReadableStream<O>;
  readonly writable: WritableStream<I>;
}

interface TransformStreamDefaultController {
  readonly desiredSize: number | null;
  error(reason?: any): void;
  terminate(): void;
}

interface StreamPipeOptions {
  preventAbort?: boolean;
  preventCancel?: boolean;
  preventClose?: boolean;
  signal?: globalThis.AbortSignal;
}

interface CountQueuingStrategy extends globalThis.QueuingStrategy {
  readonly highWaterMark: number;
  readonly size: QueuingStrategySize;
}

interface Transformer<I = any, O = any> {
  flush?: Bun.TransformerFlushCallback<O>;
  readableType?: undefined;
  start?: Bun.TransformerStartCallback<O>;
  transform?: Bun.TransformerTransformCallback<I, O>;
  writableType?: undefined;
}

interface DOMException extends Error {
  readonly message: string;
  readonly name: string;
  readonly INDEX_SIZE_ERR: 1;
  readonly DOMSTRING_SIZE_ERR: 2;
  readonly HIERARCHY_REQUEST_ERR: 3;
  readonly WRONG_DOCUMENT_ERR: 4;
  readonly INVALID_CHARACTER_ERR: 5;
  readonly NO_DATA_ALLOWED_ERR: 6;
  readonly NO_MODIFICATION_ALLOWED_ERR: 7;
  readonly NOT_FOUND_ERR: 8;
  readonly NOT_SUPPORTED_ERR: 9;
  readonly INUSE_ATTRIBUTE_ERR: 10;
  readonly INVALID_STATE_ERR: 11;
  readonly SYNTAX_ERR: 12;
  readonly INVALID_MODIFICATION_ERR: 13;
  readonly NAMESPACE_ERR: 14;
  readonly INVALID_ACCESS_ERR: 15;
  readonly VALIDATION_ERR: 16;
  readonly TYPE_MISMATCH_ERR: 17;
  readonly SECURITY_ERR: 18;
  readonly NETWORK_ERR: 19;
  readonly ABORT_ERR: 20;
  readonly URL_MISMATCH_ERR: 21;
  readonly QUOTA_EXCEEDED_ERR: 22;
  readonly TIMEOUT_ERR: 23;
  readonly INVALID_NODE_TYPE_ERR: 24;
  readonly DATA_CLONE_ERR: 25;
  /** @deprecated */
  readonly code: number;
}

interface EventSourceInit {
  withCredentials?: boolean;
}

interface BlobPropertyBag {
  /** Set a default "type". */
  type?: string;
  /** Not implemented in Bun yet. */
  // endings?: "transparent" | "native";
}

interface Blob {
  /**
   * The size of this Blob in bytes
   */
  readonly size: number;

  /**
   * The MIME type of this Blob
   */
  readonly type: string;

  /**
   * Returns a promise that resolves to the contents of the blob as a string
   */
  text(): Promise<string>;

  /**
   * Returns a promise that resolves to the contents of the blob as an ArrayBuffer
   */
  arrayBuffer(): Promise<ArrayBuffer>;
}

interface AbortController {
  readonly signal: globalThis.AbortSignal;
  abort(reason?: any): void;
}

interface AbortSignal {
  readonly aborted: boolean;
  onabort: ((this: globalThis.AbortSignal, ev: Event) => any) | null;
  readonly reason: any;
  throwIfAborted(): void;
}

interface MessageChannel {
  /**
   * Returns the first MessagePort object.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/MessageChannel/port1)
   */
  readonly port1: MessagePort;
  /**
   * Returns the second MessagePort object.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/MessageChannel/port2)
   */
  readonly port2: MessagePort;
}

// Inside `declare module "bun"`, a name means the export of "bun" when "bun" has one of that name,
// as it has for `WebSocket` and `MessageEvent`. These names stay the interfaces of this file.
type LocalFormData = FormData;
type LocalFormDataIterable = FormDataIterable;
type LocalCrypto = Crypto;
type LocalFile = File;
type LocalErrorEvent = ErrorEvent;
type LocalCloseEvent = CloseEvent;
type LocalCustomEvent<T = any> = CustomEvent<T>;
type LocalQueuingStrategy<T = any> = QueuingStrategy<T>;
type LocalQueuingStrategyInit = QueuingStrategyInit;
type LocalByteLengthQueuingStrategy = ByteLengthQueuingStrategy;
type LocalReadableStreamDefaultController = ReadableStreamDefaultController;
type LocalReadableStreamDefaultReader = ReadableStreamDefaultReader;
type LocalReadableStreamGenericReader = ReadableStreamGenericReader;
type LocalReadableWritablePair<R = any, W = any> = ReadableWritablePair<R, W>;
type LocalWritableStreamDefaultController = WritableStreamDefaultController;
type LocalWritableStreamDefaultWriter = WritableStreamDefaultWriter;
type LocalTransformStream<I = any, O = any> = TransformStream<I, O>;
type LocalTransformStreamDefaultController = TransformStreamDefaultController;
type LocalStreamPipeOptions = StreamPipeOptions;
type LocalCountQueuingStrategy = CountQueuingStrategy;
type LocalTransformer<I = any, O = any> = Transformer<I, O>;
type LocalDOMException = DOMException;
type LocalEventSourceInit = EventSourceInit;
type LocalBlobPropertyBag = BlobPropertyBag;
type LocalBlob = Blob;
type LocalAbortController = AbortController;
type LocalAbortSignal = AbortSignal;
type LocalMessageChannel = MessageChannel;

declare module "bun" {
  namespace __internal {
    type LibOrFallbackFormData = LibOrFallback<LocalFormData>;
    type LibIterableOrFallbackFormData = LibIterableOrFallback<LocalFormDataIterable>;
    type LibOrFallbackCrypto = LibOrFallback<LocalCrypto>;
    type LibOrFallbackFile = LibOrFallback<LocalFile>;
    type LibOrFallbackErrorEvent = LibOrFallback<LocalErrorEvent>;
    type LibOrFallbackCloseEvent = LibOrFallback<LocalCloseEvent>;
    type LibOrFallbackCustomEvent<T = any> = LibOrFallback<LocalCustomEvent<T>>;
    type LibOrFallbackAddEventListenerOptions = LibOrFallback<Bun.AddEventListenerOptions>;
    type LibOrFallbackQueuingStrategy<T = any> = LibOrFallback<LocalQueuingStrategy<T>>;
    type LibOrFallbackQueuingStrategyInit = LibOrFallback<LocalQueuingStrategyInit>;
    type LibOrFallbackByteLengthQueuingStrategy = LibOrFallback<LocalByteLengthQueuingStrategy>;
    type LibOrFallbackReadableStreamDefaultController = LibOrFallback<LocalReadableStreamDefaultController>;
    type LibOrFallbackReadableStreamDefaultReader = LibOrFallback<LocalReadableStreamDefaultReader>;
    type LibOrFallbackReadableStreamGenericReader = LibOrFallback<LocalReadableStreamGenericReader>;
    type LibOrFallbackReadableWritablePair<R = any, W = any> = LibOrFallback<LocalReadableWritablePair<R, W>>;
    type LibOrFallbackWritableStreamDefaultController = LibOrFallback<LocalWritableStreamDefaultController>;
    type LibOrFallbackWritableStreamDefaultWriter = LibOrFallback<LocalWritableStreamDefaultWriter>;
    type LibOrFallbackTransformStream<I = any, O = any> = LibOrFallback<LocalTransformStream<I, O>>;
    type LibOrFallbackTransformStreamDefaultController = LibOrFallback<LocalTransformStreamDefaultController>;
    type LibOrFallbackStreamPipeOptions = LibOrFallback<LocalStreamPipeOptions>;
    type LibOrFallbackCountQueuingStrategy = LibOrFallback<LocalCountQueuingStrategy>;
    type LibOrFallbackTransformer<I = any, O = any> = LibOrFallback<LocalTransformer<I, O>>;
    type LibOrFallbackDOMException = LibOrFallback<LocalDOMException>;
    type LibOrFallbackEventSourceInit = LibOrFallback<LocalEventSourceInit>;
    type LibOrFallbackBlobPropertyBag = LibOrFallback<LocalBlobPropertyBag>;
    type LibOrFallbackBlob = LibOrFallback<LocalBlob>;
    type LibOrFallbackAbortController = LibOrFallback<LocalAbortController>;
    type LibOrFallbackAbortSignal = LibOrFallback<LocalAbortSignal>;
    type LibOrFallbackMessageChannel = LibOrFallback<LocalMessageChannel>;
  }
}
