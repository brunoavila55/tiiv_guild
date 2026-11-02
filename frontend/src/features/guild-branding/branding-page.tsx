import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useForm, type Path } from "react-hook-form";

import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { Card, FormError, PageMessage } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { Markdown } from "@/components/ui/markdown";
import { useBrandingPreview } from "@/features/guild-panel/branding-preview";
import { PanelGate } from "@/features/guild-panel/panel-gate";
import { useTenant } from "@/features/guild-panel/use-tenant";
import { del, get, put, retryUnlessClientError, upload } from "@/lib/api/client";
import { SOCIAL_KEYS, type Branding, type ImageSlot } from "@/lib/api/types";
import { applyApiError } from "@/lib/forms";
import { cn } from "@/lib/utils";

import { brandingSchema, HEX, imageError, type BrandingForm } from "./rules";

const brandingKey = ["branding"] as const;

/** Imagem escolhida e ainda não enviada (ou marcada para remoção). */
interface PendingImage {
  file: File | null;
  url: string | null;
  remove: boolean;
  error: string | null;
}

const noPending: PendingImage = { file: null, url: null, remove: false, error: null };

export function BrandingPage() {
  return (
    <PanelGate>
      {(g) =>
        g.me.permissions.includes("guild.branding.edit") ? (
          <BrandingLoader />
        ) : (
          <Card className="mx-auto max-w-md text-center text-muted">{t.branding.noPermission}</Card>
        )
      }
    </PanelGate>
  );
}

