#!/usr/bin/env bash
# Deploy the teLLeM site container. Runs ON the box, same pattern as ajd3v-site.
# Prerequisite the owner does once: DNS records for tellem.alanj.dev (canonical)
# and tellem.gainful.work (301s to canonical) pointing at this host. Traefik gets the cert on first request.
set -euo pipefail
cd /home/deploy/tellem-site

docker build -q -t tellem-site -f deploy/Dockerfile .
docker rm -f tellem-site 2>/dev/null || true

HOST_RULE='Host(`tellem.alanj.dev`) || Host(`tellem.gainful.work`)'
docker run -d --name tellem-site --restart unless-stopped --network coolify \
  -l traefik.enable=true \
  -l traefik.docker.network=coolify \
  -l "traefik.http.routers.tellem-http.entryPoints=http" \
  -l "traefik.http.routers.tellem-http.rule=$HOST_RULE" \
  -l "traefik.http.routers.tellem-http.middlewares=redirect-to-https" \
  -l "traefik.http.routers.tellem-https.entryPoints=https" \
  -l "traefik.http.routers.tellem-https.rule=$HOST_RULE" \
  -l "traefik.http.routers.tellem-https.middlewares=gzip" \
  -l "traefik.http.routers.tellem-https.tls=true" \
  -l "traefik.http.routers.tellem-https.tls.certresolver=letsencrypt" \
  -l "traefik.http.services.tellem-site.loadbalancer.server.port=8940" \
  tellem-site

sleep 2
docker exec tellem-site wget -qO- http://127.0.0.1:8940/healthz && echo
docker logs tellem-site --tail 2
