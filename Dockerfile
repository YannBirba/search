# Build stage for frontend
FROM node:22-alpine AS frontend
WORKDIR /app/client

# Install pnpm
RUN npm install -g pnpm

# Copy frontend files
COPY client/package.json client/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY client/ .
RUN pnpm build

# Build stage for backend
FROM rust:1.76-slim AS backend
WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy backend files
COPY server/ .

# Build backend
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim
WORKDIR /app

# Install required runtime dependencies
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy built artifacts
COPY --from=frontend /app/client/dist ./dist
COPY --from=backend /app/target/release/search ./

# Set environment variables
ENV REDIS_URL=redis://redis:6379

# Expose port
EXPOSE 3000

# Run the application
CMD ["./search"]
