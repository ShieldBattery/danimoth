// This implementation is based on type-fest's Opaque, just want to avoid needing the dep
declare const tag: unique symbol
declare type Tagged<Token> = {
  readonly [tag]: Token
}

/** A Discord Social SDK client created by `createClient`. */
export type DiscordClient = Tagged<'DiscordClient'>

/** Returns the version of this module, so a caller can tell which build of it was loaded. */
export function version(): string

/**
 * Loads the Discord Social SDK from the `discord_partner_sdk.dll` at `dllPath`. Every other
 * function except `version` throws until this succeeds. Once the SDK is loaded, later calls do
 * nothing; it stays loaded until the process exits.
 *
 * The module is written against SDK 1.10; check `sdkVersion` before relying on another version.
 */
export function loadSdk(dllPath: string): void

/** The version of the loaded SDK. */
export interface SdkVersion {
  major: number
  minor: number
  patch: number
  hash: string
}

/** Returns the version of the loaded SDK. */
export function sdkVersion(): SdkVersion

/**
 * Runs the SDK's pending callbacks: client callbacks and the completion of pending calls happen
 * only while this runs. Call it periodically (e.g. every 100ms) for as long as any client exists.
 */
export function runCallbacks(): void

export type ClientStatus =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'ready'
  | 'reconnecting'
  | 'disconnecting'
  | 'httpWait'
  /** A status this module doesn't know about, from a newer SDK. */
  | 'unknown'

export type ClientError =
  | 'none'
  | 'connectionFailed'
  | 'unexpectedClose'
  | 'connectionCanceled'
  /** An error this module doesn't know about, from a newer SDK. */
  | 'unknown'

export type LogSeverity = 'verbose' | 'info' | 'warning' | 'error'

/**
 * Callbacks for a client's events. They're called asynchronously on the JS thread, after the
 * `runCallbacks` call that received them.
 */
export interface ClientOptions {
  /** Called with each message the SDK logs at `logSeverity` or above. */
  onLog?: (message: string, severity: LogSeverity | 'unknown') => void
  /** The lowest severity passed to `onLog`. Defaults to 'info'. */
  logSeverity?: LogSeverity
  /**
   * Called when the client's connection status changes. `errorDetail` is a code that further
   * describes `error`. The client is only usable once the status is 'ready'.
   */
  onStatusChanged?: (status: ClientStatus, error: ClientError, errorDetail: number) => void
  /**
   * Called once when the token last passed to `updateToken` is about to expire or has expired.
   * Get a new token and pass it to `updateToken`. Not called for a token that was expired or
   * revoked when connecting; the client disconnects instead.
   */
  onTokenExpiring?: () => void
  /**
   * Called when the user accepts an invite to, or asks to join, another user's activity in the
   * Discord client, with the `joinSecret` of that activity. If the Discord client launched the
   * app for this (see `registerLaunchCommand`), it's called once the app connects.
   */
  onActivityJoin?: (joinSecret: string) => void
}

/**
 * Creates a client. It does nothing until it's given a token (`updateToken`) and connected
 * (`connect`), or given an application ID (`setApplicationId`) to set rich presence through the
 * Discord desktop client without connecting.
 *
 * The client keeps its callbacks until it's destroyed, and the callbacks often reference the
 * client, so it isn't garbage collected on its own: call `destroyClient` when done with it.
 */
export function createClient(options: ClientOptions): DiscordClient

/**
 * Drops the client: it disconnects, stops calling its callbacks, and rejects its pending calls.
 * Any later call with it throws.
 */
export function destroyClient(client: DiscordClient): void

/**
 * Sets the application to use when not connected, for setting rich presence through the Discord
 * desktop client. Connecting sets it from the token instead.
 */
export function setApplicationId(client: DiscordClient, applicationId: string): void

/**
 * Sets the OAuth2 access token (a Bearer token) the client connects with. If the client is
 * connected, it may reconnect with the new token. Resolves once the token is stored; wait for that
 * before calling `connect`.
 */
export function updateToken(client: DiscordClient, token: string): Promise<void>

