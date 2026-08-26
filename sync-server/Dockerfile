FROM node:22-alpine

WORKDIR /app
COPY --chown=node:node server.mjs ./server.mjs
COPY --chown=node:node public ./public
RUN mkdir -p /data && chown node:node /data

USER node
EXPOSE 18080
VOLUME ["/data"]
CMD ["node", "server.mjs"]
