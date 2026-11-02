import { createContext, useContext } from "react";

/** Visual ainda não salvo, aplicado ao layout do tenant enquanto o usuário edita. */
export interface BrandingPreview {
  primary_color?: string | null;
  secondary_color?: string | null;
  logo_url?: string | null;
}

export const BrandingPreviewContext = createContext<{
  preview: BrandingPreview | null;
  setPreview: (p: BrandingPreview | null) => void;
}>({ preview: null, setPreview: () => {} });

export const useBrandingPreview = () => useContext(BrandingPreviewContext);
