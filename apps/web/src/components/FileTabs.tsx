import React from 'react';

interface FileTabsProps {
  openFiles: string[];
  activeFile: string | null;
  files: Map<string, { name: string; isModified?: boolean }>;
  onTabClick: (path: string) => void;
  onTabClose: (path: string) => void;
}

export const FileTabs: React.FC<FileTabsProps> = ({
  openFiles,
  activeFile,
  files,
  onTabClick,
  onTabClose,
}) => {
  const getDisplayName = (path: string) => {
    return files.get(path)?.name || path.split('/').pop() || path;
  };

  const isModified = (path: string) => {
    return files.get(path)?.isModified || false;
  };

  return (
    <div className="file-tabs">
      <div className="tabs-container">
        {openFiles.map((path) => (
          <div
            key={path}
            className={`tab ${activeFile === path ? 'active' : ''}`}
            onClick={() => onTabClick(path)}
          >
            <span className="tab-content">
              <span className="tab-name">{getDisplayName(path)}</span>
              {isModified(path) && <span className="modified-dot">●</span>}
            </span>
            <button
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                onTabClose(path);
              }}
              title={`Close ${getDisplayName(path)}`}
            >
              ×
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};