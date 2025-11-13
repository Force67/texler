import React, { useState, useEffect, useCallback } from 'react';
import Editor from '@monaco-editor/react';
import axios from 'axios';
import * as monaco from 'monaco-editor';
import { useProjectState } from './hooks/useProjectState';
import { FileBrowser } from './components/FileBrowser';
import { FileTabs } from './components/FileTabs';
import { LoginPanel } from './components/LoginPanel';
import { useAuth } from './hooks/useAuth';
import './App.css';
import { BACKEND_API_URL } from './config';

function App() {
  const auth = useAuth();
  const projectState = useProjectState(auth.token);
  const [pdfUrl, setPdfUrl] = useState<string | null>(null);
  const [compiling, setCompiling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isFirstCompile, setIsFirstCompile] = useState(true);
  const [editorZoom, setEditorZoom] = useState(14);

  const compileLatex = useCallback(async () => {
    const compilationData = projectState.getCompilationData();
    if (!compilationData) {
      setError('No files to compile or main file not set');
      return;
    }

    setCompiling(true);
    setError(null);

    try {
      // Transform data to match Rust backend expectations
      const requestData = {
        files: compilationData.files,
        main_file: compilationData.mainFile
      };

      console.log('Compiling multi-file project:', {
        fileCount: Object.keys(compilationData.files).length,
        mainFile: compilationData.mainFile,
        files: Object.keys(compilationData.files)
      });

      console.log('Sending to Rust backend:', requestData);
      const response = await axios.post(`${BACKEND_API_URL}/api/v1/latex/compile`, requestData);

      console.log('Backend response:', response.data);

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
  }, [projectState.getCompilationData, isFirstCompile]);

  // Force compilation when files or active file changes
  useEffect(() => {
    if (!projectState.isReady) {
      return;
    }

    const timer = setTimeout(() => {
      compileLatex();
    }, 1000);

    return () => clearTimeout(timer);
  }, [
    projectState.activeFile,
    projectState.version,
    projectState.isReady,
    compileLatex
  ]);

  const handleCreateNewProject = async () => {
    await projectState.createProject();
  };

  const handleCreateWorkspace = async () => {
    const name = prompt('Workspace name (e.g., Research Notes)');
    if (name) {
      await projectState.createWorkspace(name.trim());
    }
  };

  const handleEditorZoom = (editor: monaco.editor.IStandaloneCodeEditor, delta: number) => {
    const newZoom = Math.max(8, Math.min(72, editorZoom + delta));
    setEditorZoom(newZoom);
    editor.updateOptions({ fontSize: newZoom });
  };

  const handleForceRefresh = () => {
    compileLatex();
  };

  const activeFileContent = projectState.getActiveFileContent();

  const workspaceOptions = projectState.workspaces;
  const selectedWorkspace = workspaceOptions.find(ws => ws.id === projectState.workspaceId) || workspaceOptions[0];
  const projectOptions = selectedWorkspace?.projects ?? [];
  const fileKeys = new Set(Array.from(projectState.files.keys()));
  const userDisplayName = auth.user?.display_name || auth.user?.username || 'Texler User';
  const userEmail = auth.user?.email ?? '';
  const userInitials = userDisplayName
    .split(' ')
    .map(part => part[0])
    .filter(Boolean)
    .slice(0, 2)
    .join('')
    .toUpperCase() || 'TX';
  const compileDisabled = compiling || !projectState.isReady;
  const compileStatusLabel = error ? 'Needs attention' : compiling ? 'Compiling…' : 'Live preview';
  const compileStatusVariant = error ? 'error' : compiling ? 'active' : 'idle';

  const handleWorkspaceSelect = async (workspaceId: string) => {
    if (workspaceId) {
      await projectState.selectWorkspace(workspaceId);
    }
  };

  const handleProjectSelect = async (projectId: string) => {
    if (projectId) {
      await projectState.selectProject(projectId, selectedWorkspace?.id);
    }
  };

  if (auth.loading && !auth.isAuthenticated) {
    return (
      <div className="app-loading-screen">
        <div className="loading-spinner" />
        <p>Preparing Texler…</p>
      </div>
    );
  }

  if (!auth.isAuthenticated) {
    return (
      <LoginPanel
        onLogin={auth.login}
        loading={auth.loading}
        error={auth.error}
      />
    );
  }

  return (
    <div className="App">
      <header className="App-header">
        <div className="brand-block">
          <div className="brand-logo">TX</div>
          <div className="brand-copy">
            <h1>Texler IDE</h1>
            <p>{projectState.projectName ? `Editing ${projectState.projectName}` : 'Select a project to get started'}</p>
          </div>
        </div>
        <div className="header-actions">
          <div className="status-pills">
            <span className="status-pill">
              Workspaces
              <strong>{workspaceOptions.length}</strong>
            </span>
            <span className="status-pill">
              Files
              <strong>{projectState.files.size}</strong>
            </span>
          </div>
          <button
            className="compile-button"
            onClick={compileLatex}
            disabled={compileDisabled}
          >
            {compiling ? 'Compiling…' : 'Compile'}
          </button>
          <div className="user-chip">
            <div className="user-avatar">{userInitials}</div>
            <div className="user-meta">
              <span className="user-name">{userDisplayName}</span>
              <span className="user-email">{userEmail}</span>
            </div>
            <button className="chip-logout" onClick={auth.logout} title="Log out">
              Log out
            </button>
          </div>
        </div>
      </header>
      <div className="app-body">
        <aside className="workspace-sidebar">
          <div className="sidebar-section">
            <div className="section-header">
              <div>
                <h3>Workspaces</h3>
                <p>Switch between personal or shared spaces</p>
              </div>
              <button className="section-cta" onClick={handleCreateWorkspace}>
                + Workspace
              </button>
            </div>
            <div className="workspace-list">
              {workspaceOptions.length ? (
                workspaceOptions.map(workspace => (
                  <button
                    key={workspace.id}
                    className={`workspace-card ${selectedWorkspace?.id === workspace.id ? 'active' : ''}`}
                    onClick={() => handleWorkspaceSelect(workspace.id)}
                  >
                    <div className="workspace-name">{workspace.name}</div>
                    <div className="workspace-meta">
                      <span>{workspace.project_count} projects</span>
                      <span>{new Date(workspace.updated_at).toLocaleDateString()}</span>
                    </div>
                  </button>
                ))
              ) : (
                <p className="empty-state">No workspaces yet. Create one to start writing.</p>
              )}
            </div>
          </div>

          <div className="sidebar-section">
            <div className="section-header">
              <div>
                <h3>Projects</h3>
                <p>{selectedWorkspace ? `in ${selectedWorkspace.name}` : 'Select a workspace to load projects'}</p>
              </div>
              <button className="section-cta" onClick={handleCreateNewProject} disabled={!selectedWorkspace}>
                + Project
              </button>
            </div>
            <div className="project-list">
              {selectedWorkspace && projectOptions.length ? (
                projectOptions.map(project => (
                  <button
                    key={project.id}
                    className={`project-card ${projectState.projectId === project.id ? 'active' : ''}`}
                    onClick={() => handleProjectSelect(project.id)}
                  >
                    <div className="project-name">{project.name}</div>
                    <div className="project-meta">
                      <span>{project.file_count} files</span>
                      <span>{project.main_file ? `Main: ${project.main_file}` : 'Main file unassigned'}</span>
                    </div>
                  </button>
                ))
              ) : (
                <p className="empty-state">
                  {selectedWorkspace
                    ? 'No projects here yet. Create one to begin writing.'
                    : 'Choose a workspace to see projects.'}
                </p>
              )}
            </div>
          </div>
        </aside>

        <div className="content-panels">
          <div className="editor-stack">
            <div className="editor-container">
              <div className="file-browser-panel">
                <FileBrowser
                  files={projectState.files}
                  activeFile={projectState.activeFile}
                  mainFile={projectState.mainFile}
                  onFileClick={projectState.openFile}
                  onMainFileSet={projectState.setMainFile}
                  onAddFile={projectState.addFile}
                  existingFiles={fileKeys}
                />
              </div>

              <div className="editor-panel">
                <FileTabs
                  openFiles={projectState.openFiles}
                  activeFile={projectState.activeFile}
                  files={projectState.files}
                  onTabClick={projectState.openFile}
                  onTabClose={projectState.closeFile}
                />

                <div className="zoom-indicator">
                  <span>Zoom: {editorZoom}px</span>
                  <div className="zoom-controls">
                    <button onClick={() => {
                      const newZoom = Math.max(8, editorZoom - 1);
                      setEditorZoom(newZoom);
                    }} title="Zoom Out (Ctrl + Mouse Wheel Down)">−</button>
                    <button onClick={() => setEditorZoom(14)} title="Reset Zoom">100%</button>
                    <button onClick={() => {
                      const newZoom = Math.min(72, editorZoom + 1);
                      setEditorZoom(newZoom);
                    }} title="Zoom In (Ctrl + Mouse Wheel Up)">+</button>
                    <button onClick={handleForceRefresh} title="Force Compile (Debug)">🔄</button>
                  </div>
                </div>

                <div className="editor-container-inner">
                  <Editor
                    height="100%"
                    language="latex"
                    defaultValue=""
                    value={activeFileContent}
                    onChange={(value) => {
                      if (projectState.activeFile) {
                        projectState.updateFile(projectState.activeFile, value || '');
                      }
                    }}
                    theme="vs-dark"
                    options={{
                      minimap: { enabled: false },
                      fontSize: editorZoom,
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

              // Add Ctrl + mouse wheel zoom functionality
              const editorDomNode = editor.getDomNode();
              if (editorDomNode) {
                editorDomNode.addEventListener('wheel', (event: WheelEvent) => {
                  if (event.ctrlKey) {
                    event.preventDefault();
                    const delta = event.deltaY > 0 ? -1 : 1;
                    handleEditorZoom(editor, delta);
                  }
                }, { passive: false });
              }
            }}
          />
                </div>
              </div>
            </div>
          </div>

          <div className="preview-panel">
            <div className="panel-header">
              <h2>PDF Preview</h2>
              <div className="header-controls">
                <span className={`compile-status ${compileStatusVariant}`}>
                  {compileStatusLabel}
                </span>
                {projectState.mainFile && (
                  <span className="main-file-indicator">
                    Compiling: {projectState.files.get(projectState.mainFile)?.name}
                  </span>
                )}
                <span className="file-count" title={`Files: ${Array.from(projectState.files.keys()).join(', ')}`}>
                  {projectState.files.size} files
                </span>
              </div>
            </div>
            <div className="pdf-container">
              {error && (
                <div className="error-display">
                  <strong>Compilation Error:</strong>{<br />}
                  {error}
                </div>
              )}
              {pdfUrl && !error ? (
                <iframe
                  src={pdfUrl}
                  className="pdf-iframe"
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
    </div>
  );
}

export default App;
