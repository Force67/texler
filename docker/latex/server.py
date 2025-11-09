from flask import Flask, request, jsonify, send_file
from flask_cors import CORS
import subprocess
import os
import uuid
import shutil
import json
import sys

app = Flask(__name__)
CORS(app)

COMPILE_DIR = '/tmp/latex-compile'

@app.route('/health', methods=['GET'])
def health():
    return jsonify({'status': 'ok'})

@app.route('/compile', methods=['POST'])
def compile_latex():
    try:
        data = request.get_json()

        # Create a unique directory for this compilation
        job_id = str(uuid.uuid4())
        work_dir = os.path.join(COMPILE_DIR, job_id)
        os.makedirs(work_dir, exist_ok=True)

        # Write all files
        for filename, content in data.get('files', {}).items():
            filepath = os.path.join(work_dir, filename)
            os.makedirs(os.path.dirname(filepath), exist_ok=True)
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)

        # Find the main file
        main_file = data.get('mainFile', 'main.tex')
        main_path = os.path.join(work_dir, main_file)

        if not os.path.exists(main_path):
            return jsonify({
                'success': False,
                'error': f'Main file {main_file} not found'
            }), 400

        # Run /usr/bin/pdflatex with proper flags
        result = subprocess.run(
            ['/usr/bin/pdflatex', '-interaction=nonstopmode', '-shell-escape', '-output-directory=.', main_file],
            cwd=work_dir,
            capture_output=True,
            text=True
        )

        # Read the PDF if compilation was successful
        pdf_path = os.path.join(work_dir, main_file.replace('.tex', '.pdf'))
        pdf_base64 = None

        if os.path.exists(pdf_path):
            with open(pdf_path, 'rb') as f:
                pdf_base64 = f.read().hex()

        # Parse the log for errors
        log_path = os.path.join(work_dir, main_file.replace('.tex', '.log'))
        log_content = None
        errors = []

        if os.path.exists(log_path):
            with open(log_path, 'r', encoding='utf-8', errors='ignore') as f:
                log_content = f.read()

            # Extract errors from log
            lines = log_content.split('\n')
            for i, line in enumerate(lines):
                if 'Error:' in line or 'Fatal error' in line or 'Emergency stop' in line:
                    # Get context around the error
                    context = lines[max(0, i-2):i+3]
                    errors.append({
                        'line': line,
                        'context': context
                    })

        # Clean up
        shutil.rmtree(work_dir)

        return jsonify({
            'success': result.returncode == 0,
            'output': result.stdout,
            'errors': result.stderr,
            'pdf': pdf_base64,
            'log': log_content,
            'parsedErrors': errors
        })

    except Exception as e:
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8081)