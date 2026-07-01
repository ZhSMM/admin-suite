/**
 * Thin wrapper around Tauri `invoke`. All API calls funnel through here so:
 *  - we have one place to handle error shape (`{code, message}`)
 *  - we can attach common arguments (e.g. session token)
 *  - tests can mock `api.invoke` without touching call sites
 */
import { invoke } from '@tauri-apps/api/tauri'

export interface ApiError {
  code: string
  message: string
}

export class ApiException extends Error {
  code: string
  constructor(payload: ApiError) {
    super(payload.message)
    this.code = payload.code
  }
}

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return (await invoke(cmd, args)) as T
  } catch (e: any) {
    // Tauri serialises our AppError as { code, message }.
    if (e && typeof e === 'object' && 'code' in e && 'message' in e) {
      throw new ApiException(e as ApiError)
    }
    throw new ApiException({ code: 'UNKNOWN', message: String(e) })
  }
}