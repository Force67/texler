import React, { useState } from 'react';
import { FileNode } from '../types';

interface FileBrowserProps {
  files: Map<string, FileNode>;
  activeFile: string | null;
  mainFile: string | null;
  onFileClick: (path: string) => void;
  onMainFileSet: (path: string) => void;
  onAddFile: (path: string, template?: string) => void;
}

interface AddFileDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onAddFile: (path: string, template?: string) => void;
  existingFiles: Set<string>;
}

const AddFileDialog: React.FC<AddFileDialogProps> = ({ isOpen, onClose, onAddFile, existingFiles }) => {
  const [fileName, setFileName] = useState('');
  const [selectedTemplate, setSelectedTemplate] = useState('empty');

  const templates = [
    { value: 'empty', label: 'Empty File', content: '' },
    { value: 'tex-section', label: 'LaTeX Section', content: '\\section{New Section}\n\nContent goes here.' },
    { value: 'tex-chapter', label: 'LaTeX Chapter', content: '\\chapter{New Chapter}\n\nContent goes here.' },
    { value: 'tex-bibliography', label: 'Bibliography', content: '\\begin{thebibliography}{9}\n\n\\bibitem{ref1}\nAuthor, Title, Year\n\n\\end{thebibliography}' },
    { value: 'tex-figure', label: 'Figure Environment', content: '\\begin{figure}[h]\n\\centering\n\\includegraphics[width=0.8\\textwidth]{filename}\n\\caption{Figure caption}\n\\label{fig:label}\n\\end{figure}' },
  ];

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (fileName.trim()) {
      const template = templates.find(t => t.value === selectedTemplate);
      onAddFile(fileName.trim(), template?.content);
      setFileName('');
      setSelectedTemplate('empty');
      onClose();
    }
  };

  const validateFileName = (name: string) => {
    if (!name.trim()) return 'File name is required';
    if (existingFiles.has(name.trim())) return 'File already exists';
    if (name.includes('//')) return 'Invalid path';
    return '';
  };

  const validationError = validateFileName(fileName);

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Add New File</h3>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>

        <form onSubmit={handleSubmit} className="add-file-form">
          <div className="form-group">
            <label htmlFor="fileName">File Name:</label>
            <input
              id="fileName"
              type="text"
              value={fileName}
              onChange={(e) => setFileName(e.target.value)}
              placeholder="e.g., newfile.tex or sections/chapter.tex"
              className={`form-input ${validationError ? 'error' : ''}`}
              autoFocus
            />
            {validationError && <span className="error-message">{validationError}</span>}
          </div>

          <div className="form-group">
            <label htmlFor="template">Template:</label>
            <select
              id="template"
              value={selectedTemplate}
              onChange={(e) => setSelectedTemplate(e.target.value)}
              className="form-select"
            >
              {templates.map(template => (
                <option key={template.value} value={template.value}>
                  {template.label}
                </option>
              ))}
            </select>
          </div>

          <div className="form-actions">
            <button type="button" onClick={onClose} className="btn btn-secondary">
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={!!validationError || !fileName.trim()}
            >
              Add File
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export const FileBrowser: React.FC<FileBrowserProps> = ({
  files,
  activeFile,
  mainFile,
  onFileClick,
  onMainFileSet,
  onAddFile,
}) => {
  const [showAddFileDialog, setShowAddFileDialog] = useState(false);

  // Build file tree structure
  const buildTree = () => {
    const tree: { [key: string]: FileNode[] } = {};

    // Group files by directory
    files.forEach((file, path) => {
      const parts = path.split('/');
      const dir = parts.length > 1 ? parts[0] : '';

      if (!tree[dir]) {
        tree[dir] = [];
      }

      tree[dir].push(file);
    });

    return tree;
  };

  const tree = buildTree();

  const handleFileClick = (path: string) => {
    onFileClick(path);
  };

  const handleSetMainFile = (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    onMainFileSet(path);
  };

  const getIcon = (filename: string) => {
    return filename.endsWith('.tex') ? '📄' : '📝';
  };

  return (
    <div className="file-browser">
      <div className="file-browser-header">
        <div className="header-left">
          <h3>📁 Project Files</h3>
          {mainFile && (
            <div className="main-file-info">
              <span className="main-file-label">Main:</span>
              <span className="main-file-name">{files.get(mainFile)?.name}</span>
            </div>
          )}
        </div>
        <button
          className="add-file-btn"
          onClick={() => setShowAddFileDialog(true)}
          title="Add new file"
        >
          + New File
        </button>
      </div>

      <div className="file-tree">
        {Object.entries(tree).map(([dir, fileList]) => (
          <div key={dir} className="file-group">
            {dir && (
              <div className="directory">
                <span className="directory-icon">📂</span>
                <span className="directory-name">{dir}</span>
              </div>
            )}
            <div className={`${dir ? 'sub-files' : 'root-files'}`}>
              {fileList.map((file) => (
                <div
                  key={file.path}
                  className={`file-item ${activeFile === file.path ? 'active' : ''} ${mainFile === file.path ? 'main-file' : ''}`}
                  onClick={() => handleFileClick(file.path)}
                >
                  <span className="file-icon">{getIcon(file.name)}</span>
                  <span className="file-name">{file.name}</span>
                  {file.isModified && <span className="modified-indicator">●</span>}
                  {file.path.endsWith('.tex') && (
                    <button
                      className={`main-file-btn ${mainFile === file.path ? 'active' : ''}`}
                      onClick={(e) => handleSetMainFile(file.path, e)}
                      title={mainFile === file.path ? 'This is the main file' : 'Set as main compilation file'}
                    >
                      {mainFile === file.path ? '⭐' : '☆'}
                    </button>
                  )}
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>

      <div className="file-browser-footer">
        <div className="legend">
          <div className="legend-item">
            <span>⭐</span>
            <span>Main file</span>
          </div>
          <div className="legend-item">
            <span>●</span>
            <span>Modified</span>
          </div>
        </div>
      </div>

      <AddFileDialog
        isOpen={showAddFileDialog}
        onClose={() => setShowAddFileDialog(false)}
        onAddFile={onAddFile}
        existingFiles={new Set(files.keys())}
      />
    </div>
  );
};