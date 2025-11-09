import json
import urllib.request

# Test with the same default LaTeX content
latex_code = r"""\documentclass{article}
\\usepackage[utf8]{inputenc}
\\usepackage{amsmath}
\\usepackage{graphicx}
\\usepackage{geometry}
\\geometry{a4paper, margin=1in}

\\title{My LaTeX Document}
\\author{Your Name}
\\date{\\today}

\\begin{document}

\\maketitle

\\section{Introduction}

Welcome to Texler! This is a collaborative LaTeX editor with live preview.

\\section{Mathematics}

Here's some inline math: $E = mc^2$

And here's a displayed equation:
\\begin{equation}
    \\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}
\\end{equation}

\\end{document}"""

data = {
    'files': {
        'document.tex': latex_code
    },
    'mainFile': 'document.tex'
}

try:
    req = urllib.request.Request(
        'http://localhost:8081/compile',
        data=json.dumps(data).encode(),
        headers={'Content-Type': 'application/json'}
    )
    
    with urllib.request.urlopen(req) as response:
        result = json.loads(response.read().decode())
        
        print(f"Success: {result.get('success')}")
        print(f"Has PDF: {'pdf' in result and result['pdf'] is not None}")
        
        if result.get('success') and result.get('pdf'):
            pdf_hex = result.get('pdf', '')
            print(f"PDF hex length: {len(pdf_hex)}")
            
            # Check if it starts with PDF signature
            if pdf_hex.lower().startswith('25504446'):
                print("✓ PDF signature (hex) looks correct")
            else:
                print(f"✗ PDF signature wrong: {pdf_hex[:8]}")
            
            # Save test file
            import binascii
            try:
                pdf_bytes = binascii.unhexlify(pdf_hex)
                with open('/tmp/test_output.pdf', 'wb') as f:
                    f.write(pdf_bytes)
                print(f"✓ Test PDF saved to /tmp/test_output.pdf ({len(pdf_bytes)} bytes)")
                
                # Check PDF header
                if pdf_bytes.startswith(b'%PDF'):
                    print("✓ PDF binary signature correct")
                else:
                    print(f"✗ PDF binary signature wrong: {pdf_bytes[:10]}")
            except Exception as e:
                print(f"✗ Error converting hex: {e}")
        else:
            print(f"✗ No PDF in response")
            if 'parsedErrors' in result:
                print(f"Parsed errors: {result['parsedErrors']}")
            
except Exception as e:
    print(f"✗ Request failed: {e}")
