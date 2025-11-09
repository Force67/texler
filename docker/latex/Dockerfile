FROM texlive/texlive:latest

# Install minimal required packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-venv \
    inotify-tools \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create virtual environment in a different location (outside /app)
RUN python3 -m venv /opt/venv
RUN /opt/venv/bin/pip install --no-cache-dir flask flask-cors

# Set working directory
WORKDIR /app

# Create necessary directories
RUN mkdir -p /tmp/latex-compile /workspace

# Add virtual environment to PATH
ENV PATH="/opt/venv/bin:\$PATH"

# Expose port for compilation API
EXPOSE 8081

# Set up to entrypoint (use virtual environment python)
CMD ["python3", "server.py"]
