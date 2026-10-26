export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}
let csrf = "";
let sessionExpired: (() => void) | undefined;
export function setSessionExpiredHandler(handler?: () => void) {
  sessionExpired = handler;
}
export function setCsrf(value: string) {
  csrf = value;
}
export interface RequestOptions {
  method?: string;
  body?: unknown;
  signal?: AbortSignal;
}
export async function api<T>(
  path: string,
  { method = "GET", body, signal }: RequestOptions = {},
): Promise<T> {
  const requestCsrf = csrf;
  const headers: Record<string, string> = { "x-binbin-csrf": requestCsrf };
  let payload: BodyInit | undefined;
  if (body !== undefined) {
    if (body instanceof Blob) payload = body;
    else {
      payload = JSON.stringify(body);
      headers["content-type"] = "application/json";
    }
  }
  const response = await fetch(path, {
    method,
    headers,
    body: payload,
    signal,
  });
  if (
    response.status === 401 &&
    csrf === requestCsrf &&
    path !== "/api/login" &&
    !path.startsWith("/api/public/")
  )
    sessionExpired?.();
  const value = await response.json().catch(() => {
    throw new ApiError(
      `Request failed (${response.status}); retry or contact the administrator.`,
      response.status,
    );
  });
  if (!response.ok)
    throw new ApiError(
      value.error || `Request failed (${response.status})`,
      response.status,
    );
  return value as T;
}
export function errorMessage(error: unknown) {
  return error instanceof Error
    ? error.message
    : "Something went wrong. Try again.";
}
