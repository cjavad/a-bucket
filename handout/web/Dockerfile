FROM nginx:latest
WORKDIR /app

RUN apt update && apt install -y curl jq openssl file

COPY nginx.conf /etc/nginx/nginx.conf
COPY src public
COPY data data
COPY scripts scripts

ENTRYPOINT ["/app/scripts/entrypoint.sh"]