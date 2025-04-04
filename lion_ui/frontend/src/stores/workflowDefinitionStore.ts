import { create } from 'zustand';
import { 
  loadWorkflowDefinition, 
  saveWorkflowDefinition, 
  createWorkflowDefinition,
  listWorkflowDefinitions
} from '../lib/api';

export interface WorkflowNode {
  id: string;
  position: { x: number; y: number };
  type?: string;
  data: {
    label: string;
    [key: string]: any;
  };
}

export interface WorkflowEdge {
  id: string;
  source: string;
  target: string;
  type?: string;
  label?: string;
  animated?: boolean;
}

export interface WorkflowDefinition {
  id: string;
  name: string;
  description: string;
  version: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  created_at: string;
  updated_at: string;
  file_path?: string;
}

interface WorkflowDefinitionState {
  definitions: Record<string, WorkflowDefinition>;
  selectedDefinitionId: string | null;
  isLoading: boolean;
  error: string | null;
  
  // Actions
  fetchDefinitions: () => Promise<void>;
  loadDefinition: (path: string) => Promise<string>;
  saveDefinition: (id: string) => Promise<void>;
  createDefinition: (name: string, description: string) => Promise<string>;
  updateDefinition: (id: string, updates: Partial<WorkflowDefinition>) => void;
  selectDefinition: (id: string) => void;
}

export const useWorkflowDefinitionStore = create<WorkflowDefinitionState>((set, get) => ({
  definitions: {},
  selectedDefinitionId: null,
  isLoading: false,
  error: null,
  
  fetchDefinitions: async () => {
    try {
      set({ isLoading: true, error: null });
      
      const definitions = await listWorkflowDefinitions();
      
      // Convert array to record for easier lookup
      const definitionsRecord: Record<string, WorkflowDefinition> = {};
      definitions.forEach(def => {
        definitionsRecord[def.id] = def;
      });
      
      set({ 
        definitions: definitionsRecord,
        isLoading: false 
      });
    } catch (error) {
      console.error('Failed to fetch workflow definitions:', error);
      set({ 
        error: `Failed to fetch workflow definitions: ${error}`,
        isLoading: false 
      });
    }
  },
  
  loadDefinition: async (path: string) => {
    try {
      set({ isLoading: true, error: null });
      
      const definition = await loadWorkflowDefinition(path);
      
      set((state) => ({
        definitions: {
          ...state.definitions,
          [definition.id]: definition
        },
        selectedDefinitionId: definition.id,
        isLoading: false
      }));
      
      return definition.id;
    } catch (error) {
      console.error('Failed to load workflow definition:', error);
      set({ 
        error: `Failed to load workflow definition: ${error}`,
        isLoading: false 
      });
      throw error;
    }
  },
  
  saveDefinition: async (id: string) => {
    try {
      set({ isLoading: true, error: null });
      
      const { definitions } = get();
      const definition = definitions[id];
      
      if (!definition) {
        throw new Error(`Workflow definition with ID ${id} not found`);
      }
      
      await saveWorkflowDefinition(definition);
      
      set({ isLoading: false });
    } catch (error) {
      console.error('Failed to save workflow definition:', error);
      set({ 
        error: `Failed to save workflow definition: ${error}`,
        isLoading: false 
      });
      throw error;
    }
  },
  
  createDefinition: async (name: string, description: string) => {
    try {
      set({ isLoading: true, error: null });
      
      const newDefinition = await createWorkflowDefinition(name, description);
      
      set((state) => ({
        definitions: {
          ...state.definitions,
          [newDefinition.id]: newDefinition
        },
        selectedDefinitionId: newDefinition.id,
        isLoading: false
      }));
      
      return newDefinition.id;
    } catch (error) {
      console.error('Failed to create workflow definition:', error);
      set({ 
        error: `Failed to create workflow definition: ${error}`,
        isLoading: false 
      });
      throw error;
    }
  },
  
  updateDefinition: (id: string, updates: Partial<WorkflowDefinition>) => {
    set((state) => {
      const definition = state.definitions[id];
      
      if (!definition) {
        return {
          ...state,
          error: `Workflow definition with ID ${id} not found`
        };
      }
      
      return {
        ...state,
        definitions: {
          ...state.definitions,
          [id]: {
            ...definition,
            ...updates,
            updated_at: new Date().toISOString()
          }
        }
      };
    });
  },
  
  selectDefinition: (id: string) => {
    set((state) => ({ ...state, selectedDefinitionId: id }));
  }
}));