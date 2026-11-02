import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { useState, type ReactNode } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { safeNext } from "@/app/host";
import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { Card, FormError } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { post } from "@/lib/api/client";
import type { User } from "@/lib/api/types";
import { applyApiError } from "@/lib/forms";

import { meKey } from "./use-me";

const schema = z.object({
  email: z.string().trim().email("e-mail inválido"),
  password: z.string().min(1, "informe a senha"),
});
type Form = z.infer<typeof schema>;

export function LoginPage({ next }: { next?: string }) {
  const qc = useQueryClient();
  const navigate = useNavigate();
  const [formError, setFormError] = useState<string | null>(null);
  const form = useForm<Form>({ resolver: zodResolver(schema) });

  const login = useMutation({
    mutationFn: (data: Form) => post<User>("/auth/login", data),
    onSuccess: async () => {
      const target = safeNext(next);
      if (target) {
        window.location.href = target;
        return;
      }
      await qc.invalidateQueries({ queryKey: meKey });
      await navigate({ to: "/" });
    },
    onError: (e) => setFormError(applyApiError(e, form.setError, ["email", "password"])),
  });

  return (
    <AuthCard title={t.auth.loginTitle}>
      <form className="flex flex-col gap-4" onSubmit={form.handleSubmit((d) => login.mutate(d))} noValidate>
        <FormError message={formError} />
        <Field label={t.auth.email} type="email" autoComplete="email" {...form.register("email")} error={form.formState.errors.email?.message} />
        <Field label={t.auth.password} type="password" autoComplete="current-password" {...form.register("password")} error={form.formState.errors.password?.message} />
        <Button type="submit" disabled={login.isPending}>
          {t.auth.submitLogin}
        </Button>
      </form>
      <p className="mt-6 text-center text-sm text-muted">
        {t.auth.noAccount}{" "}
        <Link to="/cadastro" search={next ? { next } : {}} className="text-primary hover:underline">
          {t.auth.registerTitle}
        </Link>
      </p>
    </AuthCard>
  );
}

export function AuthCard({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mx-auto w-full max-w-sm py-16">
      <Card>
        <h1 className="mb-6 text-xl font-semibold">{title}</h1>
        {children}
      </Card>
    </div>
  );
}
