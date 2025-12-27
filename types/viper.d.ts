// Viper Runtime Type Definitions

// ============================================================================
// URL API
// ============================================================================

interface URLSearchParams {
  append(name: string, value: string): void;
  delete(name: string): void;
  get(name: string): string | null;
  getAll(name: string): string[];
  has(name: string): boolean;
  set(name: string, value: string): void;
  sort(): void;
  toString(): string;
  forEach(
    callback: (value: string, key: string, parent: URLSearchParams) => void,
  ): void;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;
  entries(): IterableIterator<[string, string]>;
  [Symbol.iterator](): IterableIterator<[string, string]>;
}

interface URLSearchParamsConstructor {
  new (
    init?: string | Record<string, string> | string[][] | URLSearchParams,
  ): URLSearchParams;
  prototype: URLSearchParams;
}

interface URL {
  hash: string;
  host: string;
  hostname: string;
  href: string;
  readonly origin: string;
  password: string;
  pathname: string;
  port: string;
  protocol: string;
  search: string;
  readonly searchParams: URLSearchParams;
  username: string;
  toString(): string;
  toJSON(): string;
}

interface URLConstructor {
  new (url: string, base?: string | URL): URL;
  prototype: URL;
}

// ============================================================================
// Fetch API
// ============================================================================

interface RequestInit {
  method?: string;
  headers?: HeadersInit;
  body?: BodyInit | null;
  mode?: RequestMode;
  credentials?: RequestCredentials;
  cache?: RequestCache;
  redirect?: RequestRedirect;
  referrer?: string;
  referrerPolicy?: ReferrerPolicy;
  integrity?: string;
  keepalive?: boolean;
  signal?: AbortSignal | null;
}

type HeadersInit = Headers | Record<string, string> | [string, string][];
type BodyInit =
  | string
  | ArrayBuffer
  | Uint8Array
  | URLSearchParams
  | FormData
  | Blob;
type RequestMode = "cors" | "navigate" | "no-cors" | "same-origin";
type RequestCredentials = "include" | "omit" | "same-origin";
type RequestCache =
  | "default"
  | "force-cache"
  | "no-cache"
  | "no-store"
  | "only-if-cached"
  | "reload";
type RequestRedirect = "error" | "follow" | "manual";
type ReferrerPolicy =
  | ""
  | "no-referrer"
  | "no-referrer-when-downgrade"
  | "origin"
  | "origin-when-cross-origin"
  | "same-origin"
  | "strict-origin"
  | "strict-origin-when-cross-origin"
  | "unsafe-url";

interface Headers {
  append(name: string, value: string): void;
  delete(name: string): void;
  get(name: string): string | null;
  has(name: string): boolean;
  set(name: string, value: string): void;
  forEach(
    callback: (value: string, key: string, parent: Headers) => void,
  ): void;
  keys(): IterableIterator<string>;
  values(): IterableIterator<string>;
  entries(): IterableIterator<[string, string]>;
  [Symbol.iterator](): IterableIterator<[string, string]>;
}

interface HeadersConstructor {
  new (init?: HeadersInit): Headers;
  prototype: Headers;
}

interface Request {
  readonly method: string;
  readonly url: string;
  readonly headers: Headers;
  readonly body: ReadableStream<Uint8Array> | null;
  readonly bodyUsed: boolean;
  readonly cache: RequestCache;
  readonly credentials: RequestCredentials;
  readonly destination: string;
  readonly integrity: string;
  readonly keepalive: boolean;
  readonly mode: RequestMode;
  readonly redirect: RequestRedirect;
  readonly referrer: string;
  readonly referrerPolicy: ReferrerPolicy;
  readonly signal: AbortSignal;
  clone(): Request;
  arrayBuffer(): Promise<ArrayBuffer>;
  blob(): Promise<Blob>;
  formData(): Promise<FormData>;
  json(): Promise<any>;
  text(): Promise<string>;
}

interface RequestConstructor {
  new (input: string | URL | Request, init?: RequestInit): Request;
  prototype: Request;
}

interface Response {
  readonly headers: Headers;
  readonly ok: boolean;
  readonly redirected: boolean;
  readonly status: number;
  readonly statusText: string;
  readonly type: ResponseType;
  readonly url: string;
  readonly body: ReadableStream<Uint8Array> | null;
  readonly bodyUsed: boolean;
  clone(): Response;
  arrayBuffer(): Promise<ArrayBuffer>;
  blob(): Promise<Blob>;
  formData(): Promise<FormData>;
  json(): Promise<any>;
  text(): Promise<string>;
}

