/**
 * Turns the raw Google OAuth/Drive error strings into an actionable sentence,
 * keeping the original text appended so nothing is hidden during debugging.
 */
const HINTS: readonly [RegExp, string][] = [
  [/invalid_grant/i, "error.invalidGrant"],
  [/access_denied|access blocked|has not completed the Google verification/i, "error.accessDenied"],
  [/accessNotConfigured|has not been used in project|is disabled/i, "error.driveApiDisabled"],
  [/insufficientPermissions|insufficientFilePermissions|does not have sufficient permissions/i, "error.insufficientScope"],
  [/rateLimitExceeded|userRateLimitExceeded/i, "error.rateLimited"],
  [/quotaExceeded|storageQuotaExceeded/i, "error.quotaExceeded"],
  [/redirect_uri_mismatch/i, "error.redirectMismatch"],
  [/invalid_client|unauthorized_client|deleted_client/i, "error.invalidClient"],
  [/fileNotDownloadable|cannotDownload/i, "error.notDownloadable"],
];

export function googleErrorHint(raw: string): string | null {
  for (const [pattern, key] of HINTS) {
    if (pattern.test(raw)) return key;
  }
  return null;
}
