import { useState, useCallback, useEffect, useRef } from 'react';
import axios from 'axios';
import { BACKEND_API_URL } from '../config';
import { FileNode, ProjectDetails, ProjectState, WorkspaceSummary } from '../types';

const api = axios.create({ baseURL: BACKEND_API_URL });

const getAuthToken = () => {
  if (typeof window !== 'undefined') {
    const localToken = window.localStorage.getItem('texler_token');
    if (localToken) {
      return localToken;
    }
  }
  return import.meta.env.VITE_TEXLER_TOKEN || '';
};

api.interceptors.request.use(config => {
  const token = getAuthToken();
  if (token) {
    config.headers = config.headers ?? {};
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

type WorkspaceListResponse = { workspaces: WorkspaceSummary[] };
type ProjectResponse = { project: ProjectDetails };
type FileResponse = { file: { path: string } };

const createInitialState = (): ProjectState => ({
  files: new Map(),
  openFiles: [],
  activeFile: null,
  mainFile: null,
  projectPath: null,
  workspaceId: null,
  projectId: null,
  workspaceName: null,
  projectName: null,
  workspaces: [],
  loading: true,
});

export const useProjectState = (authToken?: string | null) => {
  const [state, setState] = useState<ProjectState>(createInitialState);
  const [version, setVersion] = useState(0);
  const pendingSaves = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const isAuthenticated = Boolean(authToken || getAuthToken());

  useEffect(() => {
    if (authToken) {
      api.defaults.headers.common = {
        ...(api.defaults.headers.common || {}),
        Authorization: `Bearer ${authToken}`,
      };
    } else if (api.defaults.headers.common?.Authorization) {
      delete api.defaults.headers.common.Authorization;
    }
  }, [authToken]);

  const hydrateProject = useCallback((workspaceId: string, project: ProjectDetails, workspaceName?: string | null) => {
    const files = new Map<string, FileNode>();
    Object.entries(project.files || {}).forEach(([path, file]) => {
      files.set(path, {
        name: path.split('/').pop() || path,
        path,
        content: file.content,
        isModified: false,
      });
    });

    const firstFile = files.size > 0 ? Array.from(files.keys())[0] : null;
    const mainFile = project.main_file && files.has(project.main_file) ? project.main_file : firstFile;
    const openFiles = mainFile ? [mainFile] : [];

    setState(prev => ({
      ...prev,
      files,
      openFiles,
      activeFile: openFiles[0] ?? null,
      mainFile,
      workspaceId,
      projectId: project.id,
      projectPath: project.name,
      workspaceName: workspaceName ?? prev.workspaces.find(w => w.id === workspaceId)?.name ?? prev.workspaceName,
      projectName: project.name,
      loading: false,
    }));
    setVersion(v => v + 1);
  }, []);

  const refreshWorkspaces = useCallback(async () => {
    if (!isAuthenticated) {
      return [] as WorkspaceSummary[];
    }
    try {
      const response = await api.get<WorkspaceListResponse>('/api/v1/workspaces');
      const workspaces = response.data.workspaces || [];
      setState(prev => ({ ...prev, workspaces }));
      return workspaces;
    } catch (error) {
      console.error('Failed to load workspaces', error);
      return [] as WorkspaceSummary[];
    }
  }, [isAuthenticated]);

  const loadProject = useCallback(async (workspaceId: string, projectId: string, workspaceName?: string | null) => {
    if (!isAuthenticated) return;
    setState(prev => ({ ...prev, loading: true }));
    try {
      const response = await api.get<ProjectResponse>(`/api/v1/workspaces/${workspaceId}/projects/${projectId}`);
      hydrateProject(workspaceId, response.data.project, workspaceName);
    } catch (error) {
      console.error('Failed to load project', error);
      setState(prev => ({ ...prev, loading: false }));
    }
  }, [hydrateProject, isAuthenticated]);

  const bootstrap = useCallback(async () => {
    if (!isAuthenticated) {
      setState(prev => ({ ...createInitialState(), loading: false }));
      return;
    }
    const workspaces = await refreshWorkspaces();
    if (!workspaces.length) {
      setState(prev => ({ ...prev, loading: false }));
      return;
    }

    const firstWorkspace = workspaces[0];
    const firstProject = firstWorkspace.projects[0];
    if (firstWorkspace && firstProject) {
      await loadProject(firstWorkspace.id, firstProject.id, firstWorkspace.name);
    } else {
      setState(prev => ({
        ...prev,
        workspaceId: firstWorkspace.id,
        workspaceName: firstWorkspace.name,
        loading: false,
      }));
    }
  }, [refreshWorkspaces, loadProject, isAuthenticated]);

  useEffect(() => {
    if (!isAuthenticated) {
      setState({ ...createInitialState(), loading: false });
      return;
    }
    bootstrap();
  }, [isAuthenticated, bootstrap]);

  useEffect(() => () => {
    pendingSaves.current.forEach(timeout => clearTimeout(timeout));
    pendingSaves.current.clear();
  }, []);

  useEffect(() => {
    if (!isAuthenticated) {
      pendingSaves.current.forEach(timeout => clearTimeout(timeout));
      pendingSaves.current.clear();
    }
  }, [isAuthenticated]);

  const createWorkspace = useCallback(async (name: string, description?: string) => {
    const response = await api.post<{ workspace: WorkspaceSummary }>(
      '/api/v1/workspaces',
      { name, description }
    );
    const workspace = response.data.workspace;
    await refreshWorkspaces();

    if (workspace.projects.length) {
      await loadProject(workspace.id, workspace.projects[0].id, workspace.name);
    } else {
      setState(prev => ({
        ...prev,
        workspaceId: workspace.id,
        workspaceName: workspace.name,
        projectId: null,
        projectName: null,
        files: new Map(),
        openFiles: [],
        activeFile: null,
        mainFile: null,
      }));
    }

    return workspace;
  }, [refreshWorkspaces, loadProject]);

  const createProject = useCallback(async (name?: string) => {
    let workspaceId = state.workspaceId;
    if (!workspaceId) {
      const newWorkspace = await createWorkspace('New Workspace');
      workspaceId = newWorkspace.id;
    }

    if (!workspaceId) {
      return;
    }

    const response = await api.post<ProjectResponse>(
      `/api/v1/workspaces/${workspaceId}/projects`,
      {
        name: name?.trim() || undefined,
      }
    );

    await refreshWorkspaces();
    const workspaceName = state.workspaces.find(w => w.id === workspaceId)?.name;
    hydrateProject(workspaceId, response.data.project, workspaceName);
  }, [state.workspaceId, state.workspaces, createWorkspace, refreshWorkspaces, hydrateProject]);

  const addFile = useCallback(async (path: string, content = '') => {
    const workspaceId = state.workspaceId;
    const projectId = state.projectId;
    if (!workspaceId || !projectId) return;

    const normalizedPath = path.trim();
    if (!normalizedPath) return;

    const fileNode: FileNode = {
      name: normalizedPath.split('/').pop() || normalizedPath,
      path: normalizedPath,
      content,
      isModified: true,
    };

    setState(prev => ({
      ...prev,
      files: new Map(prev.files).set(normalizedPath, fileNode),
      openFiles: prev.openFiles.includes(normalizedPath)
        ? prev.openFiles
        : [...prev.openFiles, normalizedPath],
      activeFile: normalizedPath,
    }));
    setVersion(v => v + 1);

    try {
      await api.post<FileResponse>(
        `/api/v1/workspaces/${workspaceId}/projects/${projectId}/files`,
        { path: normalizedPath, content }
      );
    } catch (error) {
      console.error('Failed to add file', error);
    }
  }, [state.workspaceId, state.projectId]);

  const persistFile = useCallback(async (path: string, content: string) => {
    const workspaceId = state.workspaceId;
    const projectId = state.projectId;
    if (!workspaceId || !projectId) return;

    try {
      await api.put<FileResponse>(
        `/api/v1/workspaces/${workspaceId}/projects/${projectId}/files`,
        { path, content }
      );
    } catch (error) {
      console.error('Failed to save file', error);
    }
  }, [state.workspaceId, state.projectId]);

  const updateFile = useCallback((path: string, content: string) => {
    setState(prev => {
      const files = new Map(prev.files);
      const file = files.get(path);
      if (file) {
        files.set(path, { ...file, content, isModified: true });
      }
      return { ...prev, files };
    });
    setVersion(v => v + 1);

    const timers = pendingSaves.current;
    const existing = timers.get(path);
    if (existing) {
      clearTimeout(existing);
    }
    const timeout = setTimeout(() => {
      persistFile(path, content);
      timers.delete(path);
    }, 400);
    timers.set(path, timeout);
  }, [persistFile]);

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
      const activeFile = prev.activeFile === path ? (openFiles[0] ?? null) : prev.activeFile;
      return { ...prev, openFiles, activeFile };
    });
  }, []);

  const setActiveFile = useCallback((path: string | null) => {
    setState(prev => ({ ...prev, activeFile: path }));
  }, []);

  const setMainFile = useCallback(async (path: string) => {
    if (!state.workspaceId || !state.projectId) return;
    setState(prev => ({ ...prev, mainFile: path }));
    setVersion(v => v + 1);
    try {
      await api.post<ProjectResponse>(
        `/api/v1/workspaces/${state.workspaceId}/projects/${state.projectId}/main-file`,
        { path }
      );
      await refreshWorkspaces();
    } catch (error) {
      console.error('Failed to update main file', error);
    }
  }, [state.workspaceId, state.projectId, refreshWorkspaces]);

  const selectWorkspace = useCallback(async (workspaceId: string) => {
    const workspace = state.workspaces.find(w => w.id === workspaceId);
    if (workspace && workspace.projects.length) {
      await loadProject(workspaceId, workspace.projects[0].id, workspace.name);
    } else {
      setState(prev => ({
        ...prev,
        workspaceId,
        workspaceName: workspace?.name ?? prev.workspaceName,
        projectId: null,
        projectName: null,
        files: new Map(),
        openFiles: [],
        activeFile: null,
        mainFile: null,
      }));
    }
  }, [state.workspaces, loadProject]);

  const selectProject = useCallback(async (projectId: string, workspaceId?: string) => {
    const targetWorkspaceId = workspaceId || state.workspaceId;
    if (!targetWorkspaceId) return;
    const workspaceName = state.workspaces.find(w => w.id === targetWorkspaceId)?.name;
    await loadProject(targetWorkspaceId, projectId, workspaceName);
  }, [state.workspaceId, state.workspaces, loadProject]);

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

    const apiFiles: { [key: string]: string } = {};
    files.forEach((file, path) => {
      apiFiles[path] = file.content;
    });

    return {
      files: apiFiles,
      mainFile,
    };
  }, [state.files, state.mainFile]);

  const isReady = Boolean(state.projectId) && !state.loading;

  return {
    ...state,
    version,
    isReady,
    workspaces: state.workspaces,
    createWorkspace,
    createProject,
    refreshWorkspaces,
    addFile,
    updateFile,
    openFile,
    closeFile,
    setActiveFile,
    setMainFile,
    selectWorkspace,
    selectProject,
    getActiveFileContent,
    getCompilationData,
  };
};
