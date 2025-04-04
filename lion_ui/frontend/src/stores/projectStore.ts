import { create } from 'zustand';
import { Project } from '../types';
import { identifyProject, openProject } from '../lib/api';
import { open } from '@tauri-apps/api/dialog';

/**
 * Project store state interface
 */
interface ProjectStore {
  // Data
  project: Project;
  recentProjects: string[];
  
  // UI state
  isLoading: boolean;
  error: string | null;
  
  // Actions
  selectAndOpenProject: () => Promise<void>;
  openProjectFromPath: (path: string) => Promise<void>;
  resetError: () => void;
}

/**
 * Default project
 */
const DEFAULT_PROJECT: Project = {
  name: '',
  root_path: '',
  folders: [],
  files: [],
  is_loaded: false,
};

/**
 * Project store
 */
export const useProjectStore = create<ProjectStore>((set) => ({
  // Initial state
  project: DEFAULT_PROJECT,
  recentProjects: [],
  isLoading: false,
  error: null,

  // Select and open a project using the file dialog
  selectAndOpenProject: async () => {
    try {
      // Open file dialog to select project directory
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Project Directory',
      });

      if (!selected) {
        return; // User cancelled
      }

      const path = Array.isArray(selected) ? selected[0] : selected;
      
      // Set loading state
      await set((state) => ({ ...state, isLoading: true, error: null }));
      
      // First identify the project
      const projectInfo = await identifyProject(path);
      
      // Then open it
      const project = await openProject(path);
      
      // Update recent projects
      const recentProjects = [path];
      
      // Update state
      set({ 
        project, 
        recentProjects,
        isLoading: false 
      });
    } catch (error) {
      console.error('Failed to open project:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to open project', 
        isLoading: false 
      });
    }
  },

  // Open a project from a specific path
  openProjectFromPath: async (path) => {
    try {
      // Set loading state
      await set((state) => ({ ...state, isLoading: true, error: null }));
      
      // First identify the project
      const projectInfo = await identifyProject(path);
      
      // Then open it
      const project = await openProject(path);
      
      // Update recent projects
      const recentProjects = [path];
      
      // Update state
      set({ 
        project, 
        recentProjects,
        isLoading: false 
      });
    } catch (error) {
      console.error('Failed to open project:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to open project', 
        isLoading: false 
      });
    }
  },

  // Reset any error
  resetError: () => {
    set({ error: null });
  },
}));