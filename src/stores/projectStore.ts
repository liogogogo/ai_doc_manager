import { create } from "zustand";

export interface Project {
  id: string;
  name: string;
  rootPath: string;
  healthScore: number;
  docCount: number;
  conflictCount: number;
  staleCount: number;
  lastGcAt: string | null;
}

interface ProjectState {
  projects: Project[];
  currentProject: Project | null;
  setCurrentProject: (project: Project) => void;
  setProjects: (projects: Project[]) => void;
  addProject: (project: Project) => void;
  removeProject: (id: string) => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  projects: [],
  currentProject: null,

  setCurrentProject: (project) => set({ currentProject: project }),

  setProjects: (projects) =>
    set({ projects, currentProject: projects[0] ?? null }),

  addProject: (project) =>
    set((state) => ({ projects: [...state.projects, project] })),

  removeProject: (id) =>
    set((state) => ({
      projects: state.projects.filter((p) => p.id !== id),
      currentProject:
        state.currentProject?.id === id ? null : state.currentProject,
    })),
}));