/**
 * Connects the client to Discord. Progress is reported to `onStatusChanged`; the client is ready
 * to use once the status is 'ready'.
 */
export function connect(client: DiscordClient): void

/** Disconnects the client. Progress is reported to `onStatusChanged`. */
export function disconnect(client: DiscordClient): void

/** Returns the client's current connection status. */
export function getStatus(client: DiscordClient): ClientStatus

export type ActivityType = 'playing' | 'streaming' | 'listening' | 'watching' | 'competing'

export type ActivityGamePlatform =
  'desktop' | 'xbox' | 'samsung' | 'ios' | 'android' | 'embedded' | 'ps4' | 'ps5'

/**
 * A rich presence activity. The SDK validates the fields when it's set: strings noted as 2-128
 * characters must be that long if given. The activity's name and application come from the
 * Discord application and can't be set.
 */
export interface Activity {
  /** Defaults to 'playing'. */
  type?: ActivityType
  /** Which field the user's status line in Discord shows. Defaults to the application's name. */
  statusDisplayType?: 'name' | 'state' | 'details'
  /** What the user is doing, e.g. the game mode. 2-128 characters. */
  details?: string
  /** The state of the user's party, e.g. "In a lobby". 2-128 characters. */
  state?: string
  /**
   * In milliseconds since the Unix epoch. With `start`, Discord shows how long the activity has
   * been going; with `end`, how long it has left.
   */
  timestamps?: { start?: number; end?: number }
  /**
   * Images and their tooltips. An image is the key of an art asset uploaded to the Discord
   * application, or a URL. Tooltips are 2-128 characters.
   */
  assets?: {
    largeImage?: string
    largeText?: string
    smallImage?: string
    smallText?: string
  }
  /**
   * The group the user is in, shown as "(currentSize of maxSize)" after `state`. Everyone in the
   * party should use the same `id` (2-128 characters). A `maxSize` of 0 shows no maximum.
   * `privacy` defaults to 'private'.
   */
  party?: {
    id: string
    currentSize: number
    maxSize: number
    privacy?: 'public' | 'private'
  }
  /**
   * Makes the activity joinable from Discord (with a `party` that has room) and is passed to
   * `onActivityJoin` of whoever joins. 2-128 characters.
   */
  joinSecret?: string
  /** The platforms the activity can be joined from. */
  supportedPlatforms?: ActivityGamePlatform[]
  /** Up to 2 buttons that open a URL. They aren't shown to the user themselves. */
  buttons?: Array<{ label: string; url: string }>
}

export type ResultErrorType =
  | 'none'
  | 'networkError'
  | 'httpError'
  | 'clientNotReady'
  | 'disabled'
  | 'clientDestroyed'
  | 'validationError'
  | 'aborted'
  | 'authorizationFailed'
  | 'rpcError'
  /** An error type this module doesn't know about, from a newer SDK. */
  | 'unknown'

/** The error a call's promise rejects with when the SDK reports a failure. */
export interface DiscordCallError extends Error {
  errorType: ResultErrorType
  /** A code that further describes the error, e.g. Discord's error code for an HTTP error. */
  errorCode: number
  /** Whether the call may succeed if it's tried again. */
  retryable: boolean
  /** How long to wait before trying again, in seconds. */
  retryAfter: number
}

/**
 * Sets the user's rich presence. Resolves once Discord accepted it, or rejects with a
 * `DiscordCallError`.
 */
export function updateRichPresence(client: DiscordClient, activity: Activity): Promise<void>

/** Clears the user's rich presence. */
export function clearRichPresence(client: DiscordClient): void

/**
 * Registers the command the Discord client runs to launch the app when the user joins an activity
 * from Discord while the app isn't running. Registration is per machine; do it on every start.
 * `command` is an executable path plus any arguments, e.g. `"C:\path\app.exe" app-scheme://`; an
 * empty string registers the running executable. Returns whether it was registered.
 */
export function registerLaunchCommand(
  client: DiscordClient,
  applicationId: string,
  command: string,
): boolean
