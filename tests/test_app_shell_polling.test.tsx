import { describe, expect, it } from "vitest";
import { shouldSuspendBackgroundPolling } from "../src/components/layout/AppShell";

describe("app shell polling", () => {
  it("keeps desktop runtime polling active even when document is hidden", () => {
    expect(shouldSuspendBackgroundPolling(true, true)).toBe(false);
    expect(shouldSuspendBackgroundPolling(true, false)).toBe(true);
    expect(shouldSuspendBackgroundPolling(false, false)).toBe(false);
  });
});