interface ResponseConstructor {
  new (body?: BodyInit | null, init?: ResponseInit): Response;
  prototype: Response;
  error(): Response;
  redirect(url: string | URL, status?: number): Response;
  json(data: any, init?: ResponseInit): Response;
}

interface ResponseInit {
  headers?: HeadersInit;
  status?: number;
  statusText?: string;
}

type ResponseType =
  | "basic"
  | "cors"
  | "default"
  | "error"
  | "opaque"
  | "opaqueredirect";

// ============================================================================
// Blob & FormData
// ============================================================================

interface Blob {
  readonly size: number;
  readonly type: string;
  arrayBuffer(): Promise<ArrayBuffer>;
  slice(start?: number, end?: number, contentType?: string): Blob;
  stream(): ReadableStream<Uint8Array>;
  text(): Promise<string>;
}

type BlobPart = string | ArrayBuffer | ArrayBufferView | Blob;

interface BlobPropertyBag {
  type?: string;
  endings?: "native" | "transparent";
}

interface BlobConstructor {
  new (blobParts?: BlobPart[], options?: BlobPropertyBag): Blob;
  prototype: Blob;
}

interface FormData {
  append(name: string, value: string | Blob, fileName?: string): void;
  delete(name: string): void;
  get(name: string): FormDataEntryValue | null;
  getAll(name: string): FormDataEntryValue[];
  has(name: string): boolean;
  set(name: string, value: string | Blob, fileName?: string): void;
  forEach(
    callback: (
      value: FormDataEntryValue,
      key: string,
      parent: FormData,
    ) => void,
  ): void;
  keys(): IterableIterator<string>;
  values(): IterableIterator<FormDataEntryValue>;
  entries(): IterableIterator<[string, FormDataEntryValue]>;
  [Symbol.iterator](): IterableIterator<[string, FormDataEntryValue]>;
}

interface FormDataConstructor {
  new (): FormData;
  prototype: FormData;
}

type FormDataEntryValue = string | File;

interface File extends Blob {
  readonly lastModified: number;
  readonly name: string;
}

interface FilePropertyBag extends BlobPropertyBag {
  lastModified?: number;
}

interface FileConstructor {
  new (fileBits: BlobPart[], fileName: string, options?: FilePropertyBag): File;
  prototype: File;
}

// ============================================================================
// Streams API
// ============================================================================

interface ReadableStream<R = any> {
  readonly locked: boolean;
  cancel(reason?: any): Promise<void>;
  getReader(): ReadableStreamDefaultReader<R>;
  tee(): [ReadableStream<R>, ReadableStream<R>];
}

interface ReadableStreamDefaultReader<R = any> {
  readonly closed: Promise<undefined>;
  cancel(reason?: any): Promise<void>;
  read(): Promise<ReadableStreamReadResult<R>>;
  releaseLock(): void;
}

type ReadableStreamReadResult<T> =
  | { done: false; value: T }
  | { done: true; value?: undefined };

// ============================================================================
// AbortController & AbortSignal
// ============================================================================

interface AbortController {
  readonly signal: AbortSignal;
  abort(reason?: any): void;
}

interface AbortControllerConstructor {
  new (): AbortController;
  prototype: AbortController;
}

interface AbortSignal extends EventTarget {
  readonly aborted: boolean;
  readonly reason: any;
  onabort: ((this: AbortSignal, ev: Event) => any) | null;
  throwIfAborted(): void;
}

interface AbortSignalConstructor {
  prototype: AbortSignal;
  abort(reason?: any): AbortSignal;
  timeout(milliseconds: number): AbortSignal;
}

// ============================================================================
// Events
// ============================================================================

interface Event {
  readonly bubbles: boolean;
  readonly cancelable: boolean;
  readonly composed: boolean;
  readonly currentTarget: EventTarget | null;
  readonly defaultPrevented: boolean;
  readonly eventPhase: number;
  readonly isTrusted: boolean;
  readonly target: EventTarget | null;
  readonly timeStamp: number;
  readonly type: string;
  preventDefault(): void;
  stopImmediatePropagation(): void;
  stopPropagation(): void;
}

