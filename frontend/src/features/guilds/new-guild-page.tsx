import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Navigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { BASE_DOMAIN, guildUrl, slugError, slugify } from "@/app/host";
import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { FormError } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { AuthCard } from "@/features/auth/login-page";
import { useMe } from "@/features/auth/use-me";
import { ApiError, get, post } from "@/lib/api/client";
import type { SlugCheck } from "@/lib/api/types";
import { applyApiError } from "@/lib/forms";

const schema = z.object({
  name: z.string().trim().min(3, "mínimo de 3 caracteres").max(48, "máximo de 48 caracteres"),
  slug: z
    .string()
    .trim()
    .toLowerCase()
    .superRefine((s, ctx) => {
      const err = slugError(s);
      if (err) ctx.addIssue({ code: "custom", message: err });
    }),
  tag: z.string().trim().max(8, "máximo de 8 caracteres"),
  nickname: z.string().trim().max(32, "máximo de 32 caracteres"),
});
type Form = z.infer<typeof schema>;

export function NewGuildPage() {
  const me = useMe();
  const [formError, setFormError] = useState<string | null>(null);
  const [slugTouched, setSlugTouched] = useState(false);
  const form = useForm<Form>({
    resolver: zodResolver(schema),
    defaultValues: { name: "", slug: "", tag: "", nickname: "" },
  });
  const errors = form.formState.errors;
  const name = form.watch("name");
  const slug = form.watch("slug");

  // O endereço acompanha o nome até o usuário editá-lo.
  useEffect(() => {
    if (!slugTouched) form.setValue("slug", slugify(name));
  }, [name, slugTouched, form]);

  const debouncedSlug = useDebounced(slug, 350);
  const slugCheck = useQuery({
    queryKey: ["slug-check", debouncedSlug],
    queryFn: () => get<SlugCheck>(`/guilds/slug-check?slug=${encodeURIComponent(debouncedSlug)}`),
    enabled: slugError(debouncedSlug) === null,
    staleTime: 10_000,
  });

  const create = useMutation({
    mutationFn: (data: Form) =>
      post<{ slug: string }>("/guilds", {
        name: data.name,
        slug: data.slug,
        tag: data.tag || null,
        nickname: data.nickname || null,
      }),
    onSuccess: (g) => {
      window.location.href = guildUrl(g.slug, "/painel");
    },
    onError: (e) => setFormError(applyApiError(e, form.setError, ["name", "slug", "tag", "nickname"])),
  });

  if (me.error instanceof ApiError && me.error.status === 401) {
    return <Navigate to="/entrar" search={{ next: window.location.href }} />;
  }

  const slugHint =
    slugCheck.data && debouncedSlug === slug ? (
      slugCheck.data.available ? (
        <span className="text-secondary">{t.guilds.slugAvailable}</span>
      ) : (
        <span className="text-danger">{slugCheck.data.reason}</span>
      )
    ) : null;

  return (
    <AuthCard title={t.guilds.newTitle}>
      <form className="flex flex-col gap-4" onSubmit={form.handleSubmit((d) => create.mutate(d))} noValidate>
        <FormError message={formError} />
        <Field label={t.guilds.name} {...form.register("name")} error={errors.name?.message} />
        <div className="flex flex-col gap-1.5">
          <Field
            label={t.guilds.slug}
            {...form.register("slug", { onChange: () => setSlugTouched(true) })}
            error={errors.slug?.message}
            hint={slugHint}
          />
          <p className="text-xs text-muted">
            {slug || "suaguilda"}.{BASE_DOMAIN}
          </p>
        </div>
        <Field label={t.guilds.tag} {...form.register("tag")} error={errors.tag?.message} />
        <Field label={t.guilds.nickname} placeholder={me.data?.user.display_name} {...form.register("nickname")} error={errors.nickname?.message} />
        <Button type="submit" disabled={create.isPending}>
          {t.guilds.submit}
        </Button>
      </form>
    </AuthCard>
  );
}

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}
