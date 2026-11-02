// Uma árvore de rotas por tipo de host: o subdomínio decide qual app está rodando.
import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  type AnyRouter,
} from "@tanstack/react-router";

import { PageMessage } from "@/components/ui/card";
import { AdminGuildsPage } from "@/features/admin/admin-guilds-page";
import { LoginPage } from "@/features/auth/login-page";
import { RegisterPage } from "@/features/auth/register-page";
import { PanelHome } from "@/features/guild-panel/panel-home";
import { TenantLayout } from "@/features/guild-panel/tenant-layout";
import { PublicPage } from "@/features/guild-public/public-page";
import { AppLayout } from "@/features/guilds/app-layout";
import { MyGuildsPage } from "@/features/guilds/my-guilds-page";
import { NewGuildPage } from "@/features/guilds/new-guild-page";
import { LandingPage } from "@/features/landing/landing-page";
import { MembersPage } from "@/features/members/members-page";

import type { HostKind } from "./host";
import { t } from "./i18n";

const notFound = () => <PageMessage>{t.common.notFound}</PageMessage>;

const nextSearch = (search: Record<string, unknown>): { next?: string } =>
  typeof search.next === "string" ? { next: search.next } : {};

function rootRouter() {
  const root = createRootRoute({ component: Outlet, notFoundComponent: notFound });
  const landing = createRoute({ getParentRoute: () => root, path: "/", component: LandingPage });
  return createRouter({ routeTree: root.addChildren([landing]) });
}

function appRouter() {
  const root = createRootRoute({ component: AppLayout, notFoundComponent: notFound });
  const home = createRoute({ getParentRoute: () => root, path: "/", component: MyGuildsPage });
  const login = createRoute({
    getParentRoute: () => root,
    path: "/entrar",
    validateSearch: nextSearch,
    component: function Login() {
      return <LoginPage next={login.useSearch().next} />;
    },
  });
  const register = createRoute({
    getParentRoute: () => root,
    path: "/cadastro",
    validateSearch: nextSearch,
    component: function Register() {
      return <RegisterPage next={register.useSearch().next} />;
    },
  });
  const newGuild = createRoute({ getParentRoute: () => root, path: "/nova-guilda", component: NewGuildPage });
  return createRouter({ routeTree: root.addChildren([home, login, register, newGuild]) });
}

function adminRouter() {
  const root = createRootRoute({ component: Outlet, notFoundComponent: notFound });
  const home = createRoute({ getParentRoute: () => root, path: "/", component: AdminGuildsPage });
  return createRouter({ routeTree: root.addChildren([home]) });
}

function tenantRouter() {
  const root = createRootRoute({ component: TenantLayout, notFoundComponent: notFound });
  const publicPage = createRoute({ getParentRoute: () => root, path: "/", component: PublicPage });
  const panel = createRoute({ getParentRoute: () => root, path: "/painel", component: PanelHome });
  const members = createRoute({ getParentRoute: () => root, path: "/painel/membros", component: MembersPage });
  return createRouter({ routeTree: root.addChildren([publicPage, panel, members]) });
}

function unknownRouter() {
  const root = createRootRoute({ component: notFound });
  return createRouter({ routeTree: root });
}

export function routerFor(host: HostKind): AnyRouter {
  switch (host.kind) {
    case "root":
      return rootRouter();
    case "app":
      return appRouter();
    case "admin":
      return adminRouter();
    case "tenant":
      return tenantRouter();
    case "unknown":
      return unknownRouter();
  }
}
