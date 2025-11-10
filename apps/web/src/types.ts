export interface FileNode {
  name: string;
  path: string;
  content: string;
  isDirectory?: boolean;
  children?: FileNode[];
  isOpen?: boolean;
  isModified?: boolean;
}

export interface ProjectState {
  files: Map<string, FileNode>; // path -> FileNode
  openFiles: string[]; // array of file paths
  activeFile: string | null; // currently active file path
  mainFile: string | null; // main compilation file
  projectPath: string | null; // project directory path
}

export interface CompilationResult {
  success: boolean;
  pdf?: string; // hex-encoded PDF data
  errors?: any[];
  output?: string;
}