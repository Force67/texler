FROM texlive/texlive:latest

# Set working directory
WORKDIR /usr/src/app

# Install minimal required packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    inotify-tools \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
RUN python3 -m pip install --no-cache-dir flask python-dotenv watchdog

# Create simple main.py for now (we'll update it later)
RUN echo 'from flask import Flask, jsonify\napp = Flask(__name__)\n@app.route("/health")\ndef health():\n    return jsonify({"status": "ok"})\n@app.route("/")\ndef home():\n    return "LaTeX Service - Health check: /health"\nif __name__ == "__main__":\n    app.run(host="0.0.0.0", port=5000)' > main.py

# Expose port
EXPOSE 5000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:5000/health || exit 1

# Command to run the application
CMD ["python3", "main.py"]