interface EventInit {
  bubbles?: boolean;
  cancelable?: boolean;
  composed?: boolean;
}

interface EventConstructor {
  new (type: string, eventInitDict?: EventInit): Event;
  prototype: Event;
}

interface EventTarget {
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | null,
    options?: boolean | AddEventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | null,
    options?: boolean | EventListenerOptions,
  ): void;
  dispatchEvent(event: Event): boolean;
}

interface EventTargetConstructor {
  new (): EventTarget;
  prototype: EventTarget;
}

interface EventListener {
  (evt: Event): void;
}

interface EventListenerObject {
  handleEvent(evt: Event): void;
}

type EventListenerOrEventListenerObject = EventListener | EventListenerObject;

interface AddEventListenerOptions extends EventListenerOptions {
  once?: boolean;
  passive?: boolean;
  signal?: AbortSignal;
}

interface EventListenerOptions {
  capture?: boolean;
}

// ============================================================================
// TextEncoder / TextDecoder
// ============================================================================

interface TextEncoder {
  readonly encoding: string;
  encode(input?: string): Uint8Array;
  encodeInto(
    source: string,
    destination: Uint8Array,
  ): { read: number; written: number };
}

interface TextEncoderConstructor {
  new (): TextEncoder;
  prototype: TextEncoder;
}

interface TextDecoder {
  readonly encoding: string;
  readonly fatal: boolean;
  readonly ignoreBOM: boolean;
  decode(
    input?: ArrayBuffer | ArrayBufferView,
    options?: TextDecodeOptions,
  ): string;
}

interface TextDecoderOptions {
  fatal?: boolean;
  ignoreBOM?: boolean;
}

interface TextDecodeOptions {
  stream?: boolean;
}

interface TextDecoderConstructor {
  new (label?: string, options?: TextDecoderOptions): TextDecoder;
  prototype: TextDecoder;
}

// ============================================================================
// Structured Clone
// ============================================================================

interface StructuredSerializeOptions {
  transfer?: Transferable[];
}

type Transferable = ArrayBuffer;

// ============================================================================
// WebSocket
// ============================================================================

interface WebSocket extends EventTarget {
  readonly binaryType: BinaryType;
  readonly bufferedAmount: number;
  readonly extensions: string;
  readonly protocol: string;
  readonly readyState: number;
  readonly url: string;
  onclose: ((this: WebSocket, ev: CloseEvent) => any) | null;
  onerror: ((this: WebSocket, ev: Event) => any) | null;
  onmessage: ((this: WebSocket, ev: MessageEvent) => any) | null;
  onopen: ((this: WebSocket, ev: Event) => any) | null;
  close(code?: number, reason?: string): void;
  send(data: string | ArrayBuffer | Blob | ArrayBufferView): void;
  readonly CONNECTING: 0;
  readonly OPEN: 1;
  readonly CLOSING: 2;
  readonly CLOSED: 3;
}

interface WebSocketConstructor {
  new (url: string | URL, protocols?: string | string[]): WebSocket;
  prototype: WebSocket;
  readonly CONNECTING: 0;
  readonly OPEN: 1;
  readonly CLOSING: 2;
  readonly CLOSED: 3;
}

type BinaryType = "arraybuffer" | "blob";

interface CloseEvent extends Event {
  readonly code: number;
  readonly reason: string;
  readonly wasClean: boolean;
}

interface MessageEvent<T = any> extends Event {
  readonly data: T;
  readonly lastEventId: string;
  readonly origin: string;
  readonly source: MessageEventSource | null;
  readonly ports: readonly MessagePort[];
}

type MessageEventSource = Window | MessagePort | ServiceWorker;

// ============================================================================
// Worker API (High-Performance Web Workers)
// ============================================================================

interface WorkerOptions {
  /** Module specifiers to load before the worker script */
  preload?: string | string[];
  /** Use reduced memory mode (slower but uses less RAM) */
  smol?: boolean;
  /** Whether this worker keeps the process alive (default: true) */
  ref?: boolean;
  /** Worker name for debugging */
  name?: string;
  /** Ignored for compatibility - Viper workers always support ES modules */
  type?: "classic" | "module";
}

interface Worker extends EventTarget {
  /** Unique thread identifier */
  readonly threadId: number;