function BrandingLoader() {
  const branding = useQuery({ queryKey: brandingKey, queryFn: () => get<Branding>("/branding"), retry: retryUnlessClientError });
  if (branding.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (!branding.data) return <PageMessage>{t.common.genericError}</PageMessage>;
  return <BrandingEditor saved={branding.data} />;
}

function toForm(b: Branding): BrandingForm {
  return {
    primary_color: b.primary_color ?? "",
    secondary_color: b.secondary_color ?? "",
    motto: b.motto ?? "",
    description_md: b.description_md ?? "",
    public_page_enabled: b.public_page_enabled,
    socials: Object.fromEntries(SOCIAL_KEYS.map((k) => [k, b.socials[k] ?? ""])) as BrandingForm["socials"],
  };
}

function BrandingEditor({ saved }: { saved: Branding }) {
  const qc = useQueryClient();
  const tenant = useTenant();
  const { setPreview } = useBrandingPreview();
  const [formError, setFormError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [images, setImages] = useState<Record<ImageSlot, PendingImage>>({ logo: noPending, banner: noPending });
  const form = useForm<BrandingForm>({ resolver: zodResolver(brandingSchema), defaultValues: toForm(saved) });
  const errors = form.formState.errors;
  const values = form.watch();

  const imageUrl = (slot: ImageSlot) => {
    const p = images[slot];
    if (p.remove) return null;
    return p.url ?? (slot === "logo" ? saved.logo_url : saved.banner_url);
  };
  const color = (v: string) => (HEX.test(v) ? v.toLowerCase() : null);
  const primary = color(values.primary_color);
  const secondary = color(values.secondary_color);
  const logoUrl = imageUrl("logo");

  // Preview ao vivo no layout do tenant; desfeito ao sair da página.
  useEffect(() => {
    setPreview({ primary_color: primary, secondary_color: secondary, logo_url: logoUrl });
  }, [primary, secondary, logoUrl, setPreview]);
  useEffect(() => () => setPreview(null), [setPreview]);

  // URLs locais das imagens escolhidas: liberadas ao trocar a imagem do slot e ao sair.
  const imagesRef = useRef(images);
  imagesRef.current = images;
  useEffect(
    () => () => {
      for (const p of Object.values(imagesRef.current)) if (p.url) URL.revokeObjectURL(p.url);
    },
    [],
  );
  const replaceImage = (slot: ImageSlot, next: PendingImage) => {
    const old = images[slot].url;
    if (old) URL.revokeObjectURL(old);
    setImages((prev) => ({ ...prev, [slot]: next }));
  };

  const pick = (slot: ImageSlot, file: File | undefined) => {
    if (!file) return;
    const error = imageError(file);
    replaceImage(slot, error ? { ...noPending, error } : { file, url: URL.createObjectURL(file), remove: false, error: null });
  };
  const clearImage = (slot: ImageSlot) => replaceImage(slot, { ...noPending, remove: true });
  const resetImages = () => {
    for (const p of Object.values(images)) if (p.url) URL.revokeObjectURL(p.url);
    setImages({ logo: noPending, banner: noPending });
  };

  const save = useMutation({
    mutationFn: async (data: BrandingForm) => {
      for (const slot of ["logo", "banner"] as const) {
        const p = images[slot];
        if (p.file) await upload<Branding>(`/branding/${slot}`, p.file);
        else if (p.remove) await del<Branding>(`/branding/${slot}`);
      }
      return put<Branding>("/branding", data);
    },
    onSuccess: async (b) => {
      resetImages();
      form.reset(toForm(b));
      qc.setQueryData(brandingKey, b);
      await Promise.all([qc.invalidateQueries({ queryKey: ["tenant"] }), qc.invalidateQueries({ queryKey: ["public"] })]);
      setFormError(null);
      setNotice(t.branding.saved);
    },
    onError: (e) => {
      const fields = ["primary_color", "secondary_color", "motto", "description_md", ...SOCIAL_KEYS.map((k) => `socials.${k}`)];
      setFormError(applyApiError(e, form.setError, fields as Path<BrandingForm>[]));
      setNotice(null);
    },
  });

  const discard = () => {
    form.reset(toForm(saved));
    resetImages();
    setFormError(null);
  };

  const dirty = form.formState.isDirty || Object.values(images).some((p) => p.file || p.remove);
  const name = tenant.data?.name ?? "";
  const bannerUrl = imageUrl("banner");
  const socials = SOCIAL_KEYS.filter((k) => values.socials[k]?.startsWith("https://"));

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">{t.branding.title}</h1>
      <div className="grid gap-6 lg:grid-cols-[1fr_380px]">
        <form className="flex flex-col gap-6" onSubmit={form.handleSubmit((d) => save.mutate(d))} noValidate>
          <FormError message={formError} />
          {notice && !dirty && <p className="rounded-lg border border-secondary/40 bg-secondary/10 px-3 py-2 text-sm text-secondary">{notice}</p>}

          <Card className="grid gap-6 sm:grid-cols-2">
            <ImagePicker label={t.branding.logo} slot="logo" url={logoUrl} error={images.logo.error} onPick={pick} onClear={clearImage} square />
            <ImagePicker label={t.branding.banner} slot="banner" url={bannerUrl} error={images.banner.error} onPick={pick} onClear={clearImage} />
          </Card>

          <Card className="grid gap-4 sm:grid-cols-2">
            <ColorField label={t.branding.primaryColor} name="primary_color" value={values.primary_color} error={errors.primary_color?.message}
              register={form.register("primary_color")} onPick={(v) => form.setValue("primary_color", v, { shouldDirty: true, shouldValidate: true })} />
            <ColorField label={t.branding.secondaryColor} name="secondary_color" value={values.secondary_color} error={errors.secondary_color?.message}
              register={form.register("secondary_color")} onPick={(v) => form.setValue("secondary_color", v, { shouldDirty: true, shouldValidate: true })} />
          </Card>

          <Card className="flex flex-col gap-4">
            <Field label={t.branding.motto} {...form.register("motto")} error={errors.motto?.message} />
            <div className="flex flex-col gap-1.5">
              <label htmlFor="description_md" className="text-sm font-medium">{t.branding.description}</label>
              <textarea id="description_md" rows={6} {...form.register("description_md")}
                className={cn("rounded-lg border border-border bg-surface-2 px-3 py-2 text-sm outline-none transition focus:border-primary", errors.description_md && "border-danger")} />
              {errors.description_md && <p className="text-xs text-danger">{errors.description_md.message}</p>}
            </div>
            <label className="flex items-center gap-3 text-sm">
              <input type="checkbox" className="size-4 accent-[var(--color-primary)]" {...form.register("public_page_enabled")} />
              {t.branding.publicPage}
            </label>
          </Card>

          <Card className="flex flex-col gap-4">
            <h2 className="font-semibold">{t.branding.socials}</h2>
            <div className="grid gap-4 sm:grid-cols-2">
              {SOCIAL_KEYS.map((k) => (
                <Field key={k} label={t.branding.socialLabels[k]} placeholder="https://" {...form.register(`socials.${k}`)} error={errors.socials?.[k]?.message} />
              ))}
            </div>
          </Card>

          <div className="flex justify-end gap-3">
            <Button type="button" variant="ghost" onClick={discard} disabled={!dirty || save.isPending}>{t.branding.discard}</Button>
            <Button type="submit" disabled={!dirty || save.isPending}>{t.common.save}</Button>
          </div>
        </form>

        <aside className="flex flex-col gap-3 lg:sticky lg:top-6 lg:self-start">
          <p className="text-sm font-medium">{t.branding.preview}</p>
          <Card className="overflow-hidden p-0">
            <div className="h-28 bg-cover bg-center"
              style={bannerUrl ? { backgroundImage: `url(${bannerUrl})` } : { background: "linear-gradient(135deg, var(--color-primary), var(--color-secondary))" }} />
            <div className="flex flex-col gap-3 p-5">
              <div className="-mt-12 flex items-end gap-3">
                {logoUrl ? (
                  <img src={logoUrl} alt="" className="size-16 rounded-xl border-4 border-surface object-cover" />
                ) : (
                  <span className="grid size-16 place-items-center rounded-xl border-4 border-surface bg-primary text-lg font-bold text-white">{name.slice(0, 2).toUpperCase()}</span>
                )}
              </div>
              <div>
                <p className="text-lg font-semibold">{name}</p>
                {values.motto.trim() && <p className="text-sm text-primary">{values.motto.trim()}</p>}
              </div>
              {values.description_md.trim() && <Markdown source={values.description_md} className="line-clamp-6 gap-2 text-sm text-muted" />}
              {socials.length > 0 && (
                <div className="flex flex-wrap gap-2">
                  {socials.map((k) => (
                    <span key={k} className="rounded-md bg-secondary/15 px-2 py-1 text-xs text-secondary">{t.branding.socialLabels[k]}</span>
                  ))}
                </div>
              )}
            </div>
          </Card>
          <p className="text-xs text-muted">{t.branding.previewHint}</p>
        </aside>
      </div>
    </div>
  );
}

function ImagePicker({ label, slot, url, error, onPick, onClear, square }: {
  label: string;
  slot: ImageSlot;
  url: string | null;
  error: string | null;
  onPick: (slot: ImageSlot, file: File | undefined) => void;
  onClear: (slot: ImageSlot) => void;
  square?: boolean;
}) {
  const id = `image-${slot}`;
  return (
    <div className="flex flex-col gap-2">
      <span className="text-sm font-medium">{label}</span>
      <div className={cn("grid place-items-center overflow-hidden rounded-xl border border-dashed border-border bg-surface-2", square ? "size-28" : "h-28 w-full")}>
        {url ? <img src={url} alt="" className="size-full object-cover" /> : <span className="text-xs text-muted">—</span>}
      </div>
      <div className="flex items-center gap-2">
        <label htmlFor={id} className="inline-flex h-9 cursor-pointer items-center rounded-lg border border-border px-3 text-sm hover:bg-surface-2">
          {t.branding.chooseImage}
        </label>
        <input id={id} type="file" accept="image/png,image/jpeg,image/webp" className="sr-only"
          onChange={(e) => { onPick(slot, e.target.files?.[0]); e.target.value = ""; }} />
        {url && <Button type="button" variant="ghost" className="h-9" onClick={() => onClear(slot)}>{t.branding.removeImage}</Button>}
      </div>
      {error ? <p className="text-xs text-danger">{error}</p> : <p className="text-xs text-muted">{t.branding.imageHint}</p>}
    </div>
  );
}

function ColorField({ label, name, value, error, register, onPick }: {
  label: string;
  name: string;
  value: string;
  error?: string;
  register: ReturnType<ReturnType<typeof useForm<BrandingForm>>["register"]>;
  onPick: (v: string) => void;
}) {
  return (
    <div className="flex items-end gap-3">
      <input type="color" aria-label={label} value={HEX.test(value) ? value : "#7c6cf2"} onChange={(e) => onPick(e.target.value)}
        className="h-10 w-12 cursor-pointer rounded-lg border border-border bg-surface-2 p-1" />
      <div className="flex-1">
        <Field label={label} id={name} placeholder="#rrggbb" {...register} error={error} />
      </div>
    </div>
  );
}
