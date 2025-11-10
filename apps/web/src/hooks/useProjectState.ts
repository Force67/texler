import { useState, useCallback } from 'react';
import { FileNode, ProjectState } from '../types';

const DEFAULT_CONTENT: { [key: string]: string } = {
  'main.tex': `\\documentclass[12pt,a4paper]{article}

% Packages
\\usepackage[utf8]{inputenc}
\\usepackage[T1]{fontenc}
\\usepackage{amsmath,amssymb,amsfonts}
\\usepackage{graphicx}
\\usepackage{hyperref}
\\usepackage{geometry}

% Geometry
\\geometry{margin=1in}

% Title and author
\\title{Multi-File LaTeX Document}
\\author{Your Name}
\\date{\\today}

\\begin{document}

\\maketitle

\\tableofcontents
\\newpage

% Include sections
\\include{sections/introduction}

% Add more sections here

\\end{document}`,
  'sections/introduction.tex': `\\section{Introduction}

This is the introduction section of your multi-file LaTeX document.

\\subsection{Background}

You can write your introduction content here. LaTeX automatically handles:

\\begin{itemize}
\\item Section numbering
\\item Cross-references
\\item Citations
\\item Mathematical equations
\\end{itemize}

\\subsection{Mathematical Example}

Here's some mathematics to test compilation:

\\begin{equation}
E = mc^2
\\end{equation}

\\begin{equation}
\\int_{0}^{\\infty} e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}
\\end{equation}`
};

export const useProjectState = () => {
  const [state, setState] = useState<ProjectState>({
    files: new Map(),
    openFiles: [],
    activeFile: null,
    mainFile: 'main.tex',
    projectPath: null,
  });
  const [version, setVersion] = useState(0);

  const createProject = useCallback(() => {
    // Create a default multi-file project
    const files = new Map<string, FileNode>();

    // Add main.tex
    files.set('main.tex', {
      name: 'main.tex',
      path: 'main.tex',
      content: DEFAULT_CONTENT['main.tex'],
      isModified: false,
    });

    // Add sections/introduction.tex
    files.set('sections/introduction.tex', {
      name: 'introduction.tex',
      path: 'sections/introduction.tex',
      content: DEFAULT_CONTENT['sections/introduction.tex'],
      isModified: false,
    });

    setState(prev => ({
      ...prev,
      files,
      openFiles: ['main.tex'],
      activeFile: 'main.tex',
      mainFile: 'main.tex',
    }));
    setVersion(v => v + 1);
  }, []);

  const addFile = useCallback((path: string, content: string = '') => {
    const fileNode: FileNode = {
      name: path.split('/').pop() || path,
      path,
      content,
      isModified: true,
    };

    setState(prev => ({
      ...prev,
      files: new Map(prev.files).set(path, fileNode),
      openFiles: [...prev.openFiles, path],
      activeFile: path,
    }));
    setVersion(v => {
      console.log('🔢 Version increment (addFile):', v, '->', v + 1);
      return v + 1;
    });
  }, []);

  const updateFile = useCallback((path: string, content: string) => {
    setState(prev => {
      const files = new Map(prev.files);
      const file = files.get(path);
      if (file) {
        file.content = content;
        file.isModified = true;
        files.set(path, file);
      }
      return { ...prev, files };
    });
    setVersion(v => v + 1);
  }, []);

  const openFile = useCallback((path: string) => {
    setState(prev => ({
      ...prev,
      openFiles: prev.openFiles.includes(path)
        ? prev.openFiles
        : [...prev.openFiles, path],
      activeFile: path,
    }));
  }, []);

  const closeFile = useCallback((path: string) => {
    setState(prev => {
      const openFiles = prev.openFiles.filter(f => f !== path);
      let activeFile = prev.activeFile;

      // If closing the active file, switch to another open file
      if (activeFile === path) {
        activeFile = openFiles.length > 0 ? openFiles[0] : null;
      }

      return { ...prev, openFiles, activeFile };
    });
  }, []);

  const setActiveFile = useCallback((path: string | null) => {
    setState(prev => ({ ...prev, activeFile: path }));
  }, []);

  const setMainFile = useCallback((path: string) => {
    setState(prev => ({ ...prev, mainFile: path }));
    setVersion(v => v + 1);
  }, []);

  const getActiveFileContent = useCallback(() => {
    const { activeFile, files } = state;
    if (!activeFile) return '';
    return files.get(activeFile)?.content || '';
  }, [state.activeFile, state.files]);

  const getCompilationData = useCallback(() => {
    const { files, mainFile } = state;

    if (!mainFile || !files.has(mainFile)) {
      return null;
    }

    // Convert files Map to the format expected by the API
    const apiFiles: { [key: string]: string } = {};

    files.forEach((file, path) => {
      apiFiles[path] = file.content;
    });

    return {
      files: apiFiles,
      mainFile,
    };
  }, [state.files, state.mainFile]);

  return {
    ...state,
    version,
    createProject,
    addFile,
    updateFile,
    openFile,
    closeFile,
    setActiveFile,
    setMainFile,
    getActiveFileContent,
    getCompilationData,
  };
};