  /** Send a message to the worker */
  postMessage(message: any, transfer?: Transferable[]): void;
  postMessage(message: any, options?: StructuredSerializeOptions): void;

  /** Terminate the worker immediately */
  terminate(): void;

  /** Keep the process alive while this worker is running */
  ref(): this;

  /** Allow the process to exit even if this worker is still running */
  unref(): this;

  /** Called when the worker is ready to receive messages */
  onopen: ((this: Worker, ev: Event) => any) | null;

  /** Called when a message is received from the worker */
  onmessage: ((this: Worker, ev: MessageEvent) => any) | null;

  /** Called when an error occurs in the worker */
  onerror: ((this: Worker, ev: ErrorEvent) => any) | null;

  /** Called when the worker is closed */
  onclose: ((this: Worker, ev: CloseEvent) => any) | null;

  addEventListener<K extends keyof WorkerEventMap>(
    type: K,
    listener: (this: Worker, ev: WorkerEventMap[K]) => any,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void;

  removeEventListener<K extends keyof WorkerEventMap>(
    type: K,
    listener: (this: Worker, ev: WorkerEventMap[K]) => any,
    options?: boolean | EventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions,
  ): void;
}

interface WorkerEventMap {
  open: Event;
  message: MessageEvent;
  messageerror: MessageEvent;
  error: ErrorEvent;
  close: CloseEvent;
}

interface WorkerConstructor {
  new (scriptURL: string | URL, options?: WorkerOptions): Worker;
  prototype: Worker;
}

interface ErrorEvent extends Event {
  readonly message: string;
  readonly filename: string;
  readonly lineno: number;
  readonly colno: number;
  readonly error: any;
}

// ============================================================================
// MessageChannel & MessagePort
// ============================================================================

interface MessageChannel {
  readonly port1: MessagePort;
  readonly port2: MessagePort;
}

interface MessageChannelConstructor {
  new (): MessageChannel;
  prototype: MessageChannel;
}

interface MessagePort extends EventTarget {
  onmessage: ((this: MessagePort, ev: MessageEvent) => any) | null;
  onmessageerror: ((this: MessagePort, ev: MessageEvent) => any) | null;

  close(): void;
  postMessage(message: any, transfer?: Transferable[]): void;
  postMessage(message: any, options?: StructuredSerializeOptions): void;
  start(): void;

  addEventListener<K extends keyof MessagePortEventMap>(
    type: K,
    listener: (this: MessagePort, ev: MessagePortEventMap[K]) => any,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void;

  removeEventListener<K extends keyof MessagePortEventMap>(
    type: K,
    listener: (this: MessagePort, ev: MessagePortEventMap[K]) => any,
    options?: boolean | EventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions,
  ): void;
}

interface MessagePortEventMap {
  message: MessageEvent;
  messageerror: MessageEvent;
}

// ============================================================================
// Worker Global Scope (inside workers)
// ============================================================================

interface DedicatedWorkerGlobalScope extends WorkerGlobalScope {
  readonly name: string;

  onmessage:
    | ((this: DedicatedWorkerGlobalScope, ev: MessageEvent) => any)
    | null;
  onmessageerror:
    | ((this: DedicatedWorkerGlobalScope, ev: MessageEvent) => any)
    | null;

  close(): void;
  postMessage(message: any, transfer?: Transferable[]): void;
  postMessage(message: any, options?: StructuredSerializeOptions): void;
}

interface WorkerGlobalScope extends EventTarget {
  readonly self: WorkerGlobalScope;
  readonly location: WorkerLocation;

  onerror: ((this: WorkerGlobalScope, ev: ErrorEvent) => any) | null;
}

interface WorkerLocation {
  readonly href: string;
  readonly origin: string;
  readonly protocol: string;
  readonly host: string;
  readonly hostname: string;
  readonly port: string;
  readonly pathname: string;
  readonly search: string;
  readonly hash: string;
}

// ============================================================================
// worker_threads compatibility (Node.js style)
// ============================================================================

interface WorkerThreads {
  readonly isMainThread: boolean;
  readonly parentPort: MessagePort | null;
  readonly workerData: any;
  readonly Worker: WorkerConstructor;
  readonly threadId: number;

