export interface UserProfile {
  id: string;
  username: string;
  email: string;
  display_name: string;
  avatar_url?: string | null;
  is_active: boolean;
  email_verified: boolean;
  last_login_at?: string | null;
  created_at: string;
}

export interface LoginResponsePayload {
  user: UserProfile;
  access_token: string;
  refresh_token: string;
  expires_in: number;
}

export interface FileNode {
  name: string;
  path: string;
  content: string;
  isDirectory?: boolean;
  children?: FileNode[];
  isOpen?: boolean;
  isModified?: boolean;
}

export interface WorkspaceSummary {
  id: string;
  name: string;
  description?: string | null;
  owner_id: string;
  project_count: number;
  created_at: string;
  updated_at: string;
  projects: ProjectSummary[];
}

export interface ProjectSummary {
  id: string;
  workspace_id: string;
  name: string;
  description?: string | null;
  main_file?: string | null;
  file_count: number;
  created_at: string;
  updated_at: string;
}

export interface ProjectFilePayload {
  path: string;
  content: string;
  is_main: boolean;
  updated_at: string;
}

export interface ProjectDetails extends ProjectSummary {
  files: Record<string, ProjectFilePayload>;
}

export interface ProjectState {
  files: Map<string, FileNode>; // path -> FileNode
  openFiles: string[]; // array of file paths
  activeFile: string | null; // currently active file path
  mainFile: string | null; // main compilation file
  projectPath: string | null; // project directory path
  workspaceId: string | null;
  projectId: string | null;
  workspaceName: string | null;
  projectName: string | null;
  workspaces: WorkspaceSummary[];
  loading: boolean;
}

export interface CompilationResult {
  success: boolean;
  pdf?: string; // hex-encoded PDF data
  errors?: any[];
  output?: string;
}
