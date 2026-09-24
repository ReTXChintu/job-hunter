# Multi-stage build for the Job Hunter backend. Produces a small runtime
# image with just the compiled JS; MongoDB Atlas is external, so there is
# no local database volume to mount.

FROM node:20-bookworm-slim AS build
WORKDIR /build
RUN corepack enable
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/backend/package.json apps/backend/package.json
COPY packages/config/package.json packages/config/package.json
RUN pnpm install --filter @job-hunter/backend... --frozen-lockfile
COPY packages/config packages/config
COPY apps/backend apps/backend
RUN pnpm --filter @job-hunter/backend build

FROM node:20-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
COPY --from=build /build/apps/backend/dist ./dist
COPY --from=build /build/apps/backend/node_modules ./node_modules
COPY --from=build /build/apps/backend/package.json ./package.json
ENV JOB_HUNTER_BACKEND_PORT=8788
EXPOSE 8788
CMD ["node", "dist/index.js"]
