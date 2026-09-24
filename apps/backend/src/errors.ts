/**
 * Every failure the backend can surface over HTTP, mirroring
 * `crates/job-hunter-relay/src/error.rs`'s `{code, message}` shape and
 * status mapping so existing clients (the Flutter app's `RelayApiException`)
 * keep working unchanged.
 */
const CODES = {
  EMAIL_TAKEN: 409,
  INVALID_CREDENTIALS: 401,
  INVALID_REFRESH_TOKEN: 401,
  UNAUTHORIZED: 401,
  DEVICE_NOT_FOUND: 404,
  INVALID_PAIRING_CODE: 400,
  RATE_LIMITED: 429,
  VALIDATION: 400,
  NOT_FOUND: 404,
  INTERNAL: 500,
} as const;

export type BackendErrorCode = keyof typeof CODES;

export class BackendError extends Error {
  readonly code: BackendErrorCode;
  readonly status: number;

  constructor(code: BackendErrorCode, message: string) {
    super(message);
    this.code = code;
    this.status = CODES[code];
  }

  toBody() {
    return { error: { code: this.code, message: this.message } };
  }
}

export const Errors = {
  emailTaken: () => new BackendError("EMAIL_TAKEN", "an account with that email already exists"),
  invalidCredentials: () => new BackendError("INVALID_CREDENTIALS", "invalid email or password"),
  invalidRefreshToken: () =>
    new BackendError("INVALID_REFRESH_TOKEN", "that session has expired or was signed out; please sign in again"),
  unauthorized: () => new BackendError("UNAUTHORIZED", "missing or invalid authorization"),
  deviceNotFound: () => new BackendError("DEVICE_NOT_FOUND", "device not found"),
  invalidPairingCode: () => new BackendError("INVALID_PAIRING_CODE", "that pairing code is invalid or has expired"),
  rateLimited: () => new BackendError("RATE_LIMITED", "too many attempts; please wait a moment and try again"),
  validation: (message: string) => new BackendError("VALIDATION", message),
  notFound: (message = "not found") => new BackendError("NOT_FOUND", message),
  internal: (message = "internal error") => new BackendError("INTERNAL", message),
};
