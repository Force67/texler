import React from 'react';
import { FileNode } from '../types';

interface FileBrowserProps {
  files: Map<string, FileNode>;
  activeFile: string | null;
  mainFile: string | null;
  onFileClick: (path: string) => void;
  onMainFileSet: (path: string) => void;
}

export const FileBrowser: React.FC<FileBrowserProps> = ({
  files,
  activeFile,
  mainFile,
  onFileClick,
  onMainFileSet,
}) => {
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
        <h3>📁 Project Files</h3>
        {mainFile && (
          <div className="main-file-info">
            <span className="main-file-label">Main:</span>
            <span className="main-file-name">{files.get(mainFile)?.name}</span>
          </div>
        )}
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
    </div>
  );
};