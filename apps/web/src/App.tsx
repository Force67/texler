import React, { useState, useEffect, useCallback } from 'react';
import Editor from '@monaco-editor/react';
import axios from 'axios';
import * as monaco from 'monaco-editor';
import './App.css';

const LATEX_API_URL = 'http://localhost:8081';

const defaultLatex = `\\documentclass{article}
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

Here's some inline math: \\(E = mc^2\\)

And here's a displayed equation:
\\begin{equation}
    \\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}
\\end{equation}

\\section{Lists}

\\begin{itemize}
    \\item First item
    \\item Second item
    \\begin{itemize}
        \\item Nested item 1
        \\item Nested item 2
    \\end{itemize}
    \\item Third item
\\end{itemize}

\\section{Tables}

\\begin{tabular}{|c|c|c|}
\\hline
\\textbf{Name} & \\textbf{Age} & \\textbf{City} \\\\
\\hline
Alice & 25 & New York \\\\
Bob & 30 & Los Angeles \\\\
Charlie & 35 & Chicago \\\\
\\hline
\\end{tabular}

\\section{Conclusion}

Start editing your LaTeX code on the left to see the compiled PDF on the right!

\\end{document}`;

function App() {
  const [latexCode, setLatexCode] = useState(defaultLatex);
  const [pdfUrl, setPdfUrl] = useState<string | null>(null);
  const [compiling, setCompiling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isFirstCompile, setIsFirstCompile] = useState(true);

  const compileLatex = useCallback(async () => {
    setCompiling(true);
    setError(null);

    try {
      console.log('Compiling LaTeX code:', latexCode.substring(0, 100) + '...');
      const response = await axios.post(`${LATEX_API_URL}/compile`, {
        files: {
          'document.tex': latexCode
        },
        mainFile: 'document.tex'
      });

      if (response.data.success && response.data.pdf) {
        console.log('Compilation successful, PDF length:', response.data.pdf.length);
        console.log('First 100 chars of PDF hex:', response.data.pdf.substring(0, 100));

        const hexString = response.data.pdf;
        const matches = hexString.match(/.{1,2}/g) || [];
        const pdfBytes = new Uint8Array(matches.map((byte: string) => parseInt(byte, 16)));

        console.log('PDF bytes array length:', pdfBytes.length);
        console.log('PDF bytes preview:', pdfBytes.slice(0, 10));

        const pdfBlob = new Blob([pdfBytes], { type: 'application/pdf' });
        const url = URL.createObjectURL(pdfBlob);

        console.log('Created blob URL:', url);

        if (pdfUrl) {
          URL.revokeObjectURL(pdfUrl);
        }

        setPdfUrl(url);
        if (isFirstCompile) {
          setIsFirstCompile(false);
        }
      } else {
        const errorMsg = `Compilation failed: ${response.data.parsedErrors?.[0]?.line || 'Unknown error'}`;
        setError(errorMsg);
        console.error('Compilation errors:', response.data.parsedErrors);
        console.error('Full response:', response.data);
      }
    } catch (err: any) {
      setError('Failed to connect to LaTeX compiler. Make sure the Docker container is running.');
      console.error('Error:', err);
      console.error('Full error details:', err.response?.data || err.message);
    } finally {
      setCompiling(false);
    }
  }, [latexCode, isFirstCompile]);

  useEffect(() => {
    const timer = setTimeout(() => {
      compileLatex();
    }, 1000);

    return () => clearTimeout(timer);
  }, [latexCode, compileLatex]);

  return (
    <div className="App">
      <header className="App-header">
        <h1>Texler - Collaborative LaTeX Editor</h1>
      </header>
      <div className="editor-container">
        <div className="editor-panel">
          <div className="panel-header">
            <h2>LaTeX Editor</h2>
            <button
              className="compile-button"
              onClick={compileLatex}
              disabled={compiling}
            >
              {compiling ? 'Compiling...' : 'Compile'}
            </button>
          </div>
          <Editor
            height="calc(100vh - 120px)"
            language="latex"
            defaultValue={latexCode}
            value={latexCode}
            onChange={(value) => setLatexCode(value || '')}
            theme="vs-dark"
            options={{
              minimap: { enabled: false },
              fontSize: 14,
              lineNumbers: 'on',
              roundedSelection: false,
              scrollBeyondLastLine: false,
              automaticLayout: true,
              wordWrap: 'on',
            }}
            beforeMount={(monaco) => {
              console.log('Monaco beforeMount:', monaco);

              // Define LaTeX language with proper syntax highlighting
              monaco.languages.register({ id: 'latex' });

              // Set up LaTeX syntax highlighting
              monaco.languages.setLanguageConfiguration('latex', {
                comments: {
                  lineComment: '%',
                },
                brackets: [
                  ['{', '}'],
                  ['[', ']'],
                  ['(', ')'],
                ],
                autoClosingPairs: [
                  { open: '{', close: '}' },
                  { open: '[', close: ']' },
                  { open: '(', close: ')' },
                  { open: '"', close: '"' },
                ],
                surroundingPairs: [
                  { open: '{', close: '}' },
                  { open: '[', close: ']' },
                  { open: '(', close: ')' },
                ],
              });

              // Register LaTeX tokens for syntax highlighting
              monaco.languages.setMonarchTokensProvider('latex', {
                tokenizer: {
                  root: [
                    [/%.*$/, 'comment'],
                    [/\\\\[a-zA-Z]+/, 'keyword'],
                    [/[{}()\[\]]/, 'delimiter'],
                    [/\$.*\$/, 'string.math'],
                    [/"[^"]*"/, 'string'],
                    [/'[^']*'/, 'string'],
                  ],
                },
              });

              console.log('LaTeX language with syntax highlighting registered successfully');
            }}
            onMount={(editor) => {
              console.log('Monaco editor mounted:', editor);
              console.log('Editor language ID:', editor.getModel()?.getLanguageId());
            }}
          />
        </div>
        <div className="preview-panel">
          <div className="panel-header">
            <h2>PDF Preview</h2>
          </div>
          <div className="pdf-container">
            {error && (
              <div style={{
                padding: '20px',
                backgroundColor: '#fee',
                borderRadius: '4px',
                margin: '10px',
                color: '#721c24',
                fontFamily: 'monospace'
              }}>
                <strong>Compilation Error:</strong><br />
                {error}
              </div>
            )}
            {pdfUrl && !error ? (
              <iframe
                src={pdfUrl}
                style={{
                  width: '100%',
                  height: '100%',
                  border: 'none',
                  borderRadius: '4px',
                  boxShadow: '0 4px 8px rgba(0, 0, 0, 0.1)'
                }}
                title="PDF Preview"
              />
            ) : !error ? (
              <div className="loading">
                {compiling ? 'Compiling LaTeX...' : 'PDF will appear here'}
              </div>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
