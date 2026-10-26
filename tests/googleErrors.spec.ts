import { describe, expect, it } from "vitest";
import { googleErrorHint } from "@/lib/googleErrors";

describe("googleErrorHint", () => {
  it("maps the failures a real login actually produces", () => {
    expect(googleErrorHint("token endpoint: invalid_grant")).toBe("error.invalidGrant");
    expect(googleErrorHint("授权被拒绝：access_denied")).toBe("error.accessDenied");
    expect(googleErrorHint("Google API error 403: Drive API has not been used in project 123 before")).toBe(
      "error.driveApiDisabled",
    );
    expect(googleErrorHint("Google API error 403: The user does not have sufficient permissions")).toBe(
      "error.insufficientScope",
    );
    expect(googleErrorHint("Google API error 403: insufficientPermissions")).toBe("error.insufficientScope");
    expect(googleErrorHint("Google API error 429: userRateLimitExceeded")).toBe("error.rateLimited");
    expect(googleErrorHint("Google API error 403: storageQuotaExceeded")).toBe("error.quotaExceeded");
    expect(googleErrorHint("授权被拒绝：redirect_uri_mismatch")).toBe("error.redirectMismatch");
    expect(googleErrorHint("token endpoint: unauthorized_client")).toBe("error.invalidClient");
    expect(googleErrorHint("Google API error 403: fileNotDownloadable")).toBe("error.notDownloadable");
  });

  it("returns null for unrelated failures so the raw message survives", () => {
    expect(googleErrorHint("network error: request timed out")).toBeNull();
    expect(googleErrorHint("文件超过 512 MB，请下载后再打开")).toBeNull();
  });
});

describe("expired access token", () => {
  it("maps a 401 from Drive to the token hint", () => {
    expect(googleErrorHint("Google API error 401: Invalid Credentials")).toBe("error.unauthorized");
  });
});