  setEnvironmentData(key: any, value: any): void;
  getEnvironmentData(key: any): any;
}

// ============================================================================
// Console API
// ============================================================================

interface Console {
  log(...args: any[]): void;
  error(...args: any[]): void;
  warn(...args: any[]): void;
  info(...args: any[]): void;
  debug(...args: any[]): void;
  trace(...args: any[]): void;
  dir(obj: any): void;
  table(data: any): void;
  time(label?: string): void;
  timeEnd(label?: string): void;
  timeLog(label?: string, ...args: any[]): void;
  count(label?: string): void;
  countReset(label?: string): void;
  group(...args: any[]): void;
  groupCollapsed(...args: any[]): void;
  groupEnd(): void;
  clear(): void;
  assert(condition?: boolean, ...args: any[]): void;
}

// ============================================================================
// Path Module Types
// ============================================================================

interface ParsedPath {
  root: string;
  dir: string;
  base: string;
  ext: string;
  name: string;
}

interface FormatInputPathObject {
  root?: string;
  dir?: string;
  base?: string;
  name?: string;
  ext?: string;
}

interface PlatformPath {
  sep: string;
  delimiter: string;
  join(...paths: string[]): string;
  resolve(...paths: string[]): string;
  normalize(path: string): string;
  dirname(path: string): string;
  basename(path: string, ext?: string): string;
  extname(path: string): string;
  isAbsolute(path: string): boolean;
  relative(from: string, to: string): string;
  parse(path: string): ParsedPath;
  format(pathObject: FormatInputPathObject): string;
  toNamespacedPath(path: string): string;
  matchesGlob(path: string, pattern: string): boolean;
  posix: PlatformPath;
  win32: PlatformPath;
}

// ============================================================================
// Process Object
// ============================================================================

interface ProcessVersions {
  viper: string;
  boa: string;
  oxc: string;
}

interface Process {
  argv: string[];
  exit(code?: number): never;
  cwd(): string;
  env: Record<string, string | undefined>;
  pid: number;
  ppid: number;
  platform: "win32" | "darwin" | "linux" | "unknown";
  arch: "x64" | "arm64" | "ia32" | "arm" | "unknown";
  version: string;
  versions: ProcessVersions;
  title: string;
  execPath: string;
  hrtime: {
    (prev?: [number, number]): [number, number];
    bigint(): bigint;
  };
  memoryUsage(): {
    rss: number;
    heapTotal: number;
    heapUsed: number;
    external: number;
    arrayBuffers: number;
  };
  cpuUsage(): { user: number; system: number };
  uptime(): number;
  nextTick(callback: (...args: any[]) => void, ...args: any[]): void;
  stdout: { write(data: string): boolean; isTTY: boolean };
  stderr: { write(data: string): boolean; isTTY: boolean };
  stdin: { isTTY: boolean };
  on(event: string, listener: (...args: any[]) => void): Process;
  once(event: string, listener: (...args: any[]) => void): Process;
  off(event: string, listener: (...args: any[]) => void): Process;
  emit(event: string, ...args: any[]): boolean;
}

// ============================================================================
// Crypto API
// ============================================================================

interface ViperCrypto {
  randomUUID(): string;
  getRandomValues<T extends ArrayBufferView>(array: T): T;
  randomBytes(size: number): Uint8Array;
  subtle: {
    digest(algorithm: string, data: ArrayBuffer): Promise<ArrayBuffer>;
    encrypt(algorithm: any, key: any, data: ArrayBuffer): Promise<ArrayBuffer>;
    decrypt(algorithm: any, key: any, data: ArrayBuffer): Promise<ArrayBuffer>;
    sign(algorithm: any, key: any, data: ArrayBuffer): Promise<ArrayBuffer>;
    verify(
      algorithm: any,
      key: any,
      signature: ArrayBuffer,
      data: ArrayBuffer,
    ): Promise<boolean>;
    generateKey(
      algorithm: any,
      extractable: boolean,
      keyUsages: string[],
    ): Promise<any>;
    importKey(
      format: string,
      keyData: any,
      algorithm: any,
      extractable: boolean,
      keyUsages: string[],
    ): Promise<any>;
    exportKey(format: string, key: any): Promise<any>;
  };
}

// ============================================================================
// File System API
// ============================================================================

interface ViperFile {
  text(): Promise<string>;
  arrayBuffer(): Promise<ArrayBuffer>;
  exists(): Promise<boolean>;
}

interface FileStat {
  isFile: boolean;
  isDirectory: boolean;
  size: number;
  mtime: number;
}

// ============================================================================
// Viper Namespace
// ============================================================================

interface SpawnResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

interface SpawnOptions {
  cwd?: string;
  env?: Record<string, string>;
  shell?: boolean;
}

interface ServeOptions {
  port?: number;
  hostname?: string;
  fetch: (request: Request) => Response | Promise<Response>;
}

interface ServerInfo {
  port: number;
  hostname: string;
}

interface ViperNamespace {
  spawn(
    command: string,
    args?: string[],
    options?: SpawnOptions,
  ): Promise<SpawnResult>;
  exec(command: string): Promise<SpawnResult>;
  serve(options: ServeOptions): ServerInfo;
}

// ============================================================================
// Module Declarations
// ============================================================================

declare module "path" {
  export const sep: string;
  export const delimiter: string;
  export function join(...paths: string[]): string;
  export function resolve(...paths: string[]): string;
  export function normalize(path: string): string;
  export function dirname(path: string): string;
  export function basename(path: string, ext?: string): string;
  export function extname(path: string): string;
  export function isAbsolute(path: string): boolean;
  export function relative(from: string, to: string): string;
  export function parse(path: string): ParsedPath;
  export function format(pathObject: FormatInputPathObject): string;
  export function toNamespacedPath(path: string): string;
  export function matchesGlob(path: string, pattern: string): boolean;
  export const posix: PlatformPath;
  export const win32: PlatformPath;
  const path: PlatformPath;
  export default path;
}

declare module "node:path" {
  export * from "path";
  export { default } from "path";
}

// ============================================================================
// Global Declarations
// ============================================================================

declare global {
  // Console
  var console: Console;

  // URL
  var URL: URLConstructor;
  var URLSearchParams: URLSearchParamsConstructor;

  // Fetch API
  function fetch(
    input: string | URL | Request,
    init?: RequestInit,
  ): Promise<Response>;
  var Headers: HeadersConstructor;
  var Request: RequestConstructor;
  var Response: ResponseConstructor;

  // Blob & FormData
  var Blob: BlobConstructor;
  var File: FileConstructor;
  var FormData: FormDataConstructor;

  // Text encoding
  var TextEncoder: TextEncoderConstructor;
  var TextDecoder: TextDecoderConstructor;

  // Events
  var Event: EventConstructor;
  var EventTarget: EventTargetConstructor;

  // Abort
  var AbortController: AbortControllerConstructor;
  var AbortSignal: AbortSignalConstructor;

  // WebSocket
  var WebSocket: WebSocketConstructor;

  // Worker API
  var Worker: WorkerConstructor;
  var MessageChannel: MessageChannelConstructor;
  var MessagePort: MessagePort;

  // Timers
  function setTimeout(
    callback: (...args: any[]) => void,
    ms?: number,
    ...args: any[]
  ): number;
  function setInterval(
    callback: (...args: any[]) => void,
    ms?: number,
    ...args: any[]
  ): number;
  function clearTimeout(id?: number): void;
  function clearInterval(id?: number): void;
  function queueMicrotask(callback: () => void): void;

  // Structured Clone
  function structuredClone<T>(
    value: T,
    options?: StructuredSerializeOptions,
  ): T;

  // Path module (global)
  var path: PlatformPath;

  // Process object
  var process: Process;

  // Crypto API
  var crypto: ViperCrypto;

  // Viper namespace
  var Viper: ViperNamespace & {
    /** true if running in the main thread, false in workers */
    isMainThread: boolean;
  };

  // worker_threads compatibility
  var __worker_threads: WorkerThreads;

  // File system functions
  function file(path: string, options?: { type?: string }): ViperFile;
  function write(path: string, data: string | Uint8Array): Promise<void>;
  function readFile(path: string): Promise<string>;
  function exists(path: string): Promise<boolean>;
  function readDir(path: string): Promise<string[]>;
  function mkdir(
    path: string,
    options?: { recursive?: boolean },
  ): Promise<void>;
  function stat(path: string): Promise<FileStat>;

  // JSX Runtime
  function __viper_jsx(
    type: string | ((props: any) => any),
    props: any,
    ...children: any[]
  ): any;
  function __viper_fragment(props: any, ...children: any[]): any;
  function renderToString(element: any): string;

  // Constants
  var __VIPER_VERSION__: string;
  var __VIPER_RUNTIME__: string;
}

export {};
