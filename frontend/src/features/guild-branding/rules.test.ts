import { describe, expect, it } from "vitest";

import { brandingSchema, IMAGE_MAX_BYTES, imageError } from "./rules";

const empty = {
  primary_color: "",
  secondary_color: "",
  motto: "",
  description_md: "",
  public_page_enabled: false,
  socials: { discord: "", website: "", youtube: "", twitch: "", instagram: "", x: "", tiktok: "", facebook: "" },
};

describe("imageError", () => {
  it("aceita png/jpg/webp até 2 MB", () => {
    expect(imageError({ type: "image/png", size: 10 })).toBeNull();
    expect(imageError({ type: "image/webp", size: IMAGE_MAX_BYTES })).toBeNull();
    expect(imageError({ type: "image/svg+xml", size: 10 })).not.toBeNull();
    expect(imageError({ type: "image/jpeg", size: IMAGE_MAX_BYTES + 1 })).not.toBeNull();
  });
});

describe("brandingSchema", () => {
  it("aceita campos vazios", () => {
    expect(brandingSchema.safeParse(empty).success).toBe(true);
  });

  it("rejeita cor e link inválidos", () => {
    const r = brandingSchema.safeParse({
      ...empty,
      primary_color: "laranja",
      socials: { ...empty.socials, x: "http://x.com/a" },
    });
    expect(r.success).toBe(false);
    const paths = r.error?.issues.map((i) => i.path.join(".")) ?? [];
    expect(paths).toEqual(expect.arrayContaining(["primary_color", "socials.x"]));
  });
});
