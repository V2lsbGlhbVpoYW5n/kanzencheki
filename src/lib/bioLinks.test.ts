import { describe, expect, it } from "vitest";
import { bioParts, safeWebUrl } from "./bioLinks";

describe("person introduction links", () => {
  it("renders Markdown labels and plain web addresses while keeping punctuation", () => {
    expect(bioParts("看[官网](https://example.com/a)，也看 www.example.org。"))
      .toEqual([
        { text: "看" },
        { text: "官网", href: "https://example.com/a" },
        { text: "，也看 " },
        { text: "www.example.org", href: "https://www.example.org/" },
        { text: "。" },
      ]);
  });
  it("keeps unsafe or invalid link syntax as text", () => {
    expect(safeWebUrl("javascript:alert(1)")).toBeNull();
    expect(safeWebUrl("file:///tmp/a")).toBeNull();
    expect(bioParts("[点我](javascript:alert(1))"))
      .toEqual([{ text: "[点我](javascript:alert(1))" }]);
  });
});
