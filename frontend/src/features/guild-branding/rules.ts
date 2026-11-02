// Regras de branding do lado do cliente (espelho de `domain::branding`); o backend revalida tudo.
import { z } from "zod";

import { t } from "@/app/i18n";
import { SOCIAL_KEYS } from "@/lib/api/types";

export const IMAGE_MAX_BYTES = 2 * 1024 * 1024;
export const IMAGE_TYPES = ["image/png", "image/jpeg", "image/webp"] as const;

/** Erro de uma imagem escolhida, ou `null` se aceitável. */
export function imageError(file: { size: number; type: string }): string | null {
  if (!(IMAGE_TYPES as readonly string[]).includes(file.type)) return t.branding.imageType;
  if (file.size > IMAGE_MAX_BYTES) return t.branding.imageTooBig;
  return null;
}

export const HEX = /^#[0-9a-f]{6}$/i;

const optionalColor = z
  .string()
  .trim()
  .refine((v) => v === "" || HEX.test(v), "use uma cor no formato #rrggbb");

const social = z
  .string()
  .trim()
  .max(200, "máximo de 200 caracteres")
  .refine((v) => v === "" || v.startsWith("https://"), "use um link https://");

export const brandingSchema = z.object({
  primary_color: optionalColor,
  secondary_color: optionalColor,
  motto: z.string().trim().max(120, "máximo de 120 caracteres"),
  description_md: z.string().trim().max(10_000, "máximo de 10.000 caracteres"),
  public_page_enabled: z.boolean(),
  socials: z.object(Object.fromEntries(SOCIAL_KEYS.map((k) => [k, social])) as Record<
    (typeof SOCIAL_KEYS)[number],
    typeof social
  >),
});

export type BrandingForm = z.infer<typeof brandingSchema>;
