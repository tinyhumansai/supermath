# The supermath Python run service: the pinned solver environment plus the
# stdlib HTTP server that runs one function per fresh, limited subprocess.
#
#   docker build -t supermath-python docker/python
#
# The environment is installed from uv.lock at build time (`--frozen`: the lock
# is never re-resolved) and nothing is installed at run time.

FROM python:3.12-slim-trixie AS build
COPY --from=ghcr.io/astral-sh/uv:0.11.4 /uv /usr/local/bin/uv
# A few solvers (pynauty) ship only a source distribution for some platforms;
# the compiler stays in this stage and never reaches the runtime image.
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY pyproject.toml uv.lock ./
ENV UV_PROJECT_ENVIRONMENT=/opt/venv \
    UV_PYTHON_DOWNLOADS=never \
    UV_COMPILE_BYTECODE=1
RUN uv sync --frozen --no-install-project --no-dev

FROM python:3.12-slim-trixie
# Solver wheels link libgomp (OpenMP) and libstdc++; the slim base has neither
# libgomp, and nothing else is installed.
RUN apt-get update \
 && apt-get install -y --no-install-recommends libgomp1 \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin runner
COPY --from=build /opt/venv /opt/venv
WORKDIR /app
COPY pyproject.toml uv.lock solvers.toml server.py runner.py ./
ENV PATH=/opt/venv/bin:$PATH \
    PYTHONDONTWRITEBYTECODE=1 \
    PY_SERVICE_HOST=0.0.0.0 \
    PY_SERVICE_PORT=3160 \
    SUPERMATH_PROJECTS=/projects
USER runner
EXPOSE 3160
HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=6 \
  CMD ["/opt/venv/bin/python", "-c", "import urllib.request as u; u.urlopen('http://127.0.0.1:3160/health', timeout=4)"]
CMD ["/opt/venv/bin/python", "/app/server.py"]
