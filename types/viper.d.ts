/**
 * Viper TypeScript Runtime Type Definitions
 *
 * Similar to Bun and Deno, these types provide autocompletion and type checking
 * for Viper-specific APIs.
 */

/// <reference lib="es2023" />

// ============================================================================
// Global Viper Runtime Information
// ============================================================================

declare global {
  /**
   * The version of the Viper runtime
   */
  const __VIPER_VERSION__: string;

  /**
   * The name of the runtime
   */
  const __VIPER_RUNTIME__: "Viper";

  // ============================================================================
  // File System API
  // ============================================================================

  /**
   * A reference to a file that lazily loads contents
   * Similar to Bun's BunFile
   */
  interface ViperFile {
    /**
     * The absolute path to the file
     */
    readonly path: string;

    /**
     * The MIME type of the file (auto-detected from extension)
     */
    readonly type: string;

    /**
     * Read the file contents as a UTF-8 string
     * @returns A promise that resolves to the file contents
     */
    text(): Promise<string>;

    /**
     * Read and parse the file contents as JSON
     * @returns A promise that resolves to the parsed JSON data
     */
    json<T = any>(): Promise<T>;

    /**
     * Check if the file exists
     * @returns A promise that resolves to true if the file exists
     */
    exists(): Promise<boolean>;

    /**
     * Get the size of the file in bytes
     * @returns A promise that resolves to the file size
     */
    size(): Promise<number>;

    /**
     * Delete the file
     * @returns A promise that resolves when the file is deleted
     */
    delete(): Promise<void>;

    /**
     * Get file stats
     * @returns A promise that resolves to file statistics
     */
    stat(): Promise<ViperFileStats>;

    /**
     * Create a writer for incremental file writing
     * @param options Optional configuration for the writer
     * @returns A FileSink for writing data incrementally
     */
    writer(options?: FileSinkOptions): ViperFileSink;
  }

  /**
   * File statistics
   */
  interface ViperFileStats {
    /**
     * Size in bytes
     */
    size: number;

    /**
     * Whether this is a file
     */
    isFile: boolean;

    /**
     * Whether this is a directory
     */
    isDirectory: boolean;

    /**
     * Whether this is a symbolic link
     */
    isSymlink: boolean;
  }

  /**
   * Options for creating a FileSink
   */
  interface FileSinkOptions {
    /**
     * The high water mark for buffering (in bytes)
     * Default: 16384 (16KB)
     */
    highWaterMark?: number;
  }

  /**
   * Incremental file writer with buffering
   * Similar to Bun's FileSink
   */
  interface ViperFileSink {
    /**
     * Write a chunk of data to the buffer
     * @param chunk String, ArrayBuffer, or TypedArray to write
     */
    write(chunk: string | ArrayBuffer | ArrayBufferView): void;

    /**
     * Flush the buffer to disk
     * @returns A promise that resolves when the flush is complete
     */
    flush(): Promise<void>;

    /**
     * Flush and close the file
     * @returns A promise that resolves to the total bytes written
     */
    end(): Promise<number>;
  }

  /**
   * Create a file reference
   * @param path The path to the file
   * @param options Optional file options
   * @returns A ViperFile reference
   *
   * @example
   * ```ts
   * const f = file("data.json");
   * const data = await f.json();
   * console.log(data);
   * ```
   */
  function file(path: string, options?: { type?: string }): ViperFile;

  /**
   * Write data to a file
   *
   * @param destination The destination path or ViperFile
   * @param data The data to write (string, ArrayBuffer, TypedArray, or another ViperFile)
   * @returns A promise that resolves to the number of bytes written
   *
   * @example
   * ```ts
   * // Write a string
   * await write("output.txt", "Hello, Viper!");
   *
   * // Write JSON
   * await write("data.json", JSON.stringify({ name: "Viper" }));
   *
   * // Copy a file
   * await write(file("dest.txt"), file("source.txt"));
   * ```
   */
  function write(
    destination: string | ViperFile,
    data: string | ArrayBuffer | ArrayBufferView | ViperFile,
  ): Promise<number>;

  // ============================================================================
  // JSX Runtime
  // ============================================================================

  /**
   * JSX element creation function (classic JSX runtime)
   * @internal
   */
  function __viper_jsx(
    type: string | Function,
    props: Record<string, any> | null,
    ...children: any[]
  ): any;

  /**
   * JSX fragment function
   * @internal
   */
  function __viper_fragment(
    props: Record<string, any> | null,
    ...children: any[]
  ): any;

  /**
   * Render a JSX element to an HTML string
   * @param element The JSX element to render
   * @returns The rendered HTML string
   *
   * @example
   * ```tsx
   * const html = renderToString(<div className="greeting">Hello!</div>);
   * console.log(html); // <div class="greeting">Hello!</div>
   * ```
   */
  function renderToString(element: any): string;

  // ============================================================================
  // Standard Web APIs
  // ============================================================================

  // Console API
  interface Console {
    log(...data: any[]): void;
    info(...data: any[]): void;
    warn(...data: any[]): void;
    error(...data: any[]): void;
    debug(...data: any[]): void;
    trace(...data: any[]): void;
    assert(condition?: boolean, ...data: any[]): void;
    clear(): void;
    count(label?: string): void;
    countReset(label?: string): void;
    group(...data: any[]): void;
    groupCollapsed(...data: any[]): void;
    groupEnd(): void;
    time(label?: string): void;
    timeLog(label?: string, ...data: any[]): void;
    timeEnd(label?: string): void;
  }

  const console: Console;

  // Timer functions
  function setTimeout(
    callback: (...args: any[]) => void,
    ms?: number,
    ...args: any[]
  ): number;
  function clearTimeout(id: number): void;
  function setInterval(
    callback: (...args: any[]) => void,
    ms?: number,
    ...args: any[]
  ): number;
  function clearInterval(id: number): void;

  // Microtask
  function queueMicrotask(callback: () => void): void;

  // URL API
  class URL {
    constructor(url: string, base?: string | URL);
    href: string;
    origin: string;
    protocol: string;
    username: string;
    password: string;
    host: string;
    hostname: string;
    port: string;
    pathname: string;
    search: string;
    searchParams: URLSearchParams;
    hash: string;
    toString(): string;
    toJSON(): string;
  }

  class URLSearchParams {
    constructor(
      init?:
        | string
        | URLSearchParams
        | Record<string, string>
        | [string, string][],
    );
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
    [Symbol.iterator](): IterableIterator<[string, string]>;
    entries(): IterableIterator<[string, string]>;
    keys(): IterableIterator<string>;
    values(): IterableIterator<string>;
  }

  // Text Encoding API
  class TextEncoder {
    readonly encoding: string;
    encode(input?: string): Uint8Array;
  }

  class TextDecoder {
    constructor(
      label?: string,
      options?: { fatal?: boolean; ignoreBOM?: boolean },
    );
    readonly encoding: string;
    readonly fatal: boolean;
    readonly ignoreBOM: boolean;
    decode(input?: BufferSource, options?: { stream?: boolean }): string;
  }

  // Structured Clone
  function structuredClone<T>(value: T, options?: { transfer?: any[] }): T;

  // Global object reference (Node.js compatibility)
  const global: typeof globalThis;

  // ============================================================================
  // Process API (Node.js compatible)
  // ============================================================================

  /**
   * Process object providing information about the current process
   */
  const process: {
    /**
     * Command-line arguments passed to the process
     * First element is the executable path, second is the script path
     */
    readonly argv: string[];

    /**
     * Exit the process with an optional exit code
     * @param code Exit code (default: 0)
     */
    exit(code?: number): never;

    /**
     * Get the current working directory
     */
    cwd(): string;

    /**
     * Environment variables
     */
    readonly env: Record<string, string | undefined>;

    /**
     * Process ID
     */
    readonly pid: number;

    /**
     * Parent process ID
     */
    readonly ppid: number;

    /**
     * Operating system platform
     * 'win32' | 'darwin' | 'linux' | 'unknown'
     */
    readonly platform: string;

    /**
     * CPU architecture
     * 'x64' | 'arm64' | 'ia32' | 'arm' | 'unknown'
     */
    readonly arch: string;

    /**
     * Viper version string (prefixed with 'v')
     */
    readonly version: string;

    /**
     * Version information for runtime components
     */
    readonly versions: {
      viper: string;
      boa: string;
      oxc: string;
    };

    /**
     * Process title
     */
    readonly title: string;

    /**
     * Path to the executable
     */
    readonly execPath: string;

    /**
     * High-resolution time
     * @param prev Optional previous hrtime to calculate difference
     * @returns [seconds, nanoseconds]
     */
    hrtime(prev?: [number, number]): [number, number];

    /**
     * High-resolution time as BigInt nanoseconds
     */
    hrtime: {
      (prev?: [number, number]): [number, number];
      bigint(): bigint;
    };

    /**
     * Get memory usage (approximate)
     */
    memoryUsage(): {
      rss: number;
      heapTotal: number;
      heapUsed: number;
      external: number;
      arrayBuffers: number;
    };

    /**
     * Get CPU usage
     */
    cpuUsage(): { user: number; system: number };

    /**
     * Get process uptime in seconds
     */
    uptime(): number;

    /**
     * Schedule a callback to run before the next event loop iteration
     * @param callback Function to call
     * @param args Arguments to pass to the callback
     */
    nextTick<T extends any[]>(callback: (...args: T) => void, ...args: T): void;

    /**
     * Standard output stream
     */
    readonly stdout: {
      write(data: string): boolean;
      readonly isTTY: boolean;
    };

    /**
     * Standard error stream
     */
    readonly stderr: {
      write(data: string): boolean;
      readonly isTTY: boolean;
    };

    /**
     * Standard input stream
     */
    readonly stdin: {
      readonly isTTY: boolean;
    };

    /**
     * Add an event listener
     */
    on(event: string, listener: (...args: any[]) => void): typeof process;

    /**
     * Add a one-time event listener
     */
    once(event: string, listener: (...args: any[]) => void): typeof process;

    /**
     * Remove an event listener
     */
    off(event: string, listener: (...args: any[]) => void): typeof process;

    /**
     * Emit an event
     */
    emit(event: string, ...args: any[]): boolean;
  };

  // ============================================================================
  // Crypto API (Web Crypto compatible)
  // ============================================================================

  /**
   * Cryptographic functionality
   */
  const crypto: {
    /**
     * Generate a random UUID v4
     * @returns A random UUID string
     *
     * @example
     * ```ts
     * const id = crypto.randomUUID();
     * // "550e8400-e29b-41d4-a716-446655440000"
     * ```
     */
    randomUUID(): string;

    /**
     * Fill a typed array with cryptographically random values
     * @param array The typed array to fill
     * @returns The same array, filled with random values
     *
     * @example
     * ```ts
     * const bytes = new Uint8Array(16);
     * crypto.getRandomValues(bytes);
     * ```
     */
    getRandomValues<T extends ArrayBufferView>(array: T): T;

    /**
     * Generate random bytes (Node.js compatible)
     * @param size Number of bytes to generate
     * @returns A Uint8Array with random bytes
     *
     * @example
     * ```ts
     * const bytes = crypto.randomBytes(32);
     * ```
     */
    randomBytes(size: number): Uint8Array;

    /**
     * Web Crypto subtle interface (partial implementation)
     */
    readonly subtle: {
      digest(algorithm: string, data: BufferSource): Promise<ArrayBuffer>;
      encrypt(
        algorithm: any,
        key: CryptoKey,
        data: BufferSource,
      ): Promise<ArrayBuffer>;
      decrypt(
        algorithm: any,
        key: CryptoKey,
        data: BufferSource,
      ): Promise<ArrayBuffer>;
      sign(
        algorithm: any,
        key: CryptoKey,
        data: BufferSource,
      ): Promise<ArrayBuffer>;
      verify(
        algorithm: any,
        key: CryptoKey,
        signature: BufferSource,
        data: BufferSource,
      ): Promise<boolean>;
      generateKey(
        algorithm: any,
        extractable: boolean,
        keyUsages: string[],
      ): Promise<CryptoKey | CryptoKeyPair>;
      importKey(
        format: string,
        keyData: BufferSource,
        algorithm: any,
        extractable: boolean,
        keyUsages: string[],
      ): Promise<CryptoKey>;
      exportKey(format: string, key: CryptoKey): Promise<ArrayBuffer>;
    };
  };

  /**
   * CryptoKey interface (Web Crypto)
   */
  interface CryptoKey {
    readonly type: string;
    readonly extractable: boolean;
    readonly algorithm: any;
    readonly usages: string[];
  }

  /**
   * CryptoKeyPair interface (Web Crypto)
   */
  interface CryptoKeyPair {
    readonly publicKey: CryptoKey;
    readonly privateKey: CryptoKey;
  }

  // ============================================================================
  // Fetch API
  // ============================================================================

  /**
   * Fetch a resource from the network
   *
   * @param input URL string or Request object
   * @param init Optional request configuration
   * @returns A promise that resolves to a Response
   *
   * @example
   * ```ts
   * // Simple GET request
   * const response = await fetch("https://api.example.com/data");
   * const data = await response.json();
   *
   * // POST request with JSON body
   * const response = await fetch("https://api.example.com/users", {
   *   method: "POST",
   *   headers: { "Content-Type": "application/json" },
   *   body: JSON.stringify({ name: "Alice" })
   * });
   *
   * // Using Request object
   * const request = new Request("https://api.example.com/data", {
   *   method: "GET",
   *   headers: { "Authorization": "Bearer token" }
   * });
   * const response = await fetch(request);
   * ```
   */
  function fetch(
    input: string | Request,
    init?: RequestInit,
  ): Promise<Response>;

  // ============================================================================
  // Web API: Headers
  // ============================================================================

  /**
   * HTTP Headers interface (Web API)
   */
  class Headers {
    constructor(init?: HeadersInit);

    /**
     * Append a value to an existing header, or create a new header
     */
    append(name: string, value: string): void;

    /**
     * Delete a header
     */
    delete(name: string): void;

    /**
     * Get a header value
     */
    get(name: string): string | null;

    /**
     * Check if a header exists
     */
    has(name: string): boolean;

    /**
     * Set a header value (replaces existing)
     */
    set(name: string, value: string): void;

    /**
     * Iterate over all headers
     */
    forEach(
      callback: (value: string, key: string, parent: Headers) => void,
    ): void;

    /**
     * Get an iterator over header entries
     */
    entries(): IterableIterator<[string, string]>;

    /**
     * Get an iterator over header names
     */
    keys(): IterableIterator<string>;

    /**
     * Get an iterator over header values
     */
    values(): IterableIterator<string>;

    [Symbol.iterator](): IterableIterator<[string, string]>;
  }

  type HeadersInit = Headers | Record<string, string> | [string, string][];

  // ============================================================================
  // Web API: Request
  // ============================================================================

  /**
   * HTTP Request interface (Web API)
   */
  class Request {
    constructor(input: string | Request, init?: RequestInit);

    /**
     * The request URL
     */
    readonly url: string;

    /**
     * The HTTP method
     */
    readonly method: string;

    /**
     * Request headers
     */
    readonly headers: Headers;

    /**
     * URL parameters extracted by the router
     */
    readonly params: Record<string, string>;

    /**
     * Query string parameters parsed from the URL
     */
    readonly query: Record<string, string>;

    /**
     * The request body
     */
    readonly body: any;

    /**
     * Read body as text
     */
    text(): Promise<string>;

    /**
     * Read body as JSON
     */
    json<T = any>(): Promise<T>;

    /**
     * Read body as FormData
     */
    formData(): Promise<Map<string, string>>;

    /**
     * Clone the request
     */
    clone(): Request;
  }

  interface RequestInit {
    /**
     * HTTP method
     */
    method?: string;

    /**
     * Request headers
     */
    headers?: HeadersInit;

    /**
     * Request body
     */
    body?: string | ArrayBuffer | ArrayBufferView | null;

    /**
     * URL parameters (set by router)
     */
    params?: Record<string, string>;
  }

  // ============================================================================
  // Web API: Response
  // ============================================================================

  /**
   * HTTP Response interface (Web API)
   */
  class Response {
    constructor(body?: BodyInit | null, init?: ResponseInit);

    /**
     * HTTP status code
     */
    readonly status: number;

    /**
     * HTTP status text
     */
    readonly statusText: string;

    /**
     * Response headers
     */
    readonly headers: Headers;

    /**
     * Whether the response was successful (status 200-299)
     */
    readonly ok: boolean;

    /**
     * The response body
     */
    readonly body: any;

    /**
     * Read body as text
     */
    text(): Promise<string>;

    /**
     * Read body as JSON
     */
    json<T = any>(): Promise<T>;

    /**
     * Clone the response
     */
    clone(): Response;

    /**
     * Create a JSON response
     * @param data Data to serialize as JSON
     * @param init Optional response init
     */
    static json(data: any, init?: ResponseInit): Response;

    /**
     * Create a redirect response
     * @param url URL to redirect to
     * @param status HTTP status code (default: 302)
     */
    static redirect(url: string, status?: number): Response;

    /**
     * Create an error response
     */
    static error(): Response;
  }

  interface ResponseInit {
    /**
     * HTTP status code
     */
    status?: number;

    /**
     * HTTP status text
     */
    statusText?: string;

    /**
     * Response headers
     */
    headers?: HeadersInit;
  }

  type BodyInit = string | ArrayBuffer | ArrayBufferView | null;

  // ============================================================================
  // Viper Namespace
  // ============================================================================

  /**
   * Viper namespace containing runtime-specific APIs
   */
  namespace Viper {
    // ========================================================================
    // Router API
    // ========================================================================

    /**
     * Route handler function
     */
    type RouteHandler = (request: Request) => Response | Promise<Response>;

    /**
     * Middleware function
     * Return a Response to short-circuit, or undefined to continue
     */
    type MiddlewareHandler = (request: Request) => Response | void;

    /**
     * Bun-like HTTP Router
     *
     * @example
     * ```ts
     * const router = new Viper.Router();
     *
     * router.get("/", () => new Response("Home"));
     * router.get("/users/:id", (req) => {
     *   return Response.json({ id: req.params.id });
     * });
     *
     * Viper.serve({ port: 3000, fetch: router.fetch });
     * ```
     */
    class Router {
      constructor();

      /**
       * The fetch handler bound to this router instance.
       * Pass this to Viper.serve() as the fetch handler.
       */
      readonly fetch: RouteHandler;

      /**
       * Add middleware that runs before every request
       * @param handler Middleware function
       */
      use(handler: MiddlewareHandler): this;

      /**
       * Register a GET route
       * @param path URL path pattern (supports :param and * wildcard)
       * @param handler Route handler
       */
      get(path: string, handler: RouteHandler): this;

      /**
       * Register a POST route
       * @param path URL path pattern
       * @param handler Route handler
       */
      post(path: string, handler: RouteHandler): this;

      /**
       * Register a PUT route
       * @param path URL path pattern
       * @param handler Route handler
       */
      put(path: string, handler: RouteHandler): this;

      /**
       * Register a DELETE route
       * @param path URL path pattern
       * @param handler Route handler
       */
      delete(path: string, handler: RouteHandler): this;

      /**
       * Register a PATCH route
       * @param path URL path pattern
       * @param handler Route handler
       */
      patch(path: string, handler: RouteHandler): this;

      /**
       * Register a HEAD route
       * @param path URL path pattern
       * @param handler Route handler
       */
      head(path: string, handler: RouteHandler): this;

      /**
       * Register an OPTIONS route
       * @param path URL path pattern
       * @param handler Route handler
       */
      options(path: string, handler: RouteHandler): this;

      /**
       * Register a route that matches all HTTP methods
       * @param path URL path pattern
       * @param handler Route handler
       */
      all(path: string, handler: RouteHandler): this;

      /**
       * Group routes under a common prefix
       * @param prefix URL prefix for all routes in the group
       * @param callback Function that receives a sub-router
       */
      group(prefix: string, callback: (router: Router) => void): this;

      /**
       * Handle an incoming request (internal)
       * @param request The incoming request
       */
      handle(request: Request): Response;
    }

    // ========================================================================
    // Server API
    // ========================================================================

    /**
     * Server configuration options
     */
    interface ServeOptions {
      /**
       * The port to listen on
       * @default 3000
       */
      port?: number;

      /**
       * The hostname to bind to
       * @default "127.0.0.1"
       */
      hostname?: string;

      /**
       * Request handler function
       * Called for each incoming HTTP request
       */
      fetch: RouteHandler;
    }

    /**
     * Start an HTTP server
     *
     * Similar to Bun.serve(), this function starts a fast HTTP server
     * powered by Hyper on a single thread for maximum performance.
     *
     * @param options Server configuration
     *
     * @example
     * ```ts
     * // Simple server
     * Viper.serve({
     *   port: 3000,
     *   fetch(request) {
     *     return new Response("Hello from Viper!");
     *   }
     * });
     * ```
     *
     * @example
     * ```ts
     * // With router
     * const router = new Viper.Router();
     * router.get("/", () => new Response("Home"));
     * router.get("/api/users/:id", (req) => {
     *   return Response.json({ id: req.params.id });
     * });
     *
     * Viper.serve({ port: 8080, fetch: router.fetch });
     * ```
     */
    function serve(options: ServeOptions): void;

    // ========================================================================
    // File System API
    // ========================================================================

    /**
     * Create a file reference
     * @param path Path to the file
     * @param options Optional file options
     *
     * @example
     * ```ts
     * const f = Viper.file("data.json");
     * const data = await f.json();
     * ```
     */
    function file(path: string, options?: { type?: string }): ViperFile;

    /**
     * Write data to a file
     * @param destination Path or ViperFile to write to
     * @param data Data to write
     *
     * @example
     * ```ts
     * await Viper.write("output.txt", "Hello!");
     * ```
     */
    function write(
      destination: string | ViperFile,
      data: string | ArrayBuffer | ArrayBufferView | ViperFile,
    ): Promise<number>;

    /**
     * Read a file as text
     * @param path Path to the file
     *
     * @example
     * ```ts
     * const content = await Viper.readFile("config.json");
     * ```
     */
    function readFile(path: string): Promise<string>;

    /**
     * Read directory contents
     * @param path Path to the directory
     * @returns Array of file/directory names
     *
     * @example
     * ```ts
     * const files = await Viper.readDir("./src");
     * ```
     */
    function readDir(path: string): Promise<string[]>;

    /**
     * Create a directory
     * @param path Path to create
     * @param options Options for creation
     *
     * @example
     * ```ts
     * await Viper.mkdir("path/to/dir", { recursive: true });
     * ```
     */
    function mkdir(
      path: string,
      options?: { recursive?: boolean },
    ): Promise<void>;

    /**
     * Remove a file or directory
     * @param path Path to remove
     * @param options Options for removal
     *
     * @example
     * ```ts
     * await Viper.remove("temp", { recursive: true });
     * ```
     */
    function remove(
      path: string,
      options?: { recursive?: boolean },
    ): Promise<void>;

    /**
     * Check if a path exists
     * @param path Path to check
     *
     * @example
     * ```ts
     * if (await Viper.exists("config.json")) {
     *   // ...
     * }
     * ```
     */
    function exists(path: string): Promise<boolean>;

    /**
     * Get file/directory stats
     * @param path Path to stat
     *
     * @example
     * ```ts
     * const stats = await Viper.stat("file.txt");
     * console.log(stats.size, stats.isFile);
     * ```
     */
    function stat(path: string): Promise<ViperFileStats>;

    // ========================================================================
    // Environment API
    // ========================================================================

    /**
     * Environment variable access
     */
    const env: {
      /**
       * Get an environment variable
       * @param key Variable name
       * @returns Variable value or undefined
       */
      get(key: string): string | undefined;

      /**
       * Set an environment variable
       * @param key Variable name
       * @param value Variable value
       */
      set(key: string, value: string): void;

      /**
       * Check if an environment variable exists
       * @param key Variable name
       */
      has(key: string): boolean;

      /**
       * Get all environment variables as an object
       */
      toObject(): Record<string, string>;
    };

    // ========================================================================
    // Process API
    // ========================================================================

    /**
     * Current process ID
     */
    const pid: number;

    /**
     * Get the current working directory
     */
    function cwd(): string;

    /**
     * Viper runtime version
     */
    const version: string;

    // ========================================================================
    // Spawn/Exec API
    // ========================================================================

    /**
     * Result from spawn() or exec()
     */
    interface SpawnResult {
      /**
       * Exit code of the process
       */
      exitCode: number;

      /**
       * Whether the process exited successfully (exit code 0)
       */
      success: boolean;

      /**
       * Standard output as Uint8Array (spawn) or string (exec)
       */
      stdout: Uint8Array | string;

      /**
       * Standard error as Uint8Array (spawn) or string (exec)
       */
      stderr: Uint8Array | string;

      /**
       * Get stdout as text (spawn only)
       */
      text?(): string;

      /**
       * Get trimmed stdout as string
       */
      toString(): string;
    }

    /**
     * Options for spawn()
     */
    interface SpawnOptions {
      /**
       * Working directory for the command
       */
      cwd?: string;

      /**
       * Whether to run through shell
       */
      shell?: boolean;

      /**
       * Environment variables
       */
      env?: Record<string, string>;
    }

    /**
     * Spawn a child process
     * @param command Command to execute
     * @param args Command arguments
     * @param options Spawn options
     * @returns Result with stdout/stderr as Uint8Array
     *
     * @example
     * ```ts
     * const result = Viper.spawn("node", ["--version"]);
     * console.log(result.text()); // "v20.0.0"
     * ```
     */
    function spawn(
      command: string,
      args?: string[],
      options?: SpawnOptions,
    ): SpawnResult;

    /**
     * Execute a shell command
     * @param command Shell command to execute
     * @returns Result with stdout/stderr as strings
     *
     * @example
     * ```ts
     * const result = Viper.exec("echo Hello");
     * console.log(result.stdout); // "Hello\n"
     * ```
     */
    function exec(command: string): SpawnResult;

    /**
     * Tagged template literal for shell commands (Bun-style)
     * Values are automatically escaped
     *
     * @example
     * ```ts
     * const name = "World";
     * const result = Viper.$`echo Hello ${name}`;
     * console.log(result.toString()); // "Hello World"
     * ```
     */
    function $(strings: TemplateStringsArray, ...values: any[]): SpawnResult;

    /**
     * Find the path to an executable
     * @param command Command name to find
     * @returns Full path to executable, or null if not found
     *
     * @example
     * ```ts
     * const nodePath = Viper.which("node");
     * // "/usr/local/bin/node"
     * ```
     */
    function which(command: string): string | null;

    /**
     * Sleep for a specified duration
     * @param ms Milliseconds to sleep
     * @returns Promise that resolves after the delay
     *
     * @example
     * ```ts
     * await Viper.sleep(1000); // Wait 1 second
     * ```
     */
    function sleep(ms: number): Promise<void>;
  }

  /**
   * ViperRouter class (also available as Viper.Router)
   */
  const ViperRouter: typeof Viper.Router;
}

