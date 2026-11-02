import { describe, expect, it } from "vitest";

import { cn } from "./utils";

describe("cn", () => {
  it("resolve conflitos de classes do Tailwind", () => {
    expect(cn("px-2", "px-4")).toBe("px-4");
  });

  it("ignora valores falsos", () => {
    expect(cn("a", false, undefined, "b")).toBe("a b");
  });
});
