FROM node:26-alpine AS build
WORKDIR /app
RUN npm install -g pnpm@12
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
# Domínio base dos tenants, embutido no bundle.
ARG VITE_BASE_DOMAIN=tiiv.com.br
ENV VITE_BASE_DOMAIN=$VITE_BASE_DOMAIN
RUN pnpm build

FROM nginx:1.27-alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
