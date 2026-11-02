import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { safeNext } from "@/app/host";
import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { FormError } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { post } from "@/lib/api/client";
import type { User } from "@/lib/api/types";
import { applyApiError } from "@/lib/forms";

import { AuthCard } from "./login-page";
import { meKey } from "./use-me";

const schema = z.object({
  display_name: z.string().trim().min(2, "mínimo de 2 caracteres").max(40, "máximo de 40 caracteres"),
  email: z.string().trim().email("e-mail inválido"),
  password: z.string().min(8, "mínimo de 8 caracteres").max(128, "máximo de 128 caracteres"),
});
type Form = z.infer<typeof schema>;

export function RegisterPage({ next }: { next?: string }) {
  const qc = useQueryClient();
  const navigate = useNavigate();
  const [formError, setFormError] = useState<string | null>(null);
  const form = useForm<Form>({ resolver: zodResolver(schema) });
  const errors = form.formState.errors;

  const register = useMutation({
    mutationFn: (data: Form) => post<User>("/auth/register", data),
    onSuccess: async () => {
      const target = safeNext(next);
      if (target) {
        window.location.href = target;
        return;
      }
      await qc.invalidateQueries({ queryKey: meKey });
      await navigate({ to: "/nova-guilda" });
    },
    onError: (e) => setFormError(applyApiError(e, form.setError, ["display_name", "email", "password"])),
  });

  return (
    <AuthCard title={t.auth.registerTitle}>
      <form className="flex flex-col gap-4" onSubmit={form.handleSubmit((d) => register.mutate(d))} noValidate>
        <FormError message={formError} />
        <Field label={t.auth.displayName} autoComplete="nickname" {...form.register("display_name")} error={errors.display_name?.message} />
        <Field label={t.auth.email} type="email" autoComplete="email" {...form.register("email")} error={errors.email?.message} />
        <Field label={t.auth.password} type="password" autoComplete="new-password" {...form.register("password")} error={errors.password?.message} />
        <Button type="submit" disabled={register.isPending}>
          {t.auth.submitRegister}
        </Button>
      </form>
      <p className="mt-6 text-center text-sm text-muted">
        {t.auth.hasAccount}{" "}
        <Link to="/entrar" search={next ? { next } : {}} className="text-primary hover:underline">
          {t.auth.loginTitle}
        </Link>
      </p>
    </AuthCard>
  );
}
