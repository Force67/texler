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

              // Register LaTeX tokens for comprehensive syntax highlighting
              monaco.languages.setMonarchTokensProvider('latex', {
                tokenizer: {
                  root: [
                    // Comments
                    [/%.*$/, 'comment'],

                    // Math mode - simplified approach
                    [/\$\$[^$]*\$\$/, 'string.math'],
                    [/\\\[\\\][^]]*\\\]/, 'string.math'],
                    [/\$[^$]*\$/, 'string.math'],
                    [/\\\([^)]*\\\)/, 'string.math'],

                    // Document structure and sectioning
                    [/\\(documentclass|documentstyle|usepackage|RequirePackage|input|include|appendix)\b/, 'keyword.document'],
                    [/\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)\b/, 'keyword.section'],
                    [/\\(begin|end)\b/, 'keyword.environment'],
                    [/\\(label|ref|pageref|cite|bibliography|bibliographystyle|bibitem|nocite)\b/, 'keyword.reference'],

                    // Font styling and formatting
                    [/\\(textbf|textit|textmd|textnormal|textrm|textsf|texttt|textsc|textup|textsl|emph|underline)\b/, 'keyword.format'],
                    [/\\(tiny|scriptsize|footnotesize|small|normalsize|large|Large|LARGE|huge|Huge)\b/, 'keyword.size'],
                    [/\\(bfseries|mdseries|rmfamily|sffamily|ttfamily|upshape|itshape|slshape|scshape)\b/, 'keyword.format'],

                    // Math operators and symbols
                    [/\\(alpha|beta|gamma|delta|epsilon|varepsilon|zeta|eta|theta|vartheta|iota|kappa|lambda|mu|nu|xi|pi|varpi|rho|varrho|sigma|varsigma|tau|upsilon|phi|varphi|chi|psi|omega|Gamma|Delta|Theta|Lambda|Xi|Pi|Sigma|Upsilon|Phi|Psi|Omega)\b/, 'variable.math.greek'],
                    [/\\(sin|cos|tan|arcsin|arccos|arctan|ln|log|exp|lim|sup|inf|max|min|det|mod|gcd|lcm|Pr|operatorname)\b/, 'keyword.function.math'],
                    [/\\(sum|prod|coprod|int|iint|iiint|oint|bigcup|bigcap|biguplus|bigvee|bigwedge|bigoplus|bigotimes|bigodot|bigsqcup)\b/, 'keyword.operator.math'],
                    [/\\(left|right|big|Big|bigg|Bigg|middle)\b/, 'keyword.math.delimiter'],
                    [/\\(frac|sqrt|root|over|under|stackrel|overset|underset|choose|brace|brack|binom|genfrac)\b/, 'keyword.math.construct'],
                    [/\\(limits|nolimits|displaystyle|textstyle|scriptstyle|scriptscriptstyle)\b/, 'keyword.math.style'],

                    // Spacing and layout
                    [/\\(quad|qquad|smallskip|medskip|bigskip|vspace|hspace|newline|linebreak|pagebreak|newpage|clearpage|cleardoublepage)\b/, 'keyword.layout'],
                    [/\\[ ,;!]/, 'keyword.spacing'],

                    // Lists and environments
                    [/\\(item|itemize|enumerate|description|list|label|width|leftmargin|rightmargin|topsep|partopsep|parsep|itemsep)\b/, 'keyword.list'],

                    // Tables and figures
                    [/\\(tabular|tabulararray|tabularx|tabulary|array|matrix|pmatrix|bmatrix|vmatrix|Vmatrix|cases|aligned|gathered|split)\b/, 'keyword.table'],
                    [/\\(figure|table|center|flushleft|flushright|minipage|parbox|makebox|fbox|framebox|raisebox|rotatebox|scalebox|resizebox)\b/, 'keyword.float'],
                    [/\\(includegraphics|graphicspath|declaregraphics|includegraphics|height|width|scale|angle|clip|trim|viewport)\b/, 'keyword.graphic'],

                    // Bibliography and references
                    [/\\(thebibliography|bibliographystyle|bibliography|bibitem|cite|nocite|ref|label|pageref|eqref|vref|vpageref|prettyref|autoref|nameref)\b/, 'keyword.bib'],

                    // Cross-references and hyperlinks
                    [/\\(href|url|hyperref|hypertarget|hyperlink|autoref|footnote|footnotemark|footnotetext|marginpar|thanks)\b/, 'keyword.hyperlink'],

                    // Custom commands and environments
                    [/\\(newcommand|renewcommand|providecommand|DeclareMathOperator|newenvironment|renewenvironment|newtheorem|newcounter|setcounter|value|the|arabic|roman|Roman|alph|Alph|fnsymbol)\b/, 'keyword.definition'],

                    // Verbatim and code
                    [/\\(verb|verb\*|verbatim|verbatim\*|texttt|textsf|textsc|textup|textmd|textbf|textit|textsl)\b/, 'keyword.code'],

                    // Boxes and positioning
                    [/\\(hbox|vbox|makebox|raisebox|parbox|minipage|vcenter|vtop|vskip|vspace|hskip|hspace|kern|mkern|hfill|vfill|hspace\*|vspace\*)\b/, 'keyword.box'],

                    // Colors and graphics
                    [/\\(color|textcolor|pagecolor|definecolor|usepackage|pagecolor|colorbox|fcolorbox)\b/, 'keyword.color'],

                    // General LaTeX commands (catch-all)
                    [/\\[a-zA-Z]+/, 'keyword'],

                    // Brackets and delimiters
                    [/[{}()\[\]]/, 'delimiter'],

                    // Numbers and measurements
                    [/\d+(\.\d+)?(pt|mm|cm|in|ex|em|bp|dd|pc|sp|cc|cm|mm|nd|nc|bp)/, 'number'],
                    [/\d+/, 'number'],

                    // Strings and text content
                    [/"[^"]*"/, 'string.quoted'],
                    [/'[^']*'/, 'string.quoted'],

                    // Special characters
                    [/[&~^#%_$]/, 'delimiter.special'],
                  ],
                },
              });

              // Define custom theme colors for LaTeX syntax highlighting
              monaco.editor.defineTheme('latex-theme', {
                base: 'vs',
                inherit: true,
                rules: [
                  // Comments
                  { token: 'comment', foreground: '6A9955' },

                  // Document structure
                  { token: 'keyword.document', foreground: '569CD6', fontStyle: 'bold' },
                  { token: 'keyword.section', foreground: 'C586C0', fontStyle: 'bold' },
                  { token: 'keyword.environment', foreground: '4EC9B0', fontStyle: 'bold' },

                  // References and citations
                  { token: 'keyword.reference', foreground: '9CDCFE' },
                  { token: 'keyword.bib', foreground: 'D19A66' },
                  { token: 'keyword.hyperlink', foreground: '4EC9B0', fontStyle: 'underline' },

                  // Text formatting
                  { token: 'keyword.format', foreground: 'D4A446' },
                  { token: 'keyword.size', foreground: 'D4A446', fontStyle: 'italic' },

                  // Math content
                  { token: 'string.math', foreground: 'B5CEA8' },
                  { token: 'variable.math.greek', foreground: 'CE9178' },
                  { token: 'keyword.function.math', foreground: 'DCDCAA' },
                  { token: 'keyword.operator.math', foreground: 'C586C0', fontStyle: 'bold' },
                  { token: 'keyword.math.delimiter', foreground: 'CE9178', fontStyle: 'bold' },
                  { token: 'keyword.math.construct', foreground: 'DCDCAA', fontStyle: 'bold' },
                  { token: 'keyword.math.style', foreground: '9CDCFE', fontStyle: 'italic' },

                  // Layout and spacing
                  { token: 'keyword.layout', foreground: '808080' },
                  { token: 'keyword.spacing', foreground: '808080' },

                  // Lists and tables
                  { token: 'keyword.list', foreground: '4EC9B0' },
                  { token: 'keyword.table', foreground: '4FC1FF' },
                  { token: 'keyword.float', foreground: '4EC9B0' },
                  { token: 'keyword.graphic', foreground: 'D19A66' },

                  // Code and verbatim
                  { token: 'keyword.code', foreground: 'CE9178' },

                  // Boxes and positioning
                  { token: 'keyword.box', foreground: '808080' },

                  // Colors
                  { token: 'keyword.color', foreground: 'D19A66' },

                  // Command definitions
                  { token: 'keyword.definition', foreground: '569CD6', fontStyle: 'bold' },

                  // General keywords
                  { token: 'keyword', foreground: '569CD6' },

                  // Numbers and measurements
                  { token: 'number', foreground: 'B5CEA8' },

                  // Quoted strings
                  { token: 'string.quoted', foreground: 'CE9178' },

                  // Special characters
                  { token: 'delimiter.special', foreground: 'C586C0' },
                ],
                colors: {},
              });

              // Apply the custom theme
              monaco.editor.setTheme('latex-theme');

              console.log('LaTeX language with comprehensive syntax highlighting registered successfully');
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
