FROM texlive/texlive:latest

# Install minimal required packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    inotify-tools \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
RUN python3 -m pip install --no-cache-dir flask python-dotenv watchdog

# Set working directory
WORKDIR /usr/src/app

# Create simple health check service for testing
RUN echo '#!/usr/bin/env python3\nfrom flask import Flask, jsonify\napp = Flask(__name__)\n@app.route("/health")\ndef health():\n    return jsonify({"status": "ok", "service": "simple-test"})\n@app.route("/")\ndef home():\n    return """\nTexler LaTeX Editor - Simple Test Service\n\nUse docker-compose up for the full LaTeX compilation service:\n  - Full LaTeX compilation API on port 8081\n  - PDF generation and error handling\n  - CORS support\n  \nHealth check: /health\n"""\nif __name__ == "__main__":\n    app.run(host="0.0.0.0", port=5000, debug=True)' > main.py

# Expose port
EXPOSE 5000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:5000/health || exit 1

# Command to run the application
CMD ["python3", "main.py"]