// ============================================================================
// Module Augmentation for JSX
// ============================================================================

declare namespace JSX {
  interface Element {
    type: string | symbol;
    props: Record<string, any>;
    $$typeof: symbol;
  }

  interface IntrinsicElements {
    // HTML Elements
    a: HtmlProps;
    abbr: HtmlProps;
    address: HtmlProps;
    area: HtmlProps;
    article: HtmlProps;
    aside: HtmlProps;
    audio: HtmlProps;
    b: HtmlProps;
    base: HtmlProps;
    bdi: HtmlProps;
    bdo: HtmlProps;
    blockquote: HtmlProps;
    body: HtmlProps;
    br: HtmlProps;
    button: HtmlProps;
    canvas: HtmlProps;
    caption: HtmlProps;
    cite: HtmlProps;
    code: HtmlProps;
    col: HtmlProps;
    colgroup: HtmlProps;
    data: HtmlProps;
    datalist: HtmlProps;
    dd: HtmlProps;
    del: HtmlProps;
    details: HtmlProps;
    dfn: HtmlProps;
    dialog: HtmlProps;
    div: HtmlProps;
    dl: HtmlProps;
    dt: HtmlProps;
    em: HtmlProps;
    embed: HtmlProps;
    fieldset: HtmlProps;
    figcaption: HtmlProps;
    figure: HtmlProps;
    footer: HtmlProps;
    form: HtmlProps;
    h1: HtmlProps;
    h2: HtmlProps;
    h3: HtmlProps;
    h4: HtmlProps;
    h5: HtmlProps;
    h6: HtmlProps;
    head: HtmlProps;
    header: HtmlProps;
    hgroup: HtmlProps;
    hr: HtmlProps;
    html: HtmlProps;
    i: HtmlProps;
    iframe: HtmlProps;
    img: HtmlProps;
    input: HtmlProps;
    ins: HtmlProps;
    kbd: HtmlProps;
    label: HtmlProps;
    legend: HtmlProps;
    li: HtmlProps;
    link: HtmlProps;
    main: HtmlProps;
    map: HtmlProps;
    mark: HtmlProps;
    meta: HtmlProps;
    meter: HtmlProps;
    nav: HtmlProps;
    noscript: HtmlProps;
    object: HtmlProps;
    ol: HtmlProps;
    optgroup: HtmlProps;
    option: HtmlProps;
    output: HtmlProps;
    p: HtmlProps;
    param: HtmlProps;
    picture: HtmlProps;
    pre: HtmlProps;
    progress: HtmlProps;
    q: HtmlProps;
    rp: HtmlProps;
    rt: HtmlProps;
    ruby: HtmlProps;
    s: HtmlProps;
    samp: HtmlProps;
    script: HtmlProps;
    section: HtmlProps;
    select: HtmlProps;
    small: HtmlProps;
    source: HtmlProps;
    span: HtmlProps;
    strong: HtmlProps;
    style: HtmlProps;
    sub: HtmlProps;
    summary: HtmlProps;
    sup: HtmlProps;
    table: HtmlProps;
    tbody: HtmlProps;
    td: HtmlProps;
    textarea: HtmlProps;
    tfoot: HtmlProps;
    th: HtmlProps;
    thead: HtmlProps;
    time: HtmlProps;
    title: HtmlProps;
    tr: HtmlProps;
    track: HtmlProps;
    u: HtmlProps;
    ul: HtmlProps;
    var: HtmlProps;
    video: HtmlProps;
    wbr: HtmlProps;
  }

  interface HtmlProps {
    children?: any;
    className?: string;
    id?: string;
    style?: string | Record<string, string | number>;
    [key: string]: any;
  }

  interface ElementChildrenAttribute {
    children: {};
  }
}

export {};
