FROM node:22-alpine
WORKDIR /site
COPY . .
ENV PORT=8940 BIND=0.0.0.0 SITE_ROOT=/site
EXPOSE 8940
USER node
CMD ["node", "serve.mjs"]
