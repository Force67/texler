import json
import urllib.request
import binascii

latex_code = r"""\documentclass{article}
\begin{document}
Hello World!
\end{document}"""

data = {
    'files': {
        'test.tex': latex_code
    },
    'mainFile': 'test.tex'
}

req = urllib.request.Request(
    'http://localhost:8081/compile',
    data=json.dumps(data).encode(),
    headers={'Content-Type': 'application/json'}
)

with urllib.request.urlopen(req) as response:
    result = json.loads(response.read().decode())
    
    if result.get('success'):
        print(f"Success: {result['success']}")
        pdf_hex = result.get('pdf')
        if pdf_hex:
            print(f"PDF hex length: {len(pdf_hex)}")
            # Try to decode first few bytes
            try:
                first_bytes = binascii.unhexlify(pdf_hex[:20])
                print(f"First 10 bytes: {first_bytes}")
                # Check for PDF signature
                if first_bytes.startswith(b'%PDF'):
                    print("✓ Valid PDF signature detected")
                else:
                    print("✗ Invalid PDF signature")
            except Exception as e:
                print(f"Error decoding hex: {e}")
        else:
            print("No PDF data returned")
    else:
        print(f"Compilation failed")
        print(f"Errors: {result.get('errors')}")
        print(f"Output: {result.get('output')}")
