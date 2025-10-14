FROM scratch

COPY usr/lib/postgresql/* /
COPY usr/share/postgresql/*/extension/* /share/